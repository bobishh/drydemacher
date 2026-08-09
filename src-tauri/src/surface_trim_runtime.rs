use crate::contracts::{AppError, AppResult, CaptureSurfaceAnchor};
use crate::ecky_ir::mesh_asset::{IndexedMeshAsset, MeshAssetSource};
use crate::surface_trim_cap::{apply_surface_trim_cap, SurfaceTrimCapMode, SurfaceTrimCapReport};
use crate::surface_trim_diagnostics::{analyze_surface_trim_mesh, SurfaceTrimMeshDiagnostics};
use crate::surface_trim_external_shapes::{preview_surface_trim_region, SurfaceTrimPathMode};
use crate::surface_trim_mesh::{compose_surface_trim_mesh, TriangleCutInstruction};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const RUNTIME_EPSILON: f64 = 1.0e-9;
const DEFAULT_FLAT_CAP_TOLERANCE: f64 = 0.1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CanonicalSurfaceTrimAnchor {
    pub triangle_index: u64,
    pub barycentric: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceTrimRuntimeOutput {
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub cut_edges: Vec<[u32; 2]>,
    pub diagnostics: SurfaceTrimMeshDiagnostics,
    pub cap_reports: Vec<SurfaceTrimCapReport>,
}

pub fn execute_surface_trim(
    source_stl_path: &Path,
    expected_content_digest: &str,
    loop_anchors: &[CanonicalSurfaceTrimAnchor],
    keep_seed: &CanonicalSurfaceTrimAnchor,
    path_mode: SurfaceTrimPathMode,
    cap_mode: SurfaceTrimCapMode,
) -> AppResult<SurfaceTrimRuntimeOutput> {
    let mesh = IndexedMeshAsset::from_stl(MeshAssetSource::Imported, source_stl_path)?;
    if mesh.content_digest() != expected_content_digest {
        return Err(AppError::conflict(format!(
            "Surface trim source digest changed: expected '{}', got '{}'.",
            expected_content_digest,
            mesh.content_digest()
        )));
    }
    let anchors = loop_anchors
        .iter()
        .map(|anchor| materialize_anchor(&mesh, anchor))
        .collect::<AppResult<Vec<_>>>()?;
    let seed = materialize_anchor(&mesh, keep_seed)?;
    let preview = preview_surface_trim_region(source_stl_path, &anchors, &seed, path_mode)?;

    let (segment_map, crossed_edges) = collect_triangle_segments(&preview.loop_segments)?;
    let retained = preview
        .retained_triangle_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let edge_incidents = build_edge_incidents(mesh.triangles());
    let mut instructions = Vec::with_capacity(segment_map.len());
    for (triangle_index, segments) in segment_map {
        let ordered_cut_points = join_triangle_segments(&segments)?;
        let keep_seed = local_keep_seed(
            mesh.vertices(),
            mesh.triangles(),
            triangle_index,
            crossed_edges.get(&triangle_index),
            retained.contains(&triangle_index),
            &edge_incidents,
            &retained,
        )?;
        instructions.push(TriangleCutInstruction {
            triangle_index,
            ordered_cut_points,
            keep_seed,
        });
    }
    instructions.sort_by_key(|instruction| instruction.triangle_index);

    let mut composed =
        compose_surface_trim_mesh(mesh.vertices(), mesh.triangles(), &retained, &instructions)
            .map_err(|error| {
                AppError::validation(format!("Surface trim mesh composition failed: {error}"))
            })?;
    let initial_diagnostics =
        analyze_surface_trim_mesh(&composed.vertices, &composed.triangles, &composed.cut_edges)
            .map_err(|error| {
                AppError::validation(format!("Surface trim topology failed: {error}"))
            })?;

    let mut cap_reports = Vec::new();
    match cap_mode {
        SurfaceTrimCapMode::Open => {
            let loop_or_chain = initial_diagnostics
                .closed_boundary_loops
                .first()
                .or_else(|| initial_diagnostics.open_boundary_chains.first())
                .ok_or_else(|| {
                    AppError::validation("Surface trim produced no boundary to report.")
                })?;
            let cap = apply_surface_trim_cap(
                &composed.vertices,
                &composed.triangles,
                loop_or_chain,
                SurfaceTrimCapMode::Open,
                DEFAULT_FLAT_CAP_TOLERANCE,
            )
            .map_err(|error| {
                AppError::validation(format!("Surface trim open cap failed: {error}"))
            })?;
            cap_reports.push(cap.report);
        }
        SurfaceTrimCapMode::Flat | SurfaceTrimCapMode::SurfaceFill => {
            if !initial_diagnostics.open_boundary_chains.is_empty()
                || initial_diagnostics.closed_boundary_loops.is_empty()
            {
                return Err(AppError::validation(
                    "Surface trim cap requires closed boundary loops; output contains an open chain.",
                ));
            }
            for boundary_loop in &initial_diagnostics.closed_boundary_loops {
                let cap = apply_surface_trim_cap(
                    &composed.vertices,
                    &composed.triangles,
                    boundary_loop,
                    cap_mode,
                    DEFAULT_FLAT_CAP_TOLERANCE,
                )
                .map_err(|error| {
                    AppError::validation(format!("Surface trim cap failed: {error}"))
                })?;
                composed.vertices = cap.vertices;
                composed.triangles = cap.triangles;
                cap_reports.push(cap.report);
            }
        }
    }

    let diagnostics =
        analyze_surface_trim_mesh(&composed.vertices, &composed.triangles, &composed.cut_edges)
            .map_err(|error| {
                AppError::validation(format!("Surface trim final topology failed: {error}"))
            })?;
    if diagnostics.duplicate_position_count > 0
        || diagnostics.non_manifold_edge_count > 0
        || diagnostics.orientation_mismatch_count > 0
        || diagnostics.invalid_cut_vertex_degree_count > 0
    {
        return Err(AppError::validation(format!(
            "Surface trim output is invalid: {} duplicate positions, {} non-manifold edges, {} orientation mismatches, {} invalid cut vertices.",
            diagnostics.duplicate_position_count,
            diagnostics.non_manifold_edge_count,
            diagnostics.orientation_mismatch_count,
            diagnostics.invalid_cut_vertex_degree_count,
        )));
    }
    if matches!(cap_mode, SurfaceTrimCapMode::Open) {
        if diagnostics.boundary_edge_count == 0 {
            return Err(AppError::validation(
                "Surface trim Open output unexpectedly has no boundary.",
            ));
        }
    } else if diagnostics.boundary_edge_count > 0 {
        return Err(AppError::validation(format!(
            "Surface trim capped output is not watertight: {} boundary edges remain.",
            diagnostics.boundary_edge_count
        )));
    }

    Ok(SurfaceTrimRuntimeOutput {
        vertices: composed.vertices,
        triangles: composed.triangles,
        cut_edges: composed.cut_edges,
        diagnostics,
        cap_reports,
    })
}

fn materialize_anchor(
    mesh: &IndexedMeshAsset,
    anchor: &CanonicalSurfaceTrimAnchor,
) -> AppResult<CaptureSurfaceAnchor> {
    let triangle = mesh
        .triangles()
        .get(anchor.triangle_index as usize)
        .ok_or_else(|| AppError::validation("Surface trim anchor triangle is out of bounds."))?;
    if anchor
        .barycentric
        .iter()
        .any(|value| !value.is_finite() || *value < -RUNTIME_EPSILON)
        || (anchor.barycentric.iter().sum::<f64>() - 1.0).abs() > 1.0e-6
    {
        return Err(AppError::validation(
            "Surface trim anchor barycentric weights are invalid.",
        ));
    }
    let points = [
        mesh.vertices()[triangle[0] as usize],
        mesh.vertices()[triangle[1] as usize],
        mesh.vertices()[triangle[2] as usize],
    ];
    let source_position = [
        points[0][0] * anchor.barycentric[0]
            + points[1][0] * anchor.barycentric[1]
            + points[2][0] * anchor.barycentric[2],
        points[0][1] * anchor.barycentric[0]
            + points[1][1] * anchor.barycentric[1]
            + points[2][1] * anchor.barycentric[2],
        points[0][2] * anchor.barycentric[0]
            + points[1][2] * anchor.barycentric[1]
            + points[2][2] * anchor.barycentric[2],
    ];
    let source_normal = normalize(cross(sub(points[1], points[0]), sub(points[2], points[0])))?;
    Ok(CaptureSurfaceAnchor {
        source_mesh_content_digest: mesh.content_digest().to_string(),
        triangle_index: anchor.triangle_index,
        barycentric: anchor.barycentric,
        source_position,
        source_normal,
    })
}

fn collect_triangle_segments(
    loop_segments: &[crate::surface_trim_external_shapes::SurfaceTrimLoopSegmentPreview],
) -> AppResult<(
    BTreeMap<u64, Vec<([f64; 3], [f64; 3])>>,
    BTreeMap<u64, BTreeSet<(u32, u32)>>,
)> {
    let mut segments = BTreeMap::<u64, Vec<([f64; 3], [f64; 3])>>::new();
    let mut crossed = BTreeMap::<u64, BTreeSet<(u32, u32)>>::new();
    for loop_segment in loop_segments {
        if loop_segment.continuous_polyline.len() != loop_segment.triangle_path.len() + 1 {
            return Err(AppError::validation(
                "Surface trim continuous path does not match its triangle corridor.",
            ));
        }
        for (index, triangle_index) in loop_segment.triangle_path.iter().copied().enumerate() {
            let start = &loop_segment.continuous_polyline[index];
            let end = &loop_segment.continuous_polyline[index + 1];
            segments
                .entry(triangle_index)
                .or_default()
                .push((start.source_position, end.source_position));
            for point in [start, end] {
                if let Some(edge) = point.shared_edge {
                    crossed
                        .entry(triangle_index)
                        .or_default()
                        .insert(ordered_edge(edge[0] as u32, edge[1] as u32));
                }
            }
        }
    }
    Ok((segments, crossed))
}

fn join_triangle_segments(segments: &[([f64; 3], [f64; 3])]) -> AppResult<Vec<[f64; 3]>> {
    let mut points = Vec::<[f64; 3]>::new();
    let mut edges = Vec::<(usize, usize)>::new();
    for (start, end) in segments {
        let start_index = insert_point(&mut points, *start);
        let end_index = insert_point(&mut points, *end);
        if start_index == end_index {
            return Err(AppError::validation(
                "Surface trim triangle contains a zero-length cut segment.",
            ));
        }
        edges.push((start_index, end_index));
    }
    let mut adjacency = vec![BTreeSet::<usize>::new(); points.len()];
    for (left, right) in edges {
        adjacency[left].insert(right);
        adjacency[right].insert(left);
    }
    let endpoints = adjacency
        .iter()
        .enumerate()
        .filter(|(_, neighbors)| neighbors.len() == 1)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if endpoints.len() != 2 || adjacency.iter().any(|neighbors| neighbors.len() > 2) {
        return Err(AppError::validation(
            "Surface trim segments inside one triangle do not form one simple path.",
        ));
    }
    let mut ordered = Vec::with_capacity(points.len());
    let mut previous = None;
    let mut current = endpoints[0].min(endpoints[1]);
    loop {
        ordered.push(points[current]);
        let next = adjacency[current]
            .iter()
            .copied()
            .find(|neighbor| Some(*neighbor) != previous);
        let Some(next) = next else { break };
        previous = Some(current);
        current = next;
    }
    if ordered.len() != points.len() {
        return Err(AppError::validation(
            "Surface trim triangle path is disconnected.",
        ));
    }
    Ok(ordered)
}

fn local_keep_seed(
    vertices: &[[f64; 3]],
    triangles: &[[u32; 3]],
    triangle_index: u64,
    crossed_edges: Option<&BTreeSet<(u32, u32)>>,
    triangle_is_retained: bool,
    edge_incidents: &BTreeMap<(u32, u32), Vec<u64>>,
    retained: &BTreeSet<u64>,
) -> AppResult<[f64; 3]> {
    let triangle = triangles
        .get(triangle_index as usize)
        .ok_or_else(|| AppError::validation("Surface trim cut triangle is out of bounds."))?;
    let crossed = crossed_edges.cloned().unwrap_or_default();
    let edges = [
        (triangle[0], triangle[1], triangle[2]),
        (triangle[1], triangle[2], triangle[0]),
        (triangle[2], triangle[0], triangle[1]),
    ];
    let remaining = edges.iter().find(|(left, right, _)| {
        let edge = ordered_edge(*left, *right);
        !crossed.contains(&edge)
            && edge_incidents
                .get(&edge)
                .into_iter()
                .flatten()
                .any(|neighbor| *neighbor == triangle_index || retained.contains(neighbor))
    });
    let (left, right, opposite) = remaining.copied().unwrap_or(edges[0]);
    let left = vertices[left as usize];
    let right = vertices[right as usize];
    let opposite = vertices[opposite as usize];
    let weights = if triangle_is_retained {
        (0.45, 0.45, 0.10)
    } else {
        (0.10, 0.10, 0.80)
    };
    Ok([
        left[0] * weights.0 + right[0] * weights.1 + opposite[0] * weights.2,
        left[1] * weights.0 + right[1] * weights.1 + opposite[1] * weights.2,
        left[2] * weights.0 + right[2] * weights.1 + opposite[2] * weights.2,
    ])
}

fn build_edge_incidents(triangles: &[[u32; 3]]) -> BTreeMap<(u32, u32), Vec<u64>> {
    let mut incidents = BTreeMap::<(u32, u32), Vec<u64>>::new();
    for (triangle_index, triangle) in triangles.iter().enumerate() {
        for (left, right) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            incidents
                .entry(ordered_edge(left, right))
                .or_default()
                .push(triangle_index as u64);
        }
    }
    incidents
}

