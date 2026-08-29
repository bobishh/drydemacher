use ecky_fem::{
    solve_linear_static, FemConstraint, FemFaceTarget, FemForceVector, FemLoad, FemMaterial,
    FemMeshingEvidence, FemOptionalDisplacement, FemPoint3, FemRuntimeIdentity, FemVolumeMesh,
    FemVolumeMeshInput, FEM_SCHEMA_VERSION,
};

#[test]
fn axial_bar_matches_closed_form_stress_displacement_poisson_contraction_and_reaction() {
    let length_mm = 100.0;
    let width_mm = 10.0;
    let force_n = 1_000.0;
    let young_modulus_mpa = 200_000.0;
    let poisson_ratio = 0.3;
    let mesh = bar_mesh(length_mm, width_mm);
    let material = FemMaterial {
        schema_version: FEM_SCHEMA_VERSION,
        name: "steel-reference".to_string(),
        young_modulus_mpa,
        poisson_ratio,
        density_kg_per_mm3: 7.85e-6,
        yield_strength_mpa: 250.0,
    };
    let solved = solve_linear_static(
        &mesh,
        &material,
        &[FemLoad::SurfaceForce {
            schema_version: FEM_SCHEMA_VERSION,
            name: "axial-force".to_string(),
            faces: vec![mesh.face_group_targets[3].clone()],
            total_force_n: FemForceVector {
                x_n: force_n,
                y_n: 0.0,
                z_n: 0.0,
            },
        }],
        &[
            component_support(
                "x-symmetry",
                mesh.face_group_targets[0].clone(),
                Some(0.0),
                None,
                None,
            ),
            component_support(
                "y-symmetry",
                mesh.face_group_targets[1].clone(),
                None,
                Some(0.0),
                None,
            ),
            component_support(
                "z-symmetry",
                mesh.face_group_targets[2].clone(),
                None,
                None,
                Some(0.0),
            ),
        ],
        1.0e-10,
        24,
    )
    .expect("axial reference solve");

    let expected_stress_mpa = force_n / (width_mm * width_mm);
    let expected_end_displacement_mm =
        force_n * length_mm / (young_modulus_mpa * width_mm * width_mm);
    for element in &solved.postprocess.elements {
        assert!((element.von_mises_mpa - expected_stress_mpa).abs() <= 1.0e-8);
    }
    for (node, point) in mesh.nodes.iter().enumerate() {
        let axial = solved.displacement_dofs_mm[node * 3];
        let transverse_y = solved.displacement_dofs_mm[node * 3 + 1];
        let transverse_z = solved.displacement_dofs_mm[node * 3 + 2];
        assert!((axial - expected_end_displacement_mm * point.x_mm / length_mm).abs() <= 1.0e-10);
        assert!(
            (transverse_y + poisson_ratio * expected_end_displacement_mm * point.y_mm / length_mm)
                .abs()
                <= 1.0e-10
        );
        assert!(
            (transverse_z + poisson_ratio * expected_end_displacement_mm * point.z_mm / length_mm)
                .abs()
                <= 1.0e-10
        );
    }
    assert!(solved.equilibrium.relative_imbalance <= 1.0e-10);
    let reaction_x = solved
        .support_reactions
        .iter()
        .map(|reaction| reaction.resultant_n[0])
        .sum::<f64>();
    assert!((reaction_x + force_n).abs() <= 1.0e-8);
}

fn component_support(
    name: &str,
    face: FemFaceTarget,
    x_mm: Option<f64>,
    y_mm: Option<f64>,
    z_mm: Option<f64>,
) -> FemConstraint {
    FemConstraint::PrescribedDisplacement {
        schema_version: FEM_SCHEMA_VERSION,
        name: name.to_string(),
        faces: vec![face],
        displacement_mm: FemOptionalDisplacement { x_mm, y_mm, z_mm },
    }
}

fn bar_mesh(length: f64, width: f64) -> FemVolumeMesh {
    let nodes = vec![
        FemPoint3::new(0.0, 0.0, 0.0),
        FemPoint3::new(length, 0.0, 0.0),
        FemPoint3::new(length, width, 0.0),
        FemPoint3::new(0.0, width, 0.0),
        FemPoint3::new(0.0, 0.0, width),
        FemPoint3::new(length, 0.0, width),
        FemPoint3::new(length, width, width),
        FemPoint3::new(0.0, width, width),
    ];
    let triangles = vec![
        [0, 7, 3],
        [0, 4, 7], // x=0
        [0, 1, 5],
        [0, 5, 4], // y=0
        [0, 2, 1],
        [0, 3, 2], // z=0
        [1, 2, 6],
        [1, 6, 5], // x=L
        [3, 6, 2],
        [3, 7, 6], // y=W
        [4, 5, 6],
        [4, 6, 7], // z=W
    ];
    FemVolumeMesh::validate_and_canonicalize(FemVolumeMeshInput {
        schema_version: FEM_SCHEMA_VERSION,
        nodes,
        cells: vec![
            [0, 1, 2, 6],
            [0, 2, 3, 6],
            [0, 3, 7, 6],
            [0, 7, 4, 6],
            [0, 4, 5, 6],
            [0, 5, 1, 6],
        ],
        boundary_triangles: triangles,
        boundary_face_group_indices: vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5],
        face_group_count: 6,
        face_group_targets: (0..6).map(face_target).collect(),
        source_boundary_digest: "sha256:axial-boundary".to_string(),
        mesher_identity: FemRuntimeIdentity {
            schema_version: FEM_SCHEMA_VERSION,
            platform: "reference".to_string(),
            architecture: "reference".to_string(),
            library_name: "analytic-fixture".to_string(),
            library_version: "1".to_string(),
            library_digest: "sha256:analytic-mesh".to_string(),
            adapter_protocol_version: 1,
            supported_capabilities: vec!["tet4".to_string()],
            notice_digest: "sha256:notice".to_string(),
        },
        meshing_evidence: FemMeshingEvidence {
            schema_version: FEM_SCHEMA_VERSION,
            source_triangle_count: 12,
            inserted_source_triangle_count: 12,
            tagged_boundary_triangle_count: 12,
            maximum_boundary_deviation_mm: 0.0,
            discarded_tet4_component_count: 0,
            discarded_tet4_cell_count: 0,
            discarded_low_quality_tet4_cell_count: 0,
            deterministic_thread_count: 1,
        },
        minimum_scaled_jacobian: 1.0e-8,
    })
    .expect("valid axial bar mesh")
}

fn face_target(index: usize) -> FemFaceTarget {
    FemFaceTarget {
        schema_version: FEM_SCHEMA_VERSION,
        part_id: "bar".to_string(),
        canonical_target_id: format!("bar:face:{index}"),
        durable_target_id: format!("bar:stable:{index}"),
        source_geometry_digest: "sha256:axial-geometry".to_string(),
    }
}
