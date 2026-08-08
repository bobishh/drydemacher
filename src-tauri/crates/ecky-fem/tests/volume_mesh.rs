use ecky_fem::{
    FemFaceTarget, FemMeshingEvidence, FemPoint3, FemRuntimeIdentity, FemVolumeMesh,
    FemVolumeMeshInput, FEM_SCHEMA_VERSION,
};

#[test]
fn canonical_volume_mesh_normalizes_orientation_and_proves_exterior_ownership() {
    let mesh = FemVolumeMesh::validate_and_canonicalize(FemVolumeMeshInput {
        schema_version: FEM_SCHEMA_VERSION,
        nodes: vec![
            point(0.0, 0.0, 0.0),
            point(0.0, 1.0, 0.0),
            point(1.0, 0.0, 0.0),
            point(0.0, 0.0, 1.0),
        ],
        cells: vec![[0, 1, 2, 3]],
        boundary_triangles: vec![[1, 3, 2], [0, 2, 3], [0, 3, 1], [0, 1, 2]],
        boundary_face_group_indices: vec![3, 2, 1, 0],
        face_group_count: 4,
        face_group_targets: face_group_targets(),
        source_boundary_digest: "sha256:boundary".to_string(),
        mesher_identity: runtime_identity(),
        meshing_evidence: meshing_evidence(),
        minimum_scaled_jacobian: 1.0e-6,
    })
    .expect("valid one-cell mesh");

    assert_eq!(mesh.cells.len(), 1);
    assert!(mesh.quality.minimum_signed_volume_mm3 > 0.0);
    assert!(mesh.quality.minimum_scaled_jacobian > 0.0);
    assert!(mesh.quality.minimum_radius_ratio > 0.0);
    assert_eq!(mesh.quality.connected_component_count, 1);
    assert!(mesh.quality.worst_cell_centroid_mm.x_mm.is_finite());
    assert_eq!(mesh.boundary_triangles.len(), 4);
    assert_eq!(mesh.boundary_face_group_indices.len(), 4);
    assert_eq!(mesh.quality.boundary_area_mm2_by_group.len(), 4);
    assert!(mesh.content_digest.starts_with("sha256:"));

    let repeated = FemVolumeMesh::validate_and_canonicalize(FemVolumeMeshInput {
        nodes: vec![
            point(0.0, 0.0, 0.0),
            point(0.0, 1.0, 0.0),
            point(1.0, 0.0, 0.0),
            point(0.0, 0.0, 1.0),
        ],
        cells: vec![[0, 1, 2, 3]],
        boundary_triangles: vec![[1, 3, 2], [0, 2, 3], [0, 3, 1], [0, 1, 2]],
        boundary_face_group_indices: vec![3, 2, 1, 0],
        schema_version: FEM_SCHEMA_VERSION,
        face_group_count: 4,
        face_group_targets: face_group_targets(),
        source_boundary_digest: "sha256:boundary".to_string(),
        mesher_identity: runtime_identity(),
        meshing_evidence: meshing_evidence(),
        minimum_scaled_jacobian: 1.0e-6,
    })
    .expect("repeat mesh");
    assert_eq!(mesh.content_digest, repeated.content_digest);
}

#[test]
fn volume_mesh_rejects_nonfinite_oob_degenerate_low_quality_and_disconnected_cells() {
    let mut nonfinite = fixture_input();
    nonfinite.nodes[0].x_mm = f64::NAN;
    assert!(FemVolumeMesh::validate_and_canonicalize(nonfinite)
        .unwrap_err()
        .message
        .contains("finite"));

    let mut out_of_bounds = fixture_input();
    out_of_bounds.cells[0][3] = 99;
    assert!(FemVolumeMesh::validate_and_canonicalize(out_of_bounds)
        .unwrap_err()
        .message
        .contains("out-of-range"));

    let mut degenerate = fixture_input();
    degenerate.nodes[3] = point(0.25, 0.25, 0.0);
    assert!(FemVolumeMesh::validate_and_canonicalize(degenerate)
        .unwrap_err()
        .message
        .contains("zero signed volume"));

    let mut low_quality = fixture_input();
    low_quality.minimum_scaled_jacobian = 0.9;
    assert!(FemVolumeMesh::validate_and_canonicalize(low_quality)
        .unwrap_err()
        .message
        .contains("below threshold"));

    let mut disconnected = fixture_input();
    disconnected.nodes.extend([
        point(10.0, 0.0, 0.0),
        point(11.0, 0.0, 0.0),
        point(10.0, 1.0, 0.0),
        point(10.0, 0.0, 1.0),
    ]);
    disconnected.cells.push([4, 5, 6, 7]);
    disconnected
        .boundary_triangles
        .extend([[5, 7, 6], [4, 6, 7], [4, 7, 5], [4, 5, 6]]);
    disconnected
        .boundary_face_group_indices
        .extend([4, 5, 6, 7]);
    disconnected.meshing_evidence.tagged_boundary_triangle_count = 8;
    disconnected.face_group_count = 8;
    disconnected
        .face_group_targets
        .extend((4..8).map(|index| FemFaceTarget {
            schema_version: FEM_SCHEMA_VERSION,
            part_id: "body".to_string(),
            canonical_target_id: format!("body:face:{index}"),
            durable_target_id: format!("body:stable:{index}"),
            source_geometry_digest: "sha256:geometry".to_string(),
        }));
    assert!(FemVolumeMesh::validate_and_canonicalize(disconnected)
        .unwrap_err()
        .message
        .contains("connected component"));
}

