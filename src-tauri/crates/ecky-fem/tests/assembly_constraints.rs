use ecky_fem::{
    ElementAssembler, FemDirichletConstraint, FemIndexedTet4Mesh, FemMaterial, FemPoint3,
    FemSparseMatrix, FEM_SCHEMA_VERSION,
};

fn material() -> FemMaterial {
    FemMaterial {
        schema_version: FEM_SCHEMA_VERSION,
        name: "aluminum".to_string(),
        young_modulus_mpa: 68_900.0,
        poisson_ratio: 0.33,
        density_kg_per_mm3: 0.000_002_7,
        yield_strength_mpa: 276.0,
    }
}

#[test]
fn global_tet4_stiffness_assembles_sparse_symmetric_shared_nodes() {
    let mesh = FemIndexedTet4Mesh {
        schema_version: FEM_SCHEMA_VERSION,
        nodes: vec![
            FemPoint3::new(0.0, 0.0, 0.0),
            FemPoint3::new(1.0, 0.0, 0.0),
            FemPoint3::new(0.0, 1.0, 0.0),
            FemPoint3::new(0.0, 0.0, 1.0),
            FemPoint3::new(1.0, 1.0, 1.0),
        ],
        cells: vec![[0, 1, 2, 3], [1, 2, 3, 4]],
    };

    let matrix = ElementAssembler
        .assemble_global_stiffness(&mesh, &material())
        .expect("global assembly");

    assert_eq!(matrix.dimension, 15);
    assert!(!matrix.entries.is_empty());
    for entry in &matrix.entries {
        let transpose = matrix
            .entries
            .iter()
            .find(|candidate| candidate.row == entry.col && candidate.col == entry.row)
            .expect("symmetric transpose entry");
        assert!((entry.value - transpose.value).abs() <= 1.0e-9);
    }
}

#[test]
fn dirichlet_elimination_applies_nonzero_values_without_penalty_stiffness() {
    let matrix = FemSparseMatrix::from_dense(vec![
        vec![4.0, -1.0, 0.0],
        vec![-1.0, 3.0, -1.0],
        vec![0.0, -1.0, 2.0],
    ])
    .expect("sparse matrix");
    let reduction = matrix
        .eliminate_dirichlet(
            &[1.0, 2.0, 3.0],
            &[
                FemDirichletConstraint {
                    dof_index: 0,
                    value_mm: 0.5,
                },
                FemDirichletConstraint {
                    dof_index: 2,
                    value_mm: -1.0,
                },
            ],
        )
        .expect("Dirichlet reduction");

    assert_eq!(reduction.free_dof_indices, vec![1]);
    assert_eq!(reduction.matrix.to_dense(), vec![vec![3.0]]);
    assert!((reduction.rhs[0] - 1.5).abs() <= 1.0e-12);
    assert_eq!(reduction.constrained_dofs[0].value_mm, 0.5);
    assert_eq!(reduction.constrained_dofs[1].value_mm, -1.0);
    let full = reduction
        .recover_full_solution(&[0.5])
        .expect("full solution");
    assert_eq!(full, vec![0.5, 0.5, -1.0]);
    let reactions = reduction
        .recover_support_reactions(&[0.5])
        .expect("support reactions");
    assert!((reactions[0].reaction_n - 0.5).abs() <= 1.0e-12);
    assert!((reactions[1].reaction_n + 5.5).abs() <= 1.0e-12);
}
