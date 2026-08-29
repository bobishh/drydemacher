use ecky_fem::{
    assemble_boundary_conditions, FemConstraint, FemFaceTarget, FemForceVector, FemLoad,
    FemMeshingEvidence, FemOptionalDisplacement, FemPoint3, FemRuntimeIdentity, FemVolumeMesh,
    FemVolumeMeshInput, FEM_SCHEMA_VERSION,
};

#[test]
fn tagged_boundary_conditions_resolve_exact_faces_and_assemble_global_dofs() {
    let mesh = one_tet_mesh();
    let load_face = mesh.face_group_targets[0].clone();
    let fixed_face = mesh.face_group_targets[3].clone();
    let prescribed_face = mesh.face_group_targets[1].clone();
    let assembly = assemble_boundary_conditions(
        &mesh,
        &[FemLoad::SurfaceForce {
            schema_version: FEM_SCHEMA_VERSION,
            name: "tip-load".to_string(),
            faces: vec![load_face],
            total_force_n: FemForceVector {
                x_n: 0.0,
                y_n: 0.0,
                z_n: -12.0,
            },
        }],
        &[FemConstraint::Fixed {
            schema_version: FEM_SCHEMA_VERSION,
            name: "mount".to_string(),
            faces: vec![fixed_face],
        }],
    )
    .expect("assemble tagged conditions");

    let resultant = [
        assembly.rhs_n.iter().step_by(3).sum::<f64>(),
        assembly.rhs_n.iter().skip(1).step_by(3).sum::<f64>(),
        assembly.rhs_n.iter().skip(2).step_by(3).sum::<f64>(),
    ];
    assert_eq!(resultant, [0.0, 0.0, -12.0]);
    assert_eq!(assembly.support_groups.len(), 1);
    assert_eq!(
        assembly
            .dirichlet
            .iter()
            .map(|constraint| constraint.dof_index)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        assembly.dirichlet.len()
    );

    let prescribed = assemble_boundary_conditions(
        &mesh,
        &[],
        &[FemConstraint::PrescribedDisplacement {
            schema_version: FEM_SCHEMA_VERSION,
            name: "settlement".to_string(),
            faces: vec![prescribed_face],
            displacement_mm: FemOptionalDisplacement {
                x_mm: Some(0.01),
                y_mm: None,
                z_mm: None,
            },
        }],
    )
    .expect("component-wise prescribed displacement");
    assert!(prescribed
        .dirichlet
        .iter()
        .all(|constraint| constraint.dof_index % 3 == 0 && constraint.value_mm == 0.01));

    let mut stale = mesh.face_group_targets[0].clone();
    stale.source_geometry_digest = "sha256:stale".to_string();
    let error = assemble_boundary_conditions(
        &mesh,
        &[FemLoad::Pressure {
            schema_version: FEM_SCHEMA_VERSION,
            name: "pressure".to_string(),
            faces: vec![stale],
            pressure_mpa: 1.0,
        }],
        &[],
    )
    .expect_err("stale target");
    assert!(error.message.contains("resolve exactly"));
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
