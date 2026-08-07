use ecky_cad_lib::capture_guidance::{
    extract_surface_neighborhood_from_stl, source_mesh_content_digest,
};
use ecky_cad_lib::contracts::CaptureSurfaceAnchor;

fn planar_patch_stl() -> String {
    r#"solid planar_patch
facet normal 0 0 1
  outer loop
    vertex 0 0 0
    vertex 1 0 0
    vertex 1 1 0
  endloop
endfacet
facet normal 0 0 1
  outer loop
    vertex 0 0 0
    vertex 1 1 0
    vertex 0 1 0
  endloop
endfacet
endsolid planar_patch
"#
    .to_string()
}

#[test]
fn digest_bound_anchor_expands_into_deterministic_connected_surface_evidence() {
    let path = std::env::temp_dir().join(format!(
        "ecky-capture-neighborhood-{}.stl",
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&path, planar_patch_stl()).expect("write planar STL fixture");
    let digest = source_mesh_content_digest(&path).expect("source mesh digest");
    let anchor = CaptureSurfaceAnchor {
        source_mesh_content_digest: digest.clone(),
        triangle_index: 0,
        barycentric: [1.0 / 3.0; 3],
        source_position: [2.0 / 3.0, 1.0 / 3.0, 0.0],
        source_normal: [0.0, 0.0, 1.0],
    };

    let neighborhood =
        extract_surface_neighborhood_from_stl(&path, "landmark-planar", &anchor, 2.0, 16)
            .expect("connected neighborhood");

    assert_eq!(neighborhood.source_mesh_content_digest, digest);
    assert_eq!(neighborhood.landmark_id, "landmark-planar");
    assert_eq!(neighborhood.seed_triangle_index, 0);
    assert_eq!(neighborhood.triangle_indices, vec![0, 1]);
    assert_eq!(neighborhood.adjacency_edges, vec![[0, 1]]);
    assert_eq!(neighborhood.vertex_indices.len(), 4);
    assert_eq!(neighborhood.sample_count, 4);
    assert!((neighborhood.sampled_area_source_units_squared - 1.0).abs() <= 1.0e-12);
    assert!(neighborhood.radial_coverage_ratio > 0.0);
    assert!(neighborhood.radial_coverage_ratio <= 1.0);
    assert_eq!(neighborhood.mean_normal, [0.0, 0.0, 1.0]);
    assert!(neighborhood.normal_spread_deg <= 1.0e-9);
    assert!(neighborhood.normal_variation_rms_deg <= 1.0e-9);
    assert!(neighborhood.estimated_curvature_per_source_unit <= 1.0e-9);
    assert!(neighborhood.position_rms_source_units > 0.0);
    assert!(neighborhood.planarity_rms_source_units <= 1.0e-12);
    assert!(neighborhood.planarity_max_source_units <= 1.0e-12);
    assert!(neighborhood.position_uncertainty_source_units <= 1.0e-12);
    assert!(neighborhood.reached_mesh_boundary);
    assert!(!neighborhood.truncated_by_budget);

    let repeated =
        extract_surface_neighborhood_from_stl(&path, "landmark-planar", &anchor, 2.0, 16)
            .expect("repeated neighborhood");
    assert_eq!(neighborhood, repeated);

    let _ = std::fs::remove_file(path);
}