#[test]
fn volume_mesh_rejects_missing_exterior_group_and_duplicate_cell() {
    let mut missing_boundary = fixture_input();
    missing_boundary.boundary_triangles.pop();
    missing_boundary.boundary_face_group_indices.pop();
    missing_boundary
        .meshing_evidence
        .tagged_boundary_triangle_count = 3;
    let error = FemVolumeMesh::validate_and_canonicalize(missing_boundary)
        .expect_err("missing exterior facet");
    assert!(error.message.contains("exterior facet coverage"));

    let mut duplicate_cell = fixture_input();
    duplicate_cell.cells.push([3, 2, 1, 0]);
    let error =
        FemVolumeMesh::validate_and_canonicalize(duplicate_cell).expect_err("duplicate cell");
    assert!(error.message.contains("duplicate"));
}

#[test]
fn volume_mesh_content_mutation_changes_canonical_result_identity() {
    let original = FemVolumeMesh::validate_and_canonicalize(fixture_input()).expect("mesh");
    let mut changed_input = fixture_input();
    changed_input.nodes[3].z_mm = 1.25;
    let changed =
        FemVolumeMesh::validate_and_canonicalize(changed_input).expect("changed valid mesh");

    assert_ne!(original.content_digest, changed.content_digest);
}

fn fixture_input() -> FemVolumeMeshInput {
    FemVolumeMeshInput {
        schema_version: FEM_SCHEMA_VERSION,
        nodes: vec![
            point(0.0, 0.0, 0.0),
            point(1.0, 0.0, 0.0),
            point(0.0, 1.0, 0.0),
            point(0.0, 0.0, 1.0),
        ],
        cells: vec![[0, 1, 2, 3]],
        boundary_triangles: vec![[1, 3, 2], [0, 2, 3], [0, 3, 1], [0, 1, 2]],
        boundary_face_group_indices: vec![0, 1, 2, 3],
        face_group_count: 4,
        face_group_targets: face_group_targets(),
        source_boundary_digest: "sha256:boundary".to_string(),
        mesher_identity: runtime_identity(),
        meshing_evidence: meshing_evidence(),
        minimum_scaled_jacobian: 1.0e-6,
    }
}

fn face_group_targets() -> Vec<FemFaceTarget> {
    (0..4)
        .map(|index| FemFaceTarget {
            schema_version: FEM_SCHEMA_VERSION,
            part_id: "body".to_string(),
            canonical_target_id: format!("body:face:{index}"),
            durable_target_id: format!("body:stable:{index}"),
            source_geometry_digest: "sha256:geometry".to_string(),
        })
        .collect()
}

fn point(x_mm: f64, y_mm: f64, z_mm: f64) -> FemPoint3 {
    FemPoint3::new(x_mm, y_mm, z_mm)
}

fn runtime_identity() -> FemRuntimeIdentity {
    FemRuntimeIdentity {
        schema_version: FEM_SCHEMA_VERSION,
        platform: "test".to_string(),
        architecture: "test".to_string(),
        library_name: "fTetWild".to_string(),
        library_version: "pinned".to_string(),
        library_digest: "sha256:binary".to_string(),
        adapter_protocol_version: 1,
        supported_capabilities: vec!["tet4".to_string(), "wideSurfaceTags".to_string()],
        notice_digest: "sha256:notice".to_string(),
    }
}

fn meshing_evidence() -> FemMeshingEvidence {
    FemMeshingEvidence {
        schema_version: FEM_SCHEMA_VERSION,
        source_triangle_count: 4,
        inserted_source_triangle_count: 4,
        tagged_boundary_triangle_count: 4,
        maximum_boundary_deviation_mm: 0.0,
        deterministic_thread_count: 1,
    }
}
