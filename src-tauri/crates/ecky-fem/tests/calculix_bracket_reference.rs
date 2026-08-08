use ecky_fem::{
    solve_linear_static, FemConstraint, FemFaceTarget, FemForceVector, FemLoad, FemMaterial,
    FemMeshingEvidence, FemPoint3, FemRuntimeIdentity, FemVolumeMesh, FemVolumeMeshInput,
    FEM_SCHEMA_VERSION,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

const INPUT_NODES: [[f64; 3]; 8] = [
    [0.0, 0.0, 0.0],
    [10.0, 0.0, 0.0],
    [10.0, 10.0, 0.0],
    [0.0, 10.0, 0.0],
    [0.0, 0.0, 10.0],
    [10.0, 0.0, 10.0],
    [10.0, 10.0, 10.0],
    [0.0, 10.0, 10.0],
];
const INPUT_CELLS: [[u32; 4]; 6] = [
    [0, 1, 2, 6],
    [0, 2, 3, 6],
    [0, 3, 7, 6],
    [0, 7, 4, 6],
    [0, 4, 5, 6],
    [0, 5, 1, 6],
];

#[test]
fn ecky_matches_versioned_offline_calculix_bracket_displacement_stress_reaction_and_locations() {
    let fixture_root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/fem");
    let input_bytes = std::fs::read(fixture_root.join("calculix-bracket-v1.inp"))
        .expect("checked-in CalculiX input");
    let golden: Value = serde_json::from_slice(
        &std::fs::read(fixture_root.join("calculix-bracket-v1.golden.json"))
            .expect("checked-in CalculiX golden"),
    )
    .expect("versioned CalculiX golden JSON");
    let input_sha = format!("{:x}", Sha256::digest(input_bytes));
    assert_eq!(golden["schemaVersion"], 1);
    assert_eq!(golden["inputSha256"], input_sha);
    assert_eq!(golden["generator"]["program"], "CalculiX CrunchiX");
    assert_eq!(golden["generator"]["runtimeRequiredByTests"], false);

    let mesh = bracket_mesh();
    let solved = solve_linear_static(
        &mesh,
        &FemMaterial {
            schema_version: FEM_SCHEMA_VERSION,
            name: "aluminum-6061".into(),
            young_modulus_mpa: golden_number(&golden["model"]["youngModulusMpa"]),
            poisson_ratio: golden_number(&golden["model"]["poissonRatio"]),
            density_kg_per_mm3: 2.7e-6,
            yield_strength_mpa: 276.0,
        },
        &[FemLoad::SurfaceForce {
            schema_version: FEM_SCHEMA_VERSION,
            name: "loaded-face".into(),
            faces: vec![mesh.face_group_targets[3].clone()],
            total_force_n: FemForceVector {
                x_n: 0.0,
                y_n: 0.0,
                z_n: -100.0,
            },
        }],
        &[FemConstraint::Fixed {
            schema_version: FEM_SCHEMA_VERSION,
            name: "fixed-face".into(),
            faces: vec![mesh.face_group_targets[0].clone()],
        }],
        1.0e-10,
        24,
    )
    .expect("Ecky differential bracket solve");

    let displacement_tolerance =
        golden_number(&golden["comparisonTolerances"]["displacementRelative"]);
    for reference in golden["loadedNodeDisplacementsMm"]
        .as_array()
        .expect("loaded displacement rows")
    {
        let input_node_index = reference["nodeId"].as_u64().expect("nodeId") as usize - 1;
        let coordinate = INPUT_NODES[input_node_index];
        let canonical_index = mesh
            .nodes
            .iter()
            .position(|point| point_array(*point) == coordinate)
            .expect("golden coordinate in canonical mesh");
        let actual = &solved.displacement_dofs_mm[canonical_index * 3..canonical_index * 3 + 3];
        let expected = json_vec3(&reference["value"]);
        assert_relative_vector(actual, &expected, displacement_tolerance, "displacement");
    }

    let stress_tolerance = golden_number(&golden["comparisonTolerances"]["stressRelative"]);
    for reference in golden["elementStressMpa"]
        .as_array()
        .expect("element stress rows")
    {
        let input_element_index = reference["elementId"].as_u64().expect("elementId") as usize - 1;
        let expected_centroid = cell_centroid(INPUT_CELLS[input_element_index]);
        let actual = solved
            .postprocess
            .elements
            .iter()
            .find(|element| {
                distance(point_array(element.centroid_mm), expected_centroid) <= 1.0e-12
            })
            .expect("golden element centroid in canonical result");
        let calculix = json_vec6(&reference["voigt"]);
        let expected_ecky_order = [
            calculix[0],
            calculix[1],
            calculix[2],
            calculix[5],
            calculix[4],
            calculix[3],
        ];
        assert_relative_vector(
            &actual.stress_mpa,
            &expected_ecky_order,
            stress_tolerance,
            "stress",
        );
    }

    let summary = &golden["summary"];
    assert_relative(
        solved.postprocess.summary.maximum_displacement.value,
        golden_number(&summary["maximumDisplacementMm"]),
        displacement_tolerance,
        "maximum displacement",
    );
    assert_relative(
        solved.postprocess.summary.maximum_von_mises.value,
        golden_number(&summary["maximumVonMisesMpa"]),
        stress_tolerance,
        "maximum von Mises",
    );
    let reaction_tolerance = golden_number(&golden["comparisonTolerances"]["reactionAbsoluteN"]);
    let actual_reaction = solved
        .support_reactions
        .iter()
        .fold([0.0; 3], |mut total, reaction| {
            for (component, value) in reaction.resultant_n.iter().enumerate() {
                total[component] += value;
            }
            total
        });
    let expected_reaction = json_vec3(&summary["reactionResultantN"]);
    for component in 0..3 {
        assert!(
            (actual_reaction[component] - expected_reaction[component]).abs() <= reaction_tolerance,
            "reaction component {component}: actual {}, expected {}",
            actual_reaction[component],
            expected_reaction[component]
        );
    }
    assert_eq!(
        point_array(
            solved
                .postprocess
                .summary
                .maximum_displacement
                .coordinate_mm
        ),
        [10.0, 10.0, 0.0]
    );
    assert_eq!(
        point_array(solved.postprocess.summary.maximum_von_mises.coordinate_mm),
        [5.0, 2.5, 7.5]
    );
}

fn bracket_mesh() -> FemVolumeMesh {
    FemVolumeMesh::validate_and_canonicalize(FemVolumeMeshInput {
        schema_version: FEM_SCHEMA_VERSION,
        nodes: INPUT_NODES
            .into_iter()
            .map(|point| FemPoint3::new(point[0], point[1], point[2]))
            .collect(),
        cells: INPUT_CELLS.to_vec(),
        boundary_triangles: vec![
            [0, 7, 3],
            [0, 4, 7],
            [0, 1, 5],
            [0, 5, 4],
            [0, 2, 1],
            [0, 3, 2],
            [1, 2, 6],
            [1, 6, 5],
            [3, 6, 2],
            [3, 7, 6],
            [4, 5, 6],
            [4, 6, 7],
        ],
        boundary_face_group_indices: vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5],
        face_group_count: 6,
        face_group_targets: (0..6).map(face_target).collect(),
        source_boundary_digest: "sha256:calculix-bracket-boundary-v1".into(),
        mesher_identity: FemRuntimeIdentity {
            schema_version: FEM_SCHEMA_VERSION,
            platform: "offline-reference".into(),
            architecture: "neutral".into(),
            library_name: "checked-in-c3d4-mesh".into(),
            library_version: "1".into(),
            library_digest: "sha256:calculix-bracket-mesh-v1".into(),
            adapter_protocol_version: 1,
            supported_capabilities: vec!["tet4".into()],
            notice_digest: "sha256:offline-reference-provenance".into(),
        },
        meshing_evidence: FemMeshingEvidence {
            schema_version: FEM_SCHEMA_VERSION,
            source_triangle_count: 12,
            inserted_source_triangle_count: 12,
            tagged_boundary_triangle_count: 12,
            maximum_boundary_deviation_mm: 0.0,
            deterministic_thread_count: 1,
        },
        minimum_scaled_jacobian: 1.0e-8,
    })
    .expect("valid differential bracket mesh")
}

