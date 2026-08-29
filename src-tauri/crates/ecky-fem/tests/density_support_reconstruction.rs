use std::collections::BTreeMap;

use ecky_fem::{
    extract_density_support_component, extract_density_support_graph,
    fit_symmetric_density_centerlines, FemDensityAnchor, FemDensityCenterlineControls,
    FemDensitySupportGraph, FemDensitySupportGraphEdge, FemDensitySupportGraphNode,
    FemIndexedTet4Mesh, FemPoint3, FEM_SCHEMA_VERSION,
};

#[test]
fn density_reconstruction_keeps_anchor_connected_path_and_rejects_islands() {
    let mesh = support_path_with_island();
    let densities = [0.92, 0.84, 0.78, 0.88, 0.97];
    let anchors = [
        FemDensityAnchor {
            id: "source-anchor".into(),
            cells: vec![0],
        },
        FemDensityAnchor {
            id: "target-anchor".into(),
            cells: vec![3],
        },
    ];

    let extracted = extract_density_support_component(&mesh, &densities, 0.5, &anchors).unwrap();

    assert_eq!(extracted.retained_cells, vec![0, 1, 2, 3]);
    assert_eq!(extracted.discarded_cells, vec![4]);
    assert!(extracted.discarded_active_volume_fraction > 0.0);
    assert_eq!(
        extracted.connected_anchor_ids,
        ["source-anchor", "target-anchor"]
    );

    let disconnected = [
        anchors[0].clone(),
        FemDensityAnchor {
            id: "island-contact".into(),
            cells: vec![4],
        },
    ];
    let error = extract_density_support_component(&mesh, &densities, 0.5, &disconnected)
        .expect_err("anchors in separate active components must fail");
    assert_eq!(error.field, "densityReconstruction.anchors");
}