fn insert_point(points: &mut Vec<[f64; 3]>, point: [f64; 3]) -> usize {
    if let Some(index) = points
        .iter()
        .position(|existing| distance_squared(*existing, point) <= RUNTIME_EPSILON.powi(2))
    {
        index
    } else {
        points.push(point);
        points.len() - 1
    }
}

fn ordered_edge(left: u32, right: u32) -> (u32, u32) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn distance_squared(left: [f64; 3], right: [f64; 3]) -> f64 {
    let delta = sub(left, right);
    dot(delta, delta)
}

fn normalize(value: [f64; 3]) -> AppResult<[f64; 3]> {
    let length = dot(value, value).sqrt();
    if !length.is_finite() || length <= RUNTIME_EPSILON {
        return Err(AppError::validation(
            "Surface trim anchor triangle is degenerate.",
        ));
    }
    Ok([value[0] / length, value[1] / length, value[2] / length])
}

fn sub(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube_fixture() -> (std::path::PathBuf, IndexedMeshAsset) {
        let root = std::env::temp_dir().join(format!(
            "ecky-surface-trim-runtime-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos(),
        ));
        std::fs::create_dir_all(&root).expect("fixture directory");
        let path = root.join("cube.stl");
        let vertices = [
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ];
        let triangles = [
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ];
        let mut text = String::from("solid cube\n");
        for triangle in triangles {
            let [a, b, c] = triangle.map(|index| vertices[index]);
            text.push_str(&format!(
                "facet normal 0 0 0\n  outer loop\n    vertex {} {} {}\n    vertex {} {} {}\n    vertex {} {} {}\n  endloop\nendfacet\n",
                a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2],
            ));
        }
        text.push_str("endsolid cube\n");
        std::fs::write(&path, text).expect("fixture STL");
        let mesh =
            IndexedMeshAsset::from_stl(MeshAssetSource::Imported, &path).expect("indexed cube");
        (path, mesh)
    }

    fn horizontal_cube_loop() -> Vec<CanonicalSurfaceTrimAnchor> {
        vec![
            CanonicalSurfaceTrimAnchor {
                triangle_index: 4,
                barycentric: [0.25, 0.25, 0.5],
            },
            CanonicalSurfaceTrimAnchor {
                triangle_index: 7,
                barycentric: [0.5, 0.25, 0.25],
            },
            CanonicalSurfaceTrimAnchor {
                triangle_index: 6,
                barycentric: [0.25, 0.25, 0.5],
            },
            CanonicalSurfaceTrimAnchor {
                triangle_index: 9,
                barycentric: [0.5, 0.25, 0.25],
            },
            CanonicalSurfaceTrimAnchor {
                triangle_index: 8,
                barycentric: [0.25, 0.25, 0.5],
            },
            CanonicalSurfaceTrimAnchor {
                triangle_index: 11,
                barycentric: [0.5, 0.25, 0.25],
            },
            CanonicalSurfaceTrimAnchor {
                triangle_index: 10,
                barycentric: [0.25, 0.25, 0.5],
            },
            CanonicalSurfaceTrimAnchor {
                triangle_index: 5,
                barycentric: [0.5, 0.25, 0.25],
            },
        ]
    }

    #[test]
    fn flat_surface_trim_cuts_triangle_interiors_and_emits_watertight_half_cube() {
        let (path, mesh) = cube_fixture();
        let output = execute_surface_trim(
            &path,
            mesh.content_digest(),
            &horizontal_cube_loop(),
            &CanonicalSurfaceTrimAnchor {
                triangle_index: 2,
                barycentric: [1.0 / 3.0; 3],
            },
            SurfaceTrimPathMode::Shortest,
            SurfaceTrimCapMode::Flat,
        )
        .expect("flat surface trim");

        assert_eq!(output.diagnostics.boundary_edge_count, 0);
        assert_eq!(output.diagnostics.non_manifold_edge_count, 0);
        assert_eq!(output.diagnostics.orientation_mismatch_count, 0);
        assert!(output
            .vertices
            .iter()
            .any(|point| point[2].abs() <= RUNTIME_EPSILON));
        assert!(output.cap_reports.iter().any(|report| {
            report.mode == SurfaceTrimCapMode::Flat && report.added_triangle_count > 0
        }));

        std::fs::remove_dir_all(path.parent().expect("fixture parent")).expect("fixture cleanup");
    }

    #[test]
    fn open_surface_trim_keeps_explicit_boundary_for_solidify_guard() {
        let (path, mesh) = cube_fixture();
        let output = execute_surface_trim(
            &path,
            mesh.content_digest(),
            &horizontal_cube_loop(),
            &CanonicalSurfaceTrimAnchor {
                triangle_index: 2,
                barycentric: [1.0 / 3.0; 3],
            },
            SurfaceTrimPathMode::Shortest,
            SurfaceTrimCapMode::Open,
        )
        .expect("open surface trim");

        assert!(output.diagnostics.boundary_edge_count > 0);
        assert_eq!(output.cap_reports.len(), 1);
        assert!(output.cap_reports[0].explicitly_open);

        std::fs::remove_dir_all(path.parent().expect("fixture parent")).expect("fixture cleanup");
    }
}
