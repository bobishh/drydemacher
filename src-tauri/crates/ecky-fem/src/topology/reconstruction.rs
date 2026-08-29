use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use serde::{Deserialize, Serialize};

use crate::{FemIndexedTet4Mesh, FemPoint3, FemValidationError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemDensityAnchor {
    pub id: String,
    pub cells: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemDensitySupportComponent {
    pub retained_cells: Vec<usize>,
    pub discarded_cells: Vec<usize>,
    pub discarded_active_volume_fraction: f64,
    pub connected_anchor_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemDensitySupportGraphNode {
    pub cell_index: usize,
    pub center_mm: [f64; 3],
    pub density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemDensitySupportGraphEdge {
    pub left_cell_index: usize,
    pub right_cell_index: usize,
    pub length_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemDensitySupportGraph {
    pub nodes: Vec<FemDensitySupportGraphNode>,
    pub edges: Vec<FemDensitySupportGraphEdge>,
    pub anchor_cell_indices: BTreeMap<String, usize>,
    pub discarded_cells: Vec<usize>,
    pub discarded_active_volume_fraction: f64,
    pub connected_anchor_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemDensitySurfaceControls {
    pub density_threshold: f64,
    pub smoothing_passes: usize,
    pub maximum_smoothing_displacement_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemDensitySurfaceMesh {
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub connected_anchor_ids: Vec<String>,
    pub discarded_cell_indices: Vec<usize>,
    pub discarded_active_volume_fraction: f64,
    pub boundary_edge_count: usize,
    pub non_manifold_edge_count: usize,
    pub connected_component_count: usize,
    pub signed_volume_mm3: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemDensityCenterlineControls {
    pub symmetry_plane_x_mm: f64,
    pub symmetry_tolerance_mm: f64,
    pub smoothing_passes: usize,
    pub maximum_fit_deviation_mm: f64,
    pub minimum_thickness_mm: f64,
    pub maximum_thickness_mm: f64,
    pub maximum_curvature_per_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemDensityCenterlinePoint {
    pub center_mm: [f64; 3],
    pub thickness_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemDensityCenterlineBranch {
    pub points: Vec<FemDensityCenterlinePoint>,
    pub maximum_curvature_per_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemSymmetricDensityCenterlines {
    pub owned_half_branches: Vec<FemDensityCenterlineBranch>,
    pub mirrored_branches: Vec<FemDensityCenterlineBranch>,
    pub connected_anchor_ids: Vec<String>,
}

fn point_distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    ((left[0] - right[0]).powi(2) + (left[1] - right[1]).powi(2) + (left[2] - right[2]).powi(2))
        .sqrt()
}

fn owned_anchor_cells(
    graph: &FemDensitySupportGraph,
    nodes: &BTreeMap<usize, &FemDensitySupportGraphNode>,
    owned: &BTreeSet<usize>,
    controls: &FemDensityCenterlineControls,
) -> Result<BTreeSet<usize>, FemValidationError> {
    graph
        .anchor_cell_indices
        .iter()
        .map(|(anchor_id, cell)| {
            if owned.contains(cell) {
                return Ok(*cell);
            }
            let anchor = nodes.get(cell).ok_or_else(|| FemValidationError {
                field: "densityReconstruction.anchors".into(),
                message: format!(
                    "anchor '{anchor_id}' references missing support-graph cell {cell}"
                ),
            })?;
            let mirrored_center = [
                2.0 * controls.symmetry_plane_x_mm - anchor.center_mm[0],
                anchor.center_mm[1],
                anchor.center_mm[2],
            ];
            let counterpart = owned
                .iter()
                .filter_map(|candidate| {
                    nodes
                        .get(candidate)
                        .map(|node| (*candidate, point_distance(node.center_mm, mirrored_center)))
                })
                .min_by(|left, right| {
                    left.1
                        .total_cmp(&right.1)
                        .then_with(|| left.0.cmp(&right.0))
                })
                .ok_or_else(|| FemValidationError {
                    field: "densityReconstruction.symmetry".into(),
                    message: format!("anchor '{anchor_id}' has no owned support-graph node"),
                })?
                .0;
            Ok(counterpart)
        })
        .collect()
}

pub fn fit_symmetric_density_centerlines(
    graph: &FemDensitySupportGraph,
    controls: &FemDensityCenterlineControls,
) -> Result<FemSymmetricDensityCenterlines, FemValidationError> {
    validate_centerline_controls(controls)?;
    if graph.nodes.is_empty() {
        return error(
            "densityReconstruction.graph",
            "support graph must contain at least one node",
        );
    }
    let nodes = graph
        .nodes
        .iter()
        .map(|node| (node.cell_index, node))
        .collect::<BTreeMap<_, _>>();
    let owned = graph
        .nodes
        .iter()
        .filter(|node| {
            node.center_mm[0] >= controls.symmetry_plane_x_mm - controls.symmetry_tolerance_mm
        })
        .map(|node| node.cell_index)
        .collect::<BTreeSet<_>>();
    let mut adjacency = BTreeMap::<usize, Vec<usize>>::new();
    for edge in &graph.edges {
        if owned.contains(&edge.left_cell_index) && owned.contains(&edge.right_cell_index) {
            adjacency
                .entry(edge.left_cell_index)
                .or_default()
                .push(edge.right_cell_index);
            adjacency
                .entry(edge.right_cell_index)
                .or_default()
                .push(edge.left_cell_index);
        }
    }
    for neighbours in adjacency.values_mut() {
        neighbours.sort_unstable();
        neighbours.dedup();
    }
    let owned_anchor_cells = owned_anchor_cells(graph, &nodes, &owned, controls)?;
    let mut endpoints = adjacency
        .iter()
        .filter_map(|(cell, neighbours)| {
            (neighbours.len() != 2 || owned_anchor_cells.contains(cell)).then_some(*cell)
        })
        .collect::<Vec<_>>();
    endpoints.sort_unstable();
    let mut visited = BTreeSet::<(usize, usize)>::new();
    let mut paths = Vec::new();
    for start in endpoints {
        for next in adjacency.get(&start).into_iter().flatten().copied() {
            let edge = ordered_edge(start, next);
            if visited.contains(&edge) {
                continue;
            }
            visited.insert(edge);
            let mut path = vec![start, next];
            let mut previous = start;
            let mut current = next;
            while adjacency.get(&current).is_some_and(|neighbours| {
                neighbours.len() == 2 && !owned_anchor_cells.contains(&current)
            }) {
                let following = adjacency[&current]
                    .iter()
                    .copied()
                    .find(|candidate| *candidate != previous)
                    .expect("degree-two node has a forward neighbour");
                if !visited.insert(ordered_edge(current, following)) {
                    break;
                }
                path.push(following);
                previous = current;
                current = following;
            }
            paths.push(path);
        }
    }
    if paths.is_empty() {
        return error(
            "densityReconstruction.symmetry",
            "owned lateral half contains no connected branch",
        );
    }

    let mut owned_half_branches = Vec::with_capacity(paths.len());
    for path in paths {
        let mut points = path
            .iter()
            .map(|cell| {
                let node = nodes[cell];
                let mut center_mm = node.center_mm;
                if (center_mm[0] - controls.symmetry_plane_x_mm).abs()
                    <= controls.symmetry_tolerance_mm
                {
                    center_mm[0] = controls.symmetry_plane_x_mm;
                }
                FemDensityCenterlinePoint {
                    center_mm,
                    thickness_mm: controls.minimum_thickness_mm
                        + node.density.clamp(0.0, 1.0)
                            * (controls.maximum_thickness_mm - controls.minimum_thickness_mm),
                }
            })
            .collect::<Vec<_>>();
        for _ in 0..controls.smoothing_passes {
            let old = points.clone();
            for index in 1..points.len().saturating_sub(1) {
                for axis in 0..3 {
                    points[index].center_mm[axis] = 0.25 * old[index - 1].center_mm[axis]
                        + 0.5 * old[index].center_mm[axis]
                        + 0.25 * old[index + 1].center_mm[axis];
                }
                points[index].thickness_mm = 0.25 * old[index - 1].thickness_mm
                    + 0.5 * old[index].thickness_mm
                    + 0.25 * old[index + 1].thickness_mm;
            }
        }
        points = simplify_centerline(&points, controls.maximum_fit_deviation_mm);
        let maximum_curvature_per_mm = polyline_maximum_curvature(&points);
        if maximum_curvature_per_mm > controls.maximum_curvature_per_mm {
            return error(
                "densityReconstruction.maximumCurvaturePerMm",
                "smoothed support branch exceeds declared curvature bound",
            );
        }
        owned_half_branches.push(FemDensityCenterlineBranch {
            points,
            maximum_curvature_per_mm,
        });
    }
    let mirrored_branches = owned_half_branches
        .iter()
        .map(|branch| FemDensityCenterlineBranch {
            points: branch
                .points
                .iter()
                .map(|point| {
                    let mut mirrored = point.clone();
                    mirrored.center_mm[0] = 2.0 * controls.symmetry_plane_x_mm - point.center_mm[0];
                    mirrored
                })
                .collect(),
            maximum_curvature_per_mm: branch.maximum_curvature_per_mm,
        })
        .collect();
    Ok(FemSymmetricDensityCenterlines {
        owned_half_branches,
        mirrored_branches,
        connected_anchor_ids: graph.connected_anchor_ids.clone(),
    })
}

fn ordered_edge(left: usize, right: usize) -> (usize, usize) {
    (left.min(right), left.max(right))
}

fn simplify_centerline(
    points: &[FemDensityCenterlinePoint],
    maximum_deviation_mm: f64,
) -> Vec<FemDensityCenterlinePoint> {
    if points.len() <= 2 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut spans = vec![(0usize, points.len() - 1)];
    while let Some((start, end)) = spans.pop() {
        if end <= start + 1 {
            continue;
        }
        let mut farthest = None;
        for index in start + 1..end {
            let deviation = point_segment_distance(
                points[index].center_mm,
                points[start].center_mm,
                points[end].center_mm,
            );
            if farthest.is_none_or(|(_, old)| deviation > old) {
                farthest = Some((index, deviation));
            }
        }
        if let Some((index, deviation)) = farthest {
            if deviation > maximum_deviation_mm {
                keep[index] = true;
                spans.push((index, end));
                spans.push((start, index));
            }
        }
    }
    points
        .iter()
        .zip(keep)
        .filter_map(|(point, keep)| keep.then_some(point.clone()))
        .collect()
}

fn point_segment_distance(point: [f64; 3], start: [f64; 3], end: [f64; 3]) -> f64 {
    let delta = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
    let length_squared = dot3(delta, delta);
    if length_squared <= 1.0e-24 {
        return distance(point, start);
    }
    let offset = [
        point[0] - start[0],
        point[1] - start[1],
        point[2] - start[2],
    ];
    let t = (dot3(offset, delta) / length_squared).clamp(0.0, 1.0);
    distance(
        point,
        [
            start[0] + t * delta[0],
            start[1] + t * delta[1],
            start[2] + t * delta[2],
        ],
    )
}

fn polyline_maximum_curvature(points: &[FemDensityCenterlinePoint]) -> f64 {
    points
        .windows(3)
        .filter_map(|window| {
            let left = distance(window[0].center_mm, window[1].center_mm);
            let right = distance(window[1].center_mm, window[2].center_mm);
            if left <= 1.0e-12 || right <= 1.0e-12 {
                return None;
            }
            let a = [
                (window[0].center_mm[0] - window[1].center_mm[0]) / left,
                (window[0].center_mm[1] - window[1].center_mm[1]) / left,
                (window[0].center_mm[2] - window[1].center_mm[2]) / left,
            ];
            let b = [
                (window[2].center_mm[0] - window[1].center_mm[0]) / right,
                (window[2].center_mm[1] - window[1].center_mm[1]) / right,
                (window[2].center_mm[2] - window[1].center_mm[2]) / right,
            ];
            let angle = dot3(a, b).clamp(-1.0, 1.0).acos();
            Some((std::f64::consts::PI - angle) / (0.5 * (left + right)))
        })
        .fold(0.0, f64::max)
}

fn dot3(left: [f64; 3], right: [f64; 3]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn validate_centerline_controls(
    controls: &FemDensityCenterlineControls,
) -> Result<(), FemValidationError> {
    if !controls.symmetry_plane_x_mm.is_finite()
        || !controls.symmetry_tolerance_mm.is_finite()
        || controls.symmetry_tolerance_mm < 0.0
    {
        return error(
            "densityReconstruction.symmetry",
            "symmetry controls must be finite",
        );
    }
    if !controls.minimum_thickness_mm.is_finite()
        || !controls.maximum_thickness_mm.is_finite()
        || controls.minimum_thickness_mm <= 0.0
        || controls.maximum_thickness_mm < controls.minimum_thickness_mm
    {
        return error(
            "densityReconstruction.thickness",
            "thickness bounds must be positive, finite, and ordered",
        );
    }
    if !controls.maximum_fit_deviation_mm.is_finite() || controls.maximum_fit_deviation_mm < 0.0 {
        return error(
            "densityReconstruction.maximumFitDeviationMm",
            "fit deviation bound must be finite and non-negative",
        );
    }
    if !controls.maximum_curvature_per_mm.is_finite() || controls.maximum_curvature_per_mm <= 0.0 {
        return error(
            "densityReconstruction.maximumCurvaturePerMm",
            "curvature bound must be positive and finite",
        );
    }
    Ok(())
}

pub fn extract_density_support_component(
    mesh: &FemIndexedTet4Mesh,
    densities: &[f64],
    threshold: f64,
    anchors: &[FemDensityAnchor],
) -> Result<FemDensitySupportComponent, FemValidationError> {
    validate_inputs(mesh, densities, threshold, anchors)?;
    let active = densities
        .iter()
        .map(|density| *density >= threshold)
        .collect::<Vec<_>>();
    let mut components = DisjointSet::new(mesh.cells.len());
    let mut faces = BTreeMap::<[u32; 3], usize>::new();
    for (cell_index, cell) in mesh.cells.iter().enumerate() {
        if !active[cell_index] {
            continue;
        }
        for mut face in [
            [cell[0], cell[1], cell[2]],
            [cell[0], cell[1], cell[3]],
            [cell[0], cell[2], cell[3]],
            [cell[1], cell[2], cell[3]],
        ] {
            face.sort_unstable();
            if let Some(other) = faces.insert(face, cell_index) {
                components.union(cell_index, other);
            }
        }
    }

    let anchor_roots = anchors
        .iter()
        .map(|anchor| {
            anchor
                .cells
                .iter()
                .copied()
                .filter(|index| active[*index])
                .map(|index| components.find(index))
                .collect::<BTreeSet<_>>()
        })
        .collect::<Vec<_>>();
    if anchor_roots.iter().any(BTreeSet::is_empty) {
        return error(
            "densityReconstruction.anchors",
            "every anchor must intersect the active density field",
        );
    }
    let common_roots = anchor_roots[1..]
        .iter()
        .fold(anchor_roots[0].clone(), |common, roots| {
            common.intersection(roots).copied().collect()
        });
    if common_roots.is_empty() {
        return error(
            "densityReconstruction.anchors",
            "no active component connects every required anchor",
        );
    }

    let volumes = mesh
        .cells
        .iter()
        .map(|cell| tet_volume(mesh, *cell))
        .collect::<Result<Vec<_>, _>>()?;
    let mut volume_by_root = BTreeMap::<usize, f64>::new();
    for (cell_index, volume) in volumes.iter().enumerate() {
        if active[cell_index] {
            *volume_by_root
                .entry(components.find(cell_index))
                .or_default() += volume;
        }
    }
    let retained_root = common_roots
        .into_iter()
        .max_by(|left, right| {
            volume_by_root[left]
                .total_cmp(&volume_by_root[right])
                .then_with(|| right.cmp(left))
        })
        .expect("common roots checked non-empty");
    let mut retained_cells = Vec::new();
    let mut discarded_cells = Vec::new();
    let mut active_volume = 0.0;
    let mut discarded_volume = 0.0;
    for (cell_index, volume) in volumes.iter().enumerate() {
        if !active[cell_index] {
            continue;
        }
        active_volume += volume;
        if components.find(cell_index) == retained_root {
            retained_cells.push(cell_index);
        } else {
            discarded_cells.push(cell_index);
            discarded_volume += volume;
        }
    }
    Ok(FemDensitySupportComponent {
        retained_cells,
        discarded_cells,
        discarded_active_volume_fraction: discarded_volume / active_volume,
        connected_anchor_ids: anchors.iter().map(|anchor| anchor.id.clone()).collect(),
    })
}

pub fn reconstruct_density_surface(
    mesh: &FemIndexedTet4Mesh,
    densities: &[f64],
    anchors: &[FemDensityAnchor],
    controls: &FemDensitySurfaceControls,
) -> Result<FemDensitySurfaceMesh, FemValidationError> {
    validate_density_surface_controls(controls)?;
    let component =
        extract_density_support_component(mesh, densities, controls.density_threshold, anchors)?;
    let retained = component
        .retained_cells
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut face_owners = BTreeMap::<[u32; 3], Vec<[u32; 3]>>::new();
    for cell_index in retained {
        let cell = mesh.cells[cell_index];
        for (face, opposite) in [
            ([cell[0], cell[1], cell[2]], cell[3]),
            ([cell[0], cell[1], cell[3]], cell[2]),
            ([cell[0], cell[2], cell[3]], cell[1]),
            ([cell[1], cell[2], cell[3]], cell[0]),
        ] {
            let oriented = orient_tet_face_outward(mesh, face, opposite)?;
            let mut key = face;
            key.sort_unstable();
            face_owners.entry(key).or_default().push(oriented);
        }
    }
    if face_owners.values().any(|owners| owners.len() > 2) {
        return error(
            "densityReconstruction.surface",
            "retained Tet4 cells contain a non-manifold face shared by more than two cells",
        );
    }
    for owners in face_owners.values().filter(|owners| owners.len() == 2) {
        if !opposite_triangle_winding(owners[0], owners[1]) {
            return error(
                "densityReconstruction.surface",
                "adjacent retained Tet4 cells have inconsistent face winding",
            );
        }
    }
    let boundary_faces = face_owners
        .into_values()
        .filter_map(|owners| (owners.len() == 1).then_some(owners[0]))
        .collect::<Vec<_>>();
    if boundary_faces.is_empty() {
        return error(
            "densityReconstruction.surface",
            "retained density component has no boundary faces",
        );
    }

    let used_nodes = boundary_faces
        .iter()
        .flatten()
        .copied()
        .collect::<BTreeSet<_>>();
    let node_map = used_nodes
        .iter()
        .enumerate()
        .map(|(dense, source)| (*source, dense as u32))
        .collect::<BTreeMap<_, _>>();
    let vertices = used_nodes
        .iter()
        .map(|index| {
            mesh.nodes
                .get(*index as usize)
                .map(|point| [point.x_mm, point.y_mm, point.z_mm])
                .ok_or_else(|| FemValidationError {
                    field: "densityReconstruction.mesh".into(),
                    message: format!("surface references out-of-range node {index}"),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut triangles = boundary_faces
        .into_iter()
        .map(|face| [node_map[&face[0]], node_map[&face[1]], node_map[&face[2]]])
        .collect::<Vec<_>>();
    let mut topology = density_surface_topology(&vertices, &triangles)?;
    if topology.signed_volume_mm3 < 0.0 {
        for triangle in &mut triangles {
            triangle.swap(1, 2);
        }
        topology = density_surface_topology(&vertices, &triangles)?;
    }
    if topology.boundary_edge_count != 0
        || topology.non_manifold_edge_count != 0
        || topology.connected_component_count != 1
        || topology.signed_volume_mm3 <= 0.0
    {
        return error(
            "densityReconstruction.surface",
            "reconstructed density surface is not one closed positive-volume manifold",
        );
    }

    Ok(FemDensitySurfaceMesh {
        vertices,
        triangles,
        connected_anchor_ids: component.connected_anchor_ids,
        discarded_cell_indices: component.discarded_cells,
        discarded_active_volume_fraction: component.discarded_active_volume_fraction,
        boundary_edge_count: topology.boundary_edge_count,
        non_manifold_edge_count: topology.non_manifold_edge_count,
        connected_component_count: topology.connected_component_count,
        signed_volume_mm3: topology.signed_volume_mm3,
    })
}

fn validate_density_surface_controls(
    controls: &FemDensitySurfaceControls,
) -> Result<(), FemValidationError> {
    if controls.smoothing_passes != 0 {
        return error(
            "densityReconstruction.smoothingPasses",
            "surface smoothing is not admitted until anchor-preserving displacement checks exist",
        );
    }
    if !controls.maximum_smoothing_displacement_mm.is_finite()
        || controls.maximum_smoothing_displacement_mm < 0.0
    {
        return error(
            "densityReconstruction.maximumSmoothingDisplacementMm",
            "must be finite and non-negative",
        );
    }
    Ok(())
}

fn orient_tet_face_outward(
    mesh: &FemIndexedTet4Mesh,
    mut face: [u32; 3],
    opposite: u32,
) -> Result<[u32; 3], FemValidationError> {
    let point = |index: u32| {
        mesh.nodes
            .get(index as usize)
            .copied()
            .ok_or_else(|| FemValidationError {
                field: "densityReconstruction.mesh".into(),
                message: format!("Tet4 face references out-of-range node {index}"),
            })
    };
    let a = point(face[0])?;
    let b = point(face[1])?;
    let c = point(face[2])?;
    let interior = point(opposite)?;
    let normal = cross(subtract(b, a), subtract(c, a));
    if dot(normal, subtract(interior, a)) > 0.0 {
        face.swap(1, 2);
    }
    Ok(face)
}

fn opposite_triangle_winding(left: [u32; 3], right: [u32; 3]) -> bool {
    [(left[0], left[1]), (left[1], left[2]), (left[2], left[0])]
        .into_iter()
        .all(|edge| {
            [
                (right[0], right[1]),
                (right[1], right[2]),
                (right[2], right[0]),
            ]
            .contains(&(edge.1, edge.0))
        })
}

struct DensitySurfaceTopology {
    boundary_edge_count: usize,
    non_manifold_edge_count: usize,
    connected_component_count: usize,
    signed_volume_mm3: f64,
}

fn density_surface_topology(
    vertices: &[[f64; 3]],
    triangles: &[[u32; 3]],
) -> Result<DensitySurfaceTopology, FemValidationError> {
    let mut edge_triangles = BTreeMap::<(u32, u32), Vec<usize>>::new();
    let mut signed_volume_mm3 = 0.0;
    for (triangle_index, triangle) in triangles.iter().enumerate() {
        let [a, b, c] = triangle.map(|index| vertices[index as usize]);
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let area_normal = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        if dot(area_normal, area_normal) <= 1.0e-24 {
            return error(
                "densityReconstruction.surface",
                "reconstructed surface contains a zero-area triangle",
            );
        }
        signed_volume_mm3 += (a[0] * (b[1] * c[2] - b[2] * c[1])
            + a[1] * (b[2] * c[0] - b[0] * c[2])
            + a[2] * (b[0] * c[1] - b[1] * c[0]))
            / 6.0;
        for (left, right) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            edge_triangles
                .entry((left.min(right), left.max(right)))
                .or_default()
                .push(triangle_index);
        }
    }
    let boundary_edge_count = edge_triangles
        .values()
        .filter(|owners| owners.len() == 1)
        .count();
    let non_manifold_edge_count = edge_triangles
        .values()
        .filter(|owners| owners.len() > 2)
        .count();
    let mut adjacency = vec![Vec::new(); triangles.len()];
    for owners in edge_triangles.values().filter(|owners| owners.len() == 2) {
        adjacency[owners[0]].push(owners[1]);
        adjacency[owners[1]].push(owners[0]);
    }
    let mut visited = vec![false; triangles.len()];
    let mut connected_component_count = 0;
    for start in 0..triangles.len() {
        if visited[start] {
            continue;
        }
        connected_component_count += 1;
        visited[start] = true;
        let mut stack = vec![start];
        while let Some(current) = stack.pop() {
            for neighbour in &adjacency[current] {
                if !visited[*neighbour] {
                    visited[*neighbour] = true;
                    stack.push(*neighbour);
                }
            }
        }
    }
    Ok(DensitySurfaceTopology {
        boundary_edge_count,
        non_manifold_edge_count,
        connected_component_count,
        signed_volume_mm3,
    })
}

pub fn extract_density_support_graph(
    mesh: &FemIndexedTet4Mesh,
    densities: &[f64],
    threshold: f64,
    anchors: &[FemDensityAnchor],
) -> Result<FemDensitySupportGraph, FemValidationError> {
    let component = extract_density_support_component(mesh, densities, threshold, anchors)?;
    let retained = component
        .retained_cells
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let adjacency = retained_adjacency(mesh, &retained);
    let root = best_anchor_cell(&anchors[0], &retained, densities)
        .expect("component extraction proved every anchor intersects retained component");
    let mut graph_cells = BTreeSet::from([root]);
    let mut graph_edges = BTreeSet::new();
    let mut anchor_cell_indices = BTreeMap::from([(anchors[0].id.clone(), root)]);
    for anchor in anchors.iter().skip(1) {
        let targets = anchor
            .cells
            .iter()
            .copied()
            .filter(|index| retained.contains(index))
            .collect::<BTreeSet<_>>();
        let path = shortest_density_path(mesh, densities, &adjacency, root, &targets)?;
        let anchor_cell = *path
            .last()
            .expect("density path always includes its root and target");
        anchor_cell_indices.insert(anchor.id.clone(), anchor_cell);
        graph_cells.extend(path.iter().copied());
        for pair in path.windows(2) {
            graph_edges.insert((pair[0].min(pair[1]), pair[0].max(pair[1])));
        }
    }
    let centers = mesh
        .cells
        .iter()
        .map(|cell| cell_center(mesh, *cell))
        .collect::<Result<Vec<_>, _>>()?;
    let nodes = graph_cells
        .into_iter()
        .map(|cell_index| FemDensitySupportGraphNode {
            cell_index,
            center_mm: centers[cell_index],
            density: densities[cell_index],
        })
        .collect();
    let edges = graph_edges
        .into_iter()
        .map(
            |(left_cell_index, right_cell_index)| FemDensitySupportGraphEdge {
                left_cell_index,
                right_cell_index,
                length_mm: distance(centers[left_cell_index], centers[right_cell_index]),
            },
        )
        .collect();
    Ok(FemDensitySupportGraph {
        nodes,
        edges,
        anchor_cell_indices,
        discarded_cells: component.discarded_cells,
        discarded_active_volume_fraction: component.discarded_active_volume_fraction,
        connected_anchor_ids: component.connected_anchor_ids,
    })
}

fn best_anchor_cell(
    anchor: &FemDensityAnchor,
    retained: &BTreeSet<usize>,
    densities: &[f64],
) -> Option<usize> {
    anchor
        .cells
        .iter()
        .copied()
        .filter(|index| retained.contains(index))
        .max_by(|left, right| {
            densities[*left]
                .total_cmp(&densities[*right])
                .then_with(|| right.cmp(left))
        })
}

fn retained_adjacency(mesh: &FemIndexedTet4Mesh, retained: &BTreeSet<usize>) -> Vec<Vec<usize>> {
    let mut adjacency = vec![Vec::new(); mesh.cells.len()];
    let mut owners = BTreeMap::<[u32; 3], usize>::new();
    for cell_index in retained.iter().copied() {
        let cell = mesh.cells[cell_index];
        for mut face in [
            [cell[0], cell[1], cell[2]],
            [cell[0], cell[1], cell[3]],
            [cell[0], cell[2], cell[3]],
            [cell[1], cell[2], cell[3]],
        ] {
            face.sort_unstable();
            if let Some(other) = owners.insert(face, cell_index) {
                adjacency[cell_index].push(other);
                adjacency[other].push(cell_index);
            }
        }
    }
    for neighbours in &mut adjacency {
        neighbours.sort_unstable();
        neighbours.dedup();
    }
    adjacency
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct QueueEntry {
    cost: f64,
    cell_index: usize,
}

impl Eq for QueueEntry {}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| other.cell_index.cmp(&self.cell_index))
    }
}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn shortest_density_path(
    mesh: &FemIndexedTet4Mesh,
    densities: &[f64],
    adjacency: &[Vec<usize>],
    root: usize,
    targets: &BTreeSet<usize>,
) -> Result<Vec<usize>, FemValidationError> {
    let centers = mesh
        .cells
        .iter()
        .map(|cell| cell_center(mesh, *cell))
        .collect::<Result<Vec<_>, _>>()?;
    let mut costs = vec![f64::INFINITY; mesh.cells.len()];
    let mut previous = vec![None; mesh.cells.len()];
    let mut queue = BinaryHeap::new();
    costs[root] = 0.0;
    queue.push(QueueEntry {
        cost: 0.0,
        cell_index: root,
    });
    let target = loop {
        let Some(entry) = queue.pop() else {
            return error(
                "densityReconstruction.graph",
                "retained component has no path between required anchors",
            );
        };
        if entry.cost > costs[entry.cell_index] {
            continue;
        }
        if targets.contains(&entry.cell_index) {
            break entry.cell_index;
        }
        for neighbour in &adjacency[entry.cell_index] {
            let mean_density = 0.5 * (densities[entry.cell_index] + densities[*neighbour]);
            let edge_cost = distance(centers[entry.cell_index], centers[*neighbour])
                / mean_density.max(1.0e-12).powi(2);
            let candidate = entry.cost + edge_cost;
            if candidate < costs[*neighbour]
                || (candidate == costs[*neighbour]
                    && previous[*neighbour].is_none_or(|old| entry.cell_index < old))
            {
                costs[*neighbour] = candidate;
                previous[*neighbour] = Some(entry.cell_index);
                queue.push(QueueEntry {
                    cost: candidate,
                    cell_index: *neighbour,
                });
            }
        }
    };
    let mut path = vec![target];
    while *path.last().expect("path starts with target") != root {
        let cursor = *path.last().expect("path non-empty");
        path.push(previous[cursor].ok_or_else(|| FemValidationError {
            field: "densityReconstruction.graph".into(),
            message: "shortest-path predecessor chain is incomplete".into(),
        })?);
    }
    path.reverse();
    Ok(path)
}

fn cell_center(mesh: &FemIndexedTet4Mesh, cell: [u32; 4]) -> Result<[f64; 3], FemValidationError> {
    let mut center = [0.0; 3];
    for index in cell {
        let point = mesh
            .nodes
            .get(index as usize)
            .ok_or_else(|| FemValidationError {
                field: "densityReconstruction.mesh".into(),
                message: "Tet4 cell references an out-of-range node".into(),
            })?;
        center[0] += point.x_mm * 0.25;
        center[1] += point.y_mm * 0.25;
        center[2] += point.z_mm * 0.25;
    }
    Ok(center)
}

fn distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left - right).powi(2))
        .sum::<f64>()
        .sqrt()
}

fn validate_inputs(
    mesh: &FemIndexedTet4Mesh,
    densities: &[f64],
    threshold: f64,
    anchors: &[FemDensityAnchor],
) -> Result<(), FemValidationError> {
    if densities.len() != mesh.cells.len() {
        return error(
            "densityReconstruction.densities",
            "length must equal Tet4 cell count",
        );
    }
    if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
        return error(
            "densityReconstruction.threshold",
            "must be finite and within [0, 1]",
        );
    }
    if densities
        .iter()
        .any(|density| !density.is_finite() || !(0.0..=1.0 + 1.0e-12).contains(density))
    {
        return error(
            "densityReconstruction.densities",
            "values must be finite and within [0, 1]",
        );
    }
    if anchors.is_empty() {
        return error(
            "densityReconstruction.anchors",
            "at least one protected anchor is required",
        );
    }
    let mut ids = BTreeSet::new();
    for anchor in anchors {
        if anchor.id.trim().is_empty() || !ids.insert(anchor.id.as_str()) {
            return error(
                "densityReconstruction.anchors",
                "anchor ids must be non-empty and unique",
            );
        }
        if anchor.cells.is_empty() || anchor.cells.iter().any(|index| *index >= mesh.cells.len()) {
            return error(
                "densityReconstruction.anchors",
                "anchor cells must be non-empty and in range",
            );
        }
    }
    Ok(())
}

fn tet_volume(mesh: &FemIndexedTet4Mesh, cell: [u32; 4]) -> Result<f64, FemValidationError> {
    let point = |index: u32| {
        mesh.nodes
            .get(index as usize)
            .copied()
            .ok_or_else(|| FemValidationError {
                field: "densityReconstruction.mesh".into(),
                message: "Tet4 cell references an out-of-range node".into(),
            })
    };
    let a = point(cell[0])?;
    let b = point(cell[1])?;
    let c = point(cell[2])?;
    let d = point(cell[3])?;
    let ab = subtract(b, a);
    let ac = subtract(c, a);
    let ad = subtract(d, a);
    let volume = dot(ab, cross(ac, ad)).abs() / 6.0;
    if !volume.is_finite() || volume <= 0.0 {
        return error(
            "densityReconstruction.mesh",
            "Tet4 cell volume must be positive and finite",
        );
    }
    Ok(volume)
}

fn subtract(left: FemPoint3, right: FemPoint3) -> [f64; 3] {
    [
        left.x_mm - right.x_mm,
        left.y_mm - right.y_mm,
        left.z_mm - right.z_mm,
    ]
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn error<T>(field: &str, message: &str) -> Result<T, FemValidationError> {
    Err(FemValidationError {
        field: field.into(),
        message: message.into(),
    })
}

struct DisjointSet {
    parent: Vec<usize>,
}

impl DisjointSet {
    fn new(count: usize) -> Self {
        Self {
            parent: (0..count).collect(),
        }
    }

    fn find(&mut self, mut index: usize) -> usize {
        while self.parent[index] != index {
            self.parent[index] = self.parent[self.parent[index]];
            index = self.parent[index];
        }
        index
    }

    fn union(&mut self, left: usize, right: usize) {
        let left = self.find(left);
        let right = self.find(right);
        if left != right {
            let (root, child) = if left < right {
                (left, right)
            } else {
                (right, left)
            };
            self.parent[child] = root;
        }
    }
}