fn face_target(index: usize) -> FemFaceTarget {
    FemFaceTarget {
        schema_version: FEM_SCHEMA_VERSION,
        part_id: "bracket".into(),
        canonical_target_id: format!("bracket:face:{index}"),
        durable_target_id: format!("bracket:stable:{index}"),
        source_geometry_digest: "sha256:calculix-bracket-geometry-v1".into(),
    }
}

fn point_array(point: FemPoint3) -> [f64; 3] {
    [point.x_mm, point.y_mm, point.z_mm]
}

fn cell_centroid(cell: [u32; 4]) -> [f64; 3] {
    let mut centroid = [0.0; 3];
    for node in cell {
        for component in 0..3 {
            centroid[component] += INPUT_NODES[node as usize][component] / 4.0;
        }
    }
    centroid
}

fn distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    left.into_iter()
        .zip(right)
        .map(|(left, right)| (left - right).powi(2))
        .sum::<f64>()
        .sqrt()
}

fn golden_number(value: &Value) -> f64 {
    value.as_f64().expect("finite golden number")
}

fn json_vec3(value: &Value) -> [f64; 3] {
    let values = value.as_array().expect("golden vec3");
    [
        golden_number(&values[0]),
        golden_number(&values[1]),
        golden_number(&values[2]),
    ]
}

fn json_vec6(value: &Value) -> [f64; 6] {
    let values = value.as_array().expect("golden vec6");
    std::array::from_fn(|index| golden_number(&values[index]))
}

fn assert_relative_vector(actual: &[f64], expected: &[f64], tolerance: f64, label: &str) {
    let difference = actual
        .iter()
        .zip(expected)
        .map(|(actual, expected)| (actual - expected).powi(2))
        .sum::<f64>()
        .sqrt();
    let scale = expected
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt()
        .max(1.0e-30);
    assert!(
        difference / scale <= tolerance,
        "{label} relative difference {} exceeds declared {tolerance}; actual={actual:?}, expected={expected:?}",
        difference / scale
    );
}

fn assert_relative(actual: f64, expected: f64, tolerance: f64, label: &str) {
    let relative = (actual - expected).abs() / expected.abs().max(1.0e-30);
    assert!(
        relative <= tolerance,
        "{label} relative difference {relative} exceeds declared {tolerance}; actual={actual}, expected={expected}"
    );
}
