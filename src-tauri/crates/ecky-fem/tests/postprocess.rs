use ecky_fem::{
    postprocess_linear_static, postprocess_linear_static_with_observer, FemFaceTarget, FemMaterial,
    FemMeshingEvidence, FemPoint3, FemRuntimeIdentity, FemSafetyFactor, FemVolumeMesh,
    FemVolumeMeshInput, FEM_SCHEMA_VERSION,
};

#[test]
fn postprocess_keeps_unaveraged_verification_fields_separate_from_display_averages() {
    let mesh = one_tet_mesh();
    let material = material();
    let displacement_dofs = mesh
        .nodes
        .iter()
        .flat_map(|node| [0.001 * node.x_mm, 0.0, 0.0])
        .collect::<Vec<_>>();

    let result = postprocess_linear_static(&mesh, &material, &displacement_dofs)
        .expect("postprocess one Tet4");
    assert_eq!(result.elements.len(), 1);
    assert_eq!(result.nodal_display.len(), 4);
    assert!((result.elements[0].strain[0] - 0.001).abs() < 1.0e-12);
    assert!((result.elements[0].stress_mpa[0] - 1.2).abs() < 1.0e-12);
    assert!((result.elements[0].von_mises_mpa - 0.8).abs() < 1.0e-12);
    assert_eq!(result.elements[0].principal_stress_mpa, [1.2, 0.4, 0.4]);
    assert_eq!(result.summary.maximum_von_mises.element_id, Some(0));
    assert_eq!(result.summary.maximum_displacement.node_id, Some(3));
    assert!((result.summary.volume_mm3 - 1.0 / 6.0).abs() < 1.0e-12);
    assert!((result.summary.mass_kg - 1.0e-6 / 6.0).abs() < 1.0e-18);
    match result.summary.minimum_yield_safety_factor {
        FemSafetyFactor::Finite { value } => assert!((value - 125.0).abs() < 1.0e-10),
        FemSafetyFactor::Infinite => panic!("loaded element must have finite safety factor"),
    }
    assert!(result.result_digest.starts_with("sha256:"));

    let zero = postprocess_linear_static(&mesh, &material, &[0.0; 12])
        .expect("zero-displacement postprocess");
    assert_eq!(
        zero.summary.minimum_yield_safety_factor,
        FemSafetyFactor::Infinite
    );
}

#[test]
fn postprocess_observer_can_cancel_before_first_result_chunk() {
    let error =
        postprocess_linear_static_with_observer(&one_tet_mesh(), &material(), &[0.0; 12], |_| {
            Err(ecky_fem::FemValidationError {
                field: "cancelled".into(),
                message: "postprocess cancelled".into(),
            })
        })
        .expect_err("postprocess cancellation");
    assert_eq!(error.field, "cancelled");
}

fn one_tet_mesh() -> FemVolumeMesh {
    FemVolumeMesh::validate_and_canonicalize(FemVolumeMeshInput {
        schema_version: FEM_SCHEMA_VERSION,
        nodes: vec![
            FemPoint3::new(0.0, 0.0, 0.0),
            FemPoint3::new(1.0, 0.0, 0.0),
            FemPoint3::new(0.0, 1.0, 0.0),
            FemPoint3::new(0.0, 0.0, 1.0),
        ],
        cells: vec![[0, 1, 2, 3]],
        boundary_triangles: vec![[1, 3, 2], [0, 2, 3], [0, 3, 1], [0, 1, 2]],
        boundary_face_group_indices: vec![0, 1, 2, 3],
        face_group_count: 4,
        face_group_targets: (0..4)
            .map(|index| FemFaceTarget {
                schema_version: FEM_SCHEMA_VERSION,
                part_id: "body".to_string(),
                canonical_target_id: format!("body:face:{index}"),
                durable_target_id: format!("body:stable:{index}"),
                source_geometry_digest: "sha256:geometry".to_string(),
            })
            .collect(),
        source_boundary_digest: "sha256:boundary".to_string(),
        mesher_identity: FemRuntimeIdentity {
            schema_version: FEM_SCHEMA_VERSION,
            platform: "test".to_string(),
            architecture: "test".to_string(),
            library_name: "fTetWild".to_string(),
            library_version: "pinned".to_string(),
            library_digest: "sha256:binary".to_string(),
            adapter_protocol_version: 1,
            supported_capabilities: vec!["tet4".to_string()],
            notice_digest: "sha256:notice".to_string(),
        },
        meshing_evidence: FemMeshingEvidence {
            schema_version: FEM_SCHEMA_VERSION,
            source_triangle_count: 4,
            inserted_source_triangle_count: 4,
            tagged_boundary_triangle_count: 4,
            maximum_boundary_deviation_mm: 0.0,
            discarded_tet4_component_count: 0,
            discarded_tet4_cell_count: 0,
            discarded_low_quality_tet4_cell_count: 0,
            deterministic_thread_count: 1,
        },
        minimum_scaled_jacobian: 1.0e-6,
    })
    .expect("mesh")
}

fn material() -> FemMaterial {
    FemMaterial {
        schema_version: FEM_SCHEMA_VERSION,
        name: "test".to_string(),
        young_modulus_mpa: 1_000.0,
        poisson_ratio: 0.25,
        density_kg_per_mm3: 1.0e-6,
        yield_strength_mpa: 100.0,
    }
}