#[test]
fn density_support_graph_is_sparse_deterministic_and_anchor_connected() {
    let mesh = support_path_with_island();
    let densities = [0.92, 0.84, 0.78, 0.88, 0.97];
    let anchors = [
        FemDensityAnchor {
            id: "source-anchor".into(),
            cells: vec![0],
        },
        FemDensityAnchor {
            id: "target-anchor".into(),
            cells: vec![3],
        },
    ];

    let first = extract_density_support_graph(&mesh, &densities, 0.5, &anchors).unwrap();
    let second = extract_density_support_graph(&mesh, &densities, 0.5, &anchors).unwrap();

    assert_eq!(first, second);
    assert_eq!(
        first
            .nodes
            .iter()
            .map(|node| node.cell_index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    assert_eq!(
        first
            .edges
            .iter()
            .map(|edge| (edge.left_cell_index, edge.right_cell_index))
            .collect::<Vec<_>>(),
        vec![(0, 1), (1, 2), (2, 3)]
    );
    assert_eq!(
        first.connected_anchor_ids,
        ["source-anchor", "target-anchor"]
    );
    assert_eq!(first.discarded_cells, vec![4]);
    assert!(first.discarded_active_volume_fraction > 0.0);
}

#[test]
fn density_centerlines_own_one_half_and_mirror_with_bounded_shape() {
    let mesh = support_path_with_island();
    let densities = [0.92, 0.84, 0.78, 0.88, 0.97];
    let anchors = [
        FemDensityAnchor {
            id: "source-anchor".into(),
            cells: vec![0],
        },
        FemDensityAnchor {
            id: "target-anchor".into(),
            cells: vec![3],
        },
    ];
    let graph = extract_density_support_graph(&mesh, &densities, 0.5, &anchors).unwrap();
    let controls = FemDensityCenterlineControls {
        symmetry_plane_x_mm: 0.0,
        symmetry_tolerance_mm: 0.6,
        smoothing_passes: 2,
        maximum_fit_deviation_mm: 0.2,
        minimum_thickness_mm: 2.4,
        maximum_thickness_mm: 5.0,
        maximum_curvature_per_mm: 2.0,
    };

    let candidate = fit_symmetric_density_centerlines(&graph, &controls).unwrap();

    assert!(!candidate.owned_half_branches.is_empty());
    assert_eq!(
        candidate.owned_half_branches.len(),
        candidate.mirrored_branches.len()
    );
    assert_eq!(
        candidate.connected_anchor_ids,
        ["source-anchor", "target-anchor"]
    );
    for (owned, mirrored) in candidate
        .owned_half_branches
        .iter()
        .zip(&candidate.mirrored_branches)
    {
        assert_eq!(owned.points.len(), mirrored.points.len());
        assert!(owned.maximum_curvature_per_mm <= controls.maximum_curvature_per_mm);
        for (left, right) in owned.points.iter().zip(&mirrored.points) {
            assert!((left.center_mm[0] + right.center_mm[0]).abs() <= 1.0e-12);
            assert_eq!(left.center_mm[1], right.center_mm[1]);
            assert_eq!(left.center_mm[2], right.center_mm[2]);
            assert_eq!(left.thickness_mm, right.thickness_mm);
            assert!(left.thickness_mm >= controls.minimum_thickness_mm);
            assert!(left.thickness_mm <= controls.maximum_thickness_mm);
        }
    }
}

#[test]
fn protected_anchor_inside_a_load_path_splits_centerlines_at_the_anchor() {
    let mesh = support_path_with_island();
    let densities = [0.92, 0.84, 0.78, 0.88, 0.1];
    let anchors = [
        FemDensityAnchor {
            id: "source-anchor".into(),
            cells: vec![0],
        },
        FemDensityAnchor {
            id: "interior-anchor".into(),
            cells: vec![2],
        },
        FemDensityAnchor {
            id: "target-anchor".into(),
            cells: vec![3],
        },
    ];
    let graph = extract_density_support_graph(&mesh, &densities, 0.5, &anchors).unwrap();
    assert_eq!(graph.anchor_cell_indices["interior-anchor"], 2);

    let candidate = fit_symmetric_density_centerlines(
        &graph,
        &FemDensityCenterlineControls {
            symmetry_plane_x_mm: 0.0,
            symmetry_tolerance_mm: 0.6,
            smoothing_passes: 0,
            maximum_fit_deviation_mm: 0.0,
            minimum_thickness_mm: 2.4,
            maximum_thickness_mm: 5.0,
            maximum_curvature_per_mm: 10.0,
        },
    )
    .unwrap();
    assert_eq!(candidate.owned_half_branches.len(), 2);
    let interior_anchor_center = graph
        .nodes
        .iter()
        .find(|node| node.cell_index == 2)
        .unwrap()
        .center_mm;
    assert_eq!(
        candidate
            .owned_half_branches
            .iter()
            .filter(|branch| {
                branch.points.first().unwrap().center_mm == interior_anchor_center
                    || branch.points.last().unwrap().center_mm == interior_anchor_center
            })
            .count(),
        2
    );
}

#[test]
fn anchor_on_unowned_half_splits_nearest_owned_sparse_counterpart() {
    let graph = FemDensitySupportGraph {
        nodes: [
            [-3.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [1.0, 5.0, 0.0],
            [3.0, 5.0, 0.0],
            [5.0, 5.0, 0.0],
        ]
        .into_iter()
        .enumerate()
        .map(|(cell_index, center_mm)| FemDensitySupportGraphNode {
            cell_index,
            center_mm,
            density: 0.9,
        })
        .collect(),
        edges: (0..4)
            .map(|left_cell_index| FemDensitySupportGraphEdge {
                left_cell_index,
                right_cell_index: left_cell_index + 1,
                length_mm: 2.0,
            })
            .collect(),
        anchor_cell_indices: BTreeMap::from([
            ("mirrored-anchor".into(), 0),
            ("target-anchor".into(), 4),
        ]),
        discarded_cells: Vec::new(),
        discarded_active_volume_fraction: 0.0,
        connected_anchor_ids: vec!["mirrored-anchor".into(), "target-anchor".into()],
    };

    let candidate = fit_symmetric_density_centerlines(
        &graph,
        &FemDensityCenterlineControls {
            symmetry_plane_x_mm: 0.0,
            symmetry_tolerance_mm: 0.25,
            smoothing_passes: 0,
            maximum_fit_deviation_mm: 0.0,
            minimum_thickness_mm: 2.4,
            maximum_thickness_mm: 5.0,
            maximum_curvature_per_mm: 10.0,
        },
    )
    .unwrap();

    assert_eq!(candidate.owned_half_branches.len(), 2);
    assert_eq!(
        candidate
            .owned_half_branches
            .iter()
            .filter(|branch| {
                branch.points.first().unwrap().center_mm == [3.0, 5.0, 0.0]
                    || branch.points.last().unwrap().center_mm == [3.0, 5.0, 0.0]
            })
            .count(),
        2
    );
}

fn support_path_with_island() -> FemIndexedTet4Mesh {
    FemIndexedTet4Mesh {
        schema_version: FEM_SCHEMA_VERSION,
        nodes: vec![
            FemPoint3::new(0.0, 0.0, 0.0),
            FemPoint3::new(1.0, 0.0, 0.0),
            FemPoint3::new(0.0, 1.0, 0.0),
            FemPoint3::new(0.0, 0.0, 1.0),
            FemPoint3::new(1.0, 1.0, 0.0),
            FemPoint3::new(1.0, 0.0, 1.0),
            FemPoint3::new(2.0, 0.5, 0.5),
            FemPoint3::new(10.0, 0.0, 0.0),
            FemPoint3::new(11.0, 0.0, 0.0),
            FemPoint3::new(10.0, 1.0, 0.0),
            FemPoint3::new(10.0, 0.0, 1.0),
        ],
        cells: vec![
            [0, 1, 2, 3],
            [1, 4, 2, 3],
            [1, 5, 4, 3],
            [1, 6, 5, 3],
            [7, 8, 9, 10],
        ],
    }
}
