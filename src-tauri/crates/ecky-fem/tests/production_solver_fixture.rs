use std::{collections::BTreeSet, fmt::Write as _, path::Path};

use ecky_fem::{
    ElementAssembler, FemDirichletConstraint, FemIndexedTet4Mesh, FemMaterial, FemPoint3,
    FEM_SCHEMA_VERSION,
};

fn f64_values(path: &Path) -> Vec<f64> {
    std::fs::read(path)
        .expect("read float64 array")
        .chunks_exact(8)
        .map(|bytes| f64::from_le_bytes(bytes.try_into().expect("f64 bytes")))
        .collect()
}

fn u32_values(path: &Path) -> Vec<u32> {
    std::fs::read(path)
        .expect("read uint32 array")
        .chunks_exact(4)
        .map(|bytes| u32::from_le_bytes(bytes.try_into().expect("u32 bytes")))
        .collect()
}

#[test]
#[ignore = "explicit replay of immutable production FEM cache"]
fn exports_production_scale_spd_and_multiple_rhs() {
    let result_dir = std::env::var_os("ECKY_FEM_RESULT_DIR")
        .map(std::path::PathBuf::from)
        .expect("ECKY_FEM_RESULT_DIR must name an immutable FEM result");
    let output_dir = std::env::var_os("ECKY_SOLVER_BENCH_DIR")
        .map(std::path::PathBuf::from)
        .expect("ECKY_SOLVER_BENCH_DIR must name explicit scratch");
    let support_groups = std::env::var("ECKY_SUPPORT_FACE_GROUPS")
        .expect("ECKY_SUPPORT_FACE_GROUPS must be comma-separated indices")
        .split(',')
        .map(|value| value.parse::<u32>().expect("support group index"))
        .collect::<BTreeSet<_>>();
    let arrays = result_dir.join("arrays");
    let coordinates = f64_values(&arrays.join("nodes.f64le"));
    let nodes = coordinates
        .chunks_exact(3)
        .map(|xyz| FemPoint3::new(xyz[0], xyz[1], xyz[2]))
        .collect::<Vec<_>>();
    let cells = u32_values(&arrays.join("tet4-cells.u32le"))
        .chunks_exact(4)
        .map(|cell| [cell[0], cell[1], cell[2], cell[3]])
        .collect::<Vec<_>>();
    assert_eq!(cells.len(), 50_287, "authoritative production mesh");
    let boundary = u32_values(&arrays.join("boundary-triangles.u32le"));
    let groups = u32_values(&arrays.join("boundary-face-groups.u32le"));
    let mut support_nodes = BTreeSet::new();
    for (triangle, group) in boundary.chunks_exact(3).zip(groups) {
        if support_groups.contains(&group) {
            support_nodes.extend(triangle.iter().map(|node| *node as usize));
        }
    }
    assert!(!support_nodes.is_empty());
    let constraints = support_nodes
        .iter()
        .flat_map(|node| {
            (0..3).map(move |axis| FemDirichletConstraint {
                dof_index: node * 3 + axis,
                value_mm: 0.0,
            })
        })
        .collect::<Vec<_>>();
    let mesh = FemIndexedTet4Mesh {
        schema_version: FEM_SCHEMA_VERSION,
        nodes,
        cells,
    };
    let material = FemMaterial {
        schema_version: FEM_SCHEMA_VERSION,
        name: "PETG-CF screening proxy".into(),
        young_modulus_mpa: 4_000.0,
        poisson_ratio: 0.35,
        density_kg_per_mm3: 1.25e-6,
        yield_strength_mpa: 45.0,
    };
    let stiffness = ElementAssembler
        .assemble_global_stiffness(&mesh, &material)
        .expect("assemble cached production mesh");
    let maximum_y = mesh
        .nodes
        .iter()
        .map(|node| node.y_mm)
        .fold(f64::NEG_INFINITY, f64::max);
    let load_nodes = mesh
        .nodes
        .iter()
        .enumerate()
        .filter(|(index, node)| !support_nodes.contains(index) && node.y_mm >= maximum_y - 1.0)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    assert!(!load_nodes.is_empty());
    let directions = [
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, -1.0],
    ];
    let reductions = directions
        .iter()
        .map(|direction| {
            let mut rhs = vec![0.0; mesh.nodes.len() * 3];
            for node in &load_nodes {
                for axis in 0..3 {
                    rhs[node * 3 + axis] = direction[axis] / load_nodes.len() as f64;
                }
            }
            stiffness
                .eliminate_dirichlet(&rhs, &constraints)
                .expect("reduce system")
        })
        .collect::<Vec<_>>();
    let matrix = &reductions[0].matrix;
    assert!(matrix.dimension > 40_000);
    assert!(reductions.iter().all(|item| item.matrix == *matrix));
    use faer::prelude::Solve as _;
    use faer::sparse::{SparseColMat, Triplet};
    let triplets = matrix
        .entries
        .iter()
        .filter(|entry| entry.row <= entry.col)
        .map(|entry| Triplet::new(entry.row, entry.col, entry.value))
        .collect::<Vec<_>>();
    let sparse = SparseColMat::<usize, f64>::try_new_from_triplets(
        matrix.dimension,
        matrix.dimension,
        &triplets,
    )
    .expect("Faer production benchmark matrix");
    let rhs_matrix = faer::Mat::from_fn(matrix.dimension, reductions.len(), |row, column| {
        reductions[column].rhs[row]
    });
    let previous_parallelism = faer::get_global_parallelism();
    for (label, parallelism) in [
        ("faer-sequential", faer::Par::Seq),
        ("faer-rayon-8", faer::Par::rayon(8)),
    ] {
        faer::set_global_parallelism(parallelism);
        let started = std::time::Instant::now();
        let factor = sparse.sp_cholesky(faer::Side::Upper).expect("Faer factor");
        let factor_ms = started.elapsed().as_secs_f64() * 1_000.0;
        let started = std::time::Instant::now();
        let solved = factor.solve(&rhs_matrix);
        let solve_ms = started.elapsed().as_secs_f64() * 1_000.0;
        let mut maximum_relative_residual = 0.0_f64;
        for (column, reduction) in reductions.iter().enumerate() {
            let mut residual = reduction
                .rhs
                .iter()
                .map(|value| -*value)
                .collect::<Vec<_>>();
            for entry in &matrix.entries {
                residual[entry.row] += entry.value * solved[(entry.col, column)];
            }
            let residual_l2 = residual
                .iter()
                .map(|value| value * value)
                .sum::<f64>()
                .sqrt();
            let rhs_l2 = reduction
                .rhs
                .iter()
                .map(|value| value * value)
                .sum::<f64>()
                .sqrt();
            maximum_relative_residual =
                maximum_relative_residual.max(residual_l2 / rhs_l2.max(1.0));
        }
        eprintln!("{{:backend \"{label}\" :dimension {} :nnz {} :rhs-count {} :factor-ms {factor_ms} :solve-ms {solve_ms} :maximum-relative-residual {maximum_relative_residual}}}", matrix.dimension, matrix.entries.len(), reductions.len());
        assert!(maximum_relative_residual <= 1.0e-8);
    }
    faer::set_global_parallelism(previous_parallelism);
    std::fs::create_dir_all(&output_dir).expect("create scratch");
    let mut matrix_market = String::from("%%MatrixMarket matrix coordinate real general\n");
    writeln!(
        matrix_market,
        "{} {} {}",
        matrix.dimension,
        matrix.dimension,
        matrix.entries.len()
    )
    .unwrap();
    for entry in &matrix.entries {
        writeln!(
            matrix_market,
            "{} {} {:.17e}",
            entry.row + 1,
            entry.col + 1,
            entry.value
        )
        .unwrap();
    }
    std::fs::write(output_dir.join("cage-50k-stiffness.mtx"), matrix_market).unwrap();
    let mut rhs_market = String::from("%%MatrixMarket matrix array real general\n");
    writeln!(rhs_market, "{} {}", matrix.dimension, reductions.len()).unwrap();
    for reduction in &reductions {
        for value in &reduction.rhs {
            writeln!(rhs_market, "{value:.17e}").unwrap();
        }
    }
    std::fs::write(output_dir.join("cage-50k-rhs.mtx"), rhs_market).unwrap();
    eprintln!(
        "production solver fixture: cells={}, dimension={}, nnz={}, rhs={}, supports={}",
        mesh.cells.len(),
        matrix.dimension,
        matrix.entries.len(),
        reductions.len(),
        support_nodes.len()
    );
}
