use std::collections::BTreeMap;

use ecky_fem::{
    reconstruct_density_surface, FemDensityAnchor, FemDensitySurfaceControls, FemIndexedTet4Mesh,
    FemPoint3, FEM_SCHEMA_VERSION,
};

#[test]
fn topology_density_reconstructs_a_closed_anchor_connected_surface() {
    let mesh = two_disconnected_tetrahedra();
    let densities = [0.92, 0.88];
    let anchors = [FemDensityAnchor {
        id: "mount".into(),
        cells: vec![0],
    }];

    let surface = reconstruct_density_surface(
        &mesh,
        &densities,
        &anchors,
        &FemDensitySurfaceControls {
            density_threshold: 0.5,
            smoothing_passes: 0,
            maximum_smoothing_displacement_mm: 0.0,
        },
    )
    .expect("anchor-connected topology density must become a printable surface");

    assert_eq!(surface.connected_anchor_ids, ["mount"]);
    assert_eq!(surface.discarded_cell_indices, [1]);
    assert_eq!(surface.boundary_edge_count, 0);
    assert_eq!(surface.non_manifold_edge_count, 0);
    assert_eq!(surface.connected_component_count, 1);
    assert!(surface.signed_volume_mm3 > 0.0);
    assert_eq!(surface.vertices.len(), 4);
    assert_eq!(surface.triangles.len(), 4);

    let incidence = surface.triangles.iter().fold(
        BTreeMap::<(u32, u32), usize>::new(),
        |mut edges, triangle| {
            for [left, right] in [
                [triangle[0], triangle[1]],
                [triangle[1], triangle[2]],
                [triangle[2], triangle[0]],
            ] {
                *edges.entry((left.min(right), left.max(right))).or_default() += 1;
            }
            edges
        },
    );
    assert!(incidence.values().all(|count| *count == 2));
}

fn two_disconnected_tetrahedra() -> FemIndexedTet4Mesh {
    FemIndexedTet4Mesh {
        schema_version: FEM_SCHEMA_VERSION,
        nodes: vec![
            FemPoint3::new(0.0, 0.0, 0.0),
            FemPoint3::new(10.0, 0.0, 0.0),
            FemPoint3::new(0.0, 10.0, 0.0),
            FemPoint3::new(0.0, 0.0, 10.0),
            FemPoint3::new(30.0, 0.0, 0.0),
            FemPoint3::new(40.0, 0.0, 0.0),
            FemPoint3::new(30.0, 10.0, 0.0),
            FemPoint3::new(30.0, 0.0, 10.0),
        ],
        cells: vec![[0, 2, 1, 3], [4, 6, 5, 7]],
    }
}
