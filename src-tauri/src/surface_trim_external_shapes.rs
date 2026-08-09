use crate::capture_guidance::validate_surface_anchor_from_stl;
use crate::contracts::{AppError, AppResult, CaptureSurfaceAnchor};
use crate::ecky_ir::mesh_asset::IndexedMeshAsset;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

pub const SURFACE_TRIM_SCHEMA_VERSION: u32 = 1;
pub const SURFACE_TRIM_PATH_MODES: &[&str] = &["shortest", "feature"];
const ANCHOR_POSITION_TOLERANCE: f64 = 1.0e-6;
const VERTEX_EPSILON: f64 = 1e-12;
const COST_EPSILON: f64 = 1e-12;
const EDGE_LENGTH_WEIGHT: f64 = 1.0;
const DIRECTION_DEVIATION_WEIGHT: f64 = 0.05;
const EDGE_FLATNESS_WEIGHT: f64 = 0.25;
const CREASE_REWARD_WEIGHT: f64 = 0.25;
const MIN_RETAINED_REGION_TRIANGLES: usize = 1;
const MIN_RETAINED_REGION_AREA: f64 = 1.0e-12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SurfaceTrimPathMode {
    Shortest,
    Feature,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceTrimPathSegment {
    pub from_triangle: u64,
    pub to_triangle: u64,
    pub shared_edge: [u64; 2],
    pub cost: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceTrimBoundaryPoint {
    pub source_position: [f64; 3],
    pub shared_edge: Option<[u64; 2]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceTrimPathDiagnostics {
    pub source_mesh_content_digest: String,
    pub path_mode: SurfaceTrimPathMode,
    pub schema_version: u32,
    pub triangles: u64,
    pub connected_components: u64,
    pub boundary_edges: u64,
    pub non_manifold_edges: u64,
    pub anchor_start_triangle: u64,
    pub anchor_end_triangle: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceTrimPathResult {
    pub source_mesh_content_digest: String,
    pub source_mesh_triangles: u64,
    pub path_mode: SurfaceTrimPathMode,
    pub start_triangle_index: u64,
    pub end_triangle_index: u64,
    pub total_cost: f64,
    pub triangle_corridor: Vec<u64>,
    pub edge_segments: Vec<SurfaceTrimPathSegment>,
    pub continuous_polyline: Vec<SurfaceTrimBoundaryPoint>,
    pub diagnostics: SurfaceTrimPathDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceTrimLoopSegmentPreview {
    pub segment_index: u64,
    pub from_triangle_index: u64,
    pub to_triangle_index: u64,
    pub triangle_path: Vec<u64>,
    pub edge_segments: Vec<SurfaceTrimPathSegment>,
    pub continuous_polyline: Vec<SurfaceTrimBoundaryPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceTrimRegionPreview {
    pub source_mesh_content_digest: String,
    pub path_mode: SurfaceTrimPathMode,
    pub loop_segment_count: u64,
    pub loop_triangle_path: Vec<u64>,
    pub keep_seed_triangle_index: u64,
    pub retained_triangle_indices: Vec<u64>,
    pub retained_triangle_count: u64,
    pub excluded_triangle_count: u64,
    pub loop_segments: Vec<SurfaceTrimLoopSegmentPreview>,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SurfaceTrimPathPreviewRequest {
    pub schema_version: u32,
    pub thread_id: String,
    #[serde(default)]
    pub target_message_id: Option<String>,
    pub node_id: u64,
    pub expected_source_digest: String,
    pub expected_mesh_content_digest: String,
    pub from_anchor: CaptureSurfaceAnchor,
    pub to_anchor: CaptureSurfaceAnchor,
    pub path_mode: SurfaceTrimPathMode,
    pub preview_id: u64,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceTrimPathPreviewResponse {
    pub preview_id: u64,
    pub path: SurfaceTrimPathResult,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SurfaceTrimLoopPreviewRequest {
    pub schema_version: u32,
    pub thread_id: String,
    #[serde(default)]
    pub target_message_id: Option<String>,
    pub node_id: u64,
    pub expected_source_digest: String,
    pub expected_mesh_content_digest: String,
    pub loop_anchors: Vec<CaptureSurfaceAnchor>,
    pub path_mode: SurfaceTrimPathMode,
    pub preview_id: u64,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceTrimLoopPreviewResponse {
    pub preview_id: u64,
    pub source_mesh_content_digest: String,
    pub path_mode: SurfaceTrimPathMode,
    pub loop_triangle_path: Vec<u64>,
    pub loop_segments: Vec<SurfaceTrimLoopSegmentPreview>,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SurfaceTrimRegionPreviewRequest {
    pub schema_version: u32,
    pub thread_id: String,
    #[serde(default)]
    pub target_message_id: Option<String>,
    pub node_id: u64,
    pub expected_source_digest: String,
    pub expected_mesh_content_digest: String,
    pub loop_anchors: Vec<CaptureSurfaceAnchor>,
    pub keep_seed: CaptureSurfaceAnchor,
    pub path_mode: SurfaceTrimPathMode,
    pub cap_mode: crate::surface_trim_cap::SurfaceTrimCapMode,
    pub preview_id: u64,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceTrimRegionPreviewResponse {
    pub preview_id: u64,
    pub preview: SurfaceTrimRegionPreview,
    pub topology: crate::surface_trim_diagnostics::SurfaceTrimMeshDiagnostics,
    pub cap_reports: Vec<crate::surface_trim_cap::SurfaceTrimCapReport>,
    pub cap_preview: Option<SurfaceTrimCapPreview>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceTrimCapPreview {
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

#[derive(Clone)]
struct SurfaceMeshGraph {
    source_mesh_content_digest: String,
    vertices: Vec<[f64; 3]>,
    triangles: Vec<[u32; 3]>,
    triangle_centroids: Vec<[f64; 3]>,
    adjacency: Vec<Vec<MeshAdjacency>>,
    components: Vec<usize>,
    edge_incident_counts: BTreeMap<(u32, u32), usize>,
    boundary_edges: u64,
    non_manifold_edges: u64,
}

#[derive(Clone)]
struct MeshAdjacency {
    neighbor: usize,
    shared_edge: [u32; 2],
    edge_index: u64,
    edge_length: f64,
    dihedral: f64,
}

#[derive(Clone)]
struct SurfaceMeshGraphCacheEntry {
    graph: Arc<SurfaceMeshGraph>,
}

static SURFACE_TRIM_GRAPH_CACHE: OnceLock<Mutex<HashMap<String, SurfaceMeshGraphCacheEntry>>> =
    OnceLock::new();

fn cache_mutex() -> &'static Mutex<HashMap<String, SurfaceMeshGraphCacheEntry>> {
    SURFACE_TRIM_GRAPH_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn clear_surface_trim_graph_cache() {
    if let Ok(mut cache) = cache_mutex().lock() {
        cache.clear();
    }
}

pub fn require_surface_trim_schema_version(schema_version: u32) -> AppResult<()> {
    if schema_version != SURFACE_TRIM_SCHEMA_VERSION {
        return Err(AppError::validation(format!(
            "Unsupported surface trim schema version {schema_version}; expected {SURFACE_TRIM_SCHEMA_VERSION}."
        )));
    }
    Ok(())
}

pub fn surface_trim_path(
    source_stl_path: &Path,
    start_anchor: &CaptureSurfaceAnchor,
    end_anchor: &CaptureSurfaceAnchor,
    path_mode: SurfaceTrimPathMode,
) -> AppResult<SurfaceTrimPathResult> {
    let validated_start =
        validate_surface_anchor_from_stl(source_stl_path, start_anchor, ANCHOR_POSITION_TOLERANCE)?;
    let validated_end =
        validate_surface_anchor_from_stl(source_stl_path, end_anchor, ANCHOR_POSITION_TOLERANCE)?;

    if start_anchor.source_mesh_content_digest != end_anchor.source_mesh_content_digest {
        return Err(AppError::conflict(
            "Source trim anchors reference different source mesh digests.",
        ));
    }

    let start_triangle = usize::try_from(start_anchor.triangle_index)
        .map_err(|_| AppError::validation("Capture anchor triangle index is out of bounds."))?;
    let end_triangle = usize::try_from(end_anchor.triangle_index)
        .map_err(|_| AppError::validation("Capture anchor triangle index is out of bounds."))?;

    let cached = load_or_build_graph(source_stl_path, &start_anchor.source_mesh_content_digest)?;
    if start_triangle >= cached.graph.adjacency.len()
        || end_triangle >= cached.graph.adjacency.len()
    {
        return Err(AppError::validation(
            "Capture anchor triangle index is out of bounds.",
        ));
    }
    if cached.graph.components[start_triangle] != cached.graph.components[end_triangle] {
        return Err(AppError::validation(
            "Source trim anchors are not on the same connected mesh component.",
        ));
    }

    let global_direction = sub(
        validated_end.source_position,
        validated_start.source_position,
    );
    let (triangle_corridor, edge_segments, total_cost) = shortest_triangle_corridor(
        &cached.graph,
        start_triangle,
        end_triangle,
        &global_direction,
        path_mode,
    )?;

    if triangle_corridor.is_empty() {
        return Err(AppError::internal("Failed to compute any path triangles."));
    }

    let source_mesh_content_digest = cached.graph.source_mesh_content_digest.clone();
    let connected_components = cached.graph.component_count();
    let continuous_polyline = build_continuous_polyline(
        &cached.graph,
        validated_start.source_position,
        validated_end.source_position,
        &edge_segments,
    )?;

    Ok(SurfaceTrimPathResult {
        source_mesh_content_digest,
        source_mesh_triangles: cached.graph.adjacency.len() as u64,
        path_mode,
        start_triangle_index: start_triangle as u64,
        end_triangle_index: end_triangle as u64,
        total_cost,
        triangle_corridor,
        edge_segments,
        continuous_polyline,
        diagnostics: SurfaceTrimPathDiagnostics {
            source_mesh_content_digest: start_anchor.source_mesh_content_digest.clone(),
            path_mode,
            schema_version: SURFACE_TRIM_SCHEMA_VERSION,
            triangles: cached.graph.adjacency.len() as u64,
            connected_components,
            boundary_edges: cached.graph.boundary_edges,
            non_manifold_edges: cached.graph.non_manifold_edges,
            anchor_start_triangle: start_triangle as u64,
            anchor_end_triangle: end_triangle as u64,
        },
    })
}

pub fn preview_surface_trim_loop(
    source_stl_path: &Path,
    loop_anchors: &[CaptureSurfaceAnchor],
    path_mode: SurfaceTrimPathMode,
    preview_id: u64,
) -> AppResult<SurfaceTrimLoopPreviewResponse> {
    if loop_anchors.len() < 3 {
        return Err(AppError::validation(
            "Surface trim loop needs at least three anchors.",
        ));
    }
    let mut validated = Vec::with_capacity(loop_anchors.len());
    let mut digest = None::<String>;
    for (index, anchor) in loop_anchors.iter().enumerate() {
        validate_surface_anchor_from_stl(source_stl_path, anchor, ANCHOR_POSITION_TOLERANCE)?;
        if let Some(expected) = &digest {
            if expected != &anchor.source_mesh_content_digest {
                return Err(AppError::conflict(
                    "Loop anchors reference different source mesh digests.",
                ));
            }
        } else {
            digest = Some(anchor.source_mesh_content_digest.clone());
        }
        if validated
            .iter()
            .any(|existing: &CaptureSurfaceAnchor| anchors_match(existing, &anchor))
        {
            return Err(AppError::validation(format!(
                "Loop anchor {} is duplicated.",
                index + 1
            )));
        }
        validated.push(anchor.clone());
    }
    let source_mesh_content_digest = digest
        .ok_or_else(|| AppError::validation("Surface trim loop needs at least three anchors."))?;
    let cached = load_or_build_graph(source_stl_path, &source_mesh_content_digest)?;
    let component = cached.graph.components[validated[0].triangle_index as usize];
    if validated
        .iter()
        .any(|anchor| cached.graph.components[anchor.triangle_index as usize] != component)
    {
        return Err(AppError::validation(
            "Surface trim loop anchors are not on one connected mesh component.",
        ));
    }
    let closed = build_closed_loop_preview(source_stl_path, &validated, path_mode, &cached.graph)?;
    let probe_triangle = cached
        .graph
        .adjacency
        .iter()
        .enumerate()
        .find_map(|(triangle, edges)| {
            edges
                .iter()
                .any(|edge| {
                    closed
                        .blocked_edges
                        .contains(&normalized_edge_key_u32(edge.shared_edge))
                })
                .then_some(triangle)
        })
        .ok_or_else(|| {
            AppError::validation(
                "Surface trim loop does not cross a mesh edge and cannot partition the surface.",
            )
        })?;
    let (probe_region, component_size) =
        flood_fill_retained_region(&cached.graph, probe_triangle, &closed.blocked_edges)?;
    if probe_region.is_empty() || probe_region.len() == component_size {
        return Err(AppError::validation(
            "Surface trim loop does not partition the selected surface.",
        ));
    }
    Ok(SurfaceTrimLoopPreviewResponse {
        preview_id,
        source_mesh_content_digest,
        path_mode,
        loop_triangle_path: closed.loop_triangle_path,
        loop_segments: closed.segments,
    })
}

pub fn preview_surface_trim_region(
    source_stl_path: &Path,
    loop_anchors: &[CaptureSurfaceAnchor],
    keep_seed: &CaptureSurfaceAnchor,
    path_mode: SurfaceTrimPathMode,
) -> AppResult<SurfaceTrimRegionPreview> {
    if loop_anchors.len() < 3 {
        return Err(AppError::validation(
            "Surface trim loop needs at least three anchors.",
        ));
    }

    let mut validated_loop_anchors = Vec::with_capacity(loop_anchors.len());
    let mut digest = None::<String>;
    for (index, anchor) in loop_anchors.iter().enumerate() {
        validate_surface_anchor_from_stl(source_stl_path, anchor, ANCHOR_POSITION_TOLERANCE)?;
        if let Some(existing) = &digest {
            if existing != &anchor.source_mesh_content_digest {
                return Err(AppError::conflict(
                    "Loop anchors reference different source mesh digests.",
                ));
            }
        } else {
            digest = Some(anchor.source_mesh_content_digest.clone());
        }
        if validated_loop_anchors
            .iter()
            .any(|existing: &CaptureSurfaceAnchor| anchors_match(existing, anchor))
        {
            return Err(AppError::validation(format!(
                "Loop anchor {} is duplicated.",
                index + 1
            )));
        }
        validated_loop_anchors.push(anchor.clone());
    }

    if keep_seed.source_mesh_content_digest != digest.as_deref().unwrap_or_default() {
        return Err(AppError::conflict(
            "Keep seed references a different source mesh digest.",
        ));
    }
    let validated_seed =
        validate_surface_anchor_from_stl(source_stl_path, keep_seed, ANCHOR_POSITION_TOLERANCE)?;

    let source_mesh_content_digest = digest
        .ok_or_else(|| AppError::validation("Surface trim loop needs at least three anchors."))?;
    let cached = load_or_build_graph(source_stl_path, &source_mesh_content_digest)?;
    let seed_triangle = usize::try_from(keep_seed.triangle_index)
        .map_err(|_| AppError::validation("Keep seed triangle index is out of bounds."))?;
    if seed_triangle >= cached.graph.adjacency.len() {
        return Err(AppError::validation(
            "Keep seed triangle index is out of bounds.",
        ));
    }

    let loop_triangle = usize::try_from(validated_loop_anchors[0].triangle_index)
        .map_err(|_| AppError::validation("Loop triangle index is out of bounds."))?;
    if loop_triangle >= cached.graph.adjacency.len() {
        return Err(AppError::validation(
            "Loop triangle index is out of bounds.",
        ));
    }
    let loop_component = cached.graph.components[loop_triangle];
    if cached.graph.components[seed_triangle] != loop_component {
        return Err(AppError::validation(
            "Keep seed is not on the same connected surface component as the loop.",
        ));
    }

    let loop_preview =
        build_closed_loop_preview(source_stl_path, loop_anchors, path_mode, &cached.graph)?;
    if loop_preview.segments.iter().any(|segment| {
        segment.continuous_polyline.windows(2).any(|pair| {
            point_segment_distance(
                validated_seed.source_position,
                pair[0].source_position,
                pair[1].source_position,
            ) <= ANCHOR_POSITION_TOLERANCE
        })
    }) {
        return Err(AppError::validation(
            "Keep seed lies on the surface trim boundary.",
        ));
    }
    let SurfaceTrimClosedLoopPreview {
        loop_triangle_path,
        blocked_edges,
        segments,
    } = loop_preview;

    let (retained_triangles, component_size) =
        flood_fill_retained_region(&cached.graph, seed_triangle, &blocked_edges)?;
    let retained_count = retained_triangles.len();
    if retained_count < MIN_RETAINED_REGION_TRIANGLES || retained_count == component_size {
        return Err(AppError::validation(
            "Loop does not partition the selected surface.",
        ));
    }
    let retained_area = retained_triangles
        .iter()
        .try_fold(0.0, |area, triangle_index| {
            let triangle = cached.graph.triangles.get(*triangle_index).ok_or_else(|| {
                AppError::validation("Surface trim retained region references an invalid triangle.")
            })?;
            let points = triangle.map(|index| cached.graph.vertices[index as usize]);
            Ok::<_, AppError>(
                area + 0.5 * norm(cross(sub(points[1], points[0]), sub(points[2], points[0]))),
            )
        })?;
    if !retained_area.is_finite() || retained_area <= MIN_RETAINED_REGION_AREA {
        return Err(AppError::validation(format!(
            "Selected surface trim region is below minimum area: {:.12}.",
            retained_area
        )));
    }

    let retained_triangle_indices = retained_triangles
        .into_iter()
        .map(|triangle| triangle as u64)
        .collect::<Vec<_>>();

    Ok(SurfaceTrimRegionPreview {
        source_mesh_content_digest,
        path_mode,
        loop_segment_count: segments.len() as u64,
        loop_triangle_path,
        keep_seed_triangle_index: keep_seed.triangle_index,
        retained_triangle_indices: retained_triangle_indices.clone(),
        retained_triangle_count: retained_triangle_indices.len() as u64,
        excluded_triangle_count: cached.graph.adjacency.len() as u64
            - retained_triangle_indices.len() as u64,
        loop_segments: segments,
    })
}

fn point_segment_distance(point: [f64; 3], start: [f64; 3], end: [f64; 3]) -> f64 {
    let segment = sub(end, start);
    let length_squared = dot(segment, segment);
    if length_squared <= VERTEX_EPSILON {
        return distance(point, start);
    }
    let parameter = (dot(sub(point, start), segment) / length_squared).clamp(0.0, 1.0);
    distance(
        point,
        [
            start[0] + segment[0] * parameter,
            start[1] + segment[1] * parameter,
            start[2] + segment[2] * parameter,
        ],
    )
}

struct SurfaceTrimClosedLoopPreview {
    loop_triangle_path: Vec<u64>,
    blocked_edges: HashSet<(u32, u32)>,
    segments: Vec<SurfaceTrimLoopSegmentPreview>,
}

fn build_closed_loop_preview(
    source_stl_path: &Path,
    loop_anchors: &[CaptureSurfaceAnchor],
    path_mode: SurfaceTrimPathMode,
    graph: &SurfaceMeshGraph,
) -> AppResult<SurfaceTrimClosedLoopPreview> {
    let mut loop_triangle_path = Vec::new();
    let mut blocked_edges = HashSet::new();
    let mut blocked_edge_owners = HashMap::<(u32, u32), u64>::new();
    let mut segments = Vec::with_capacity(loop_anchors.len());
    for segment_index in 0..loop_anchors.len() {
        let from = &loop_anchors[segment_index];
        let to = &loop_anchors[(segment_index + 1) % loop_anchors.len()];
        let path = surface_trim_path(source_stl_path, from, to, path_mode).map_err(|error| {
            AppError::validation(format!(
                "Loop segment {} failed: {error}",
                segment_index + 1
            ))
        })?;

        if distance(from.source_position, to.source_position) <= VERTEX_EPSILON {
            return Err(AppError::validation(format!(
                "Loop segment {} has zero length.",
                segment_index + 1
            )));
        }

        for (path_index, triangle) in path.triangle_corridor.iter().enumerate() {
            let is_join = path_index == 0 && loop_triangle_path.last() == Some(triangle);
            if is_join {
                continue;
            }

            loop_triangle_path.push(*triangle);
        }

        for edge in &path.edge_segments {
            let edge_key = normalized_edge_key(edge.shared_edge);
            if let Some(incident_count) = graph.edge_incident_counts.get(&edge_key) {
                if *incident_count > 2 {
                    return Err(AppError::validation(format!(
                        "Loop segment {} crosses a non-manifold boundary edge.",
                        segment_index + 1
                    )));
                }
            }
            if let Some(previous_segment) =
                blocked_edge_owners.insert(edge_key, segment_index as u64)
            {
                return Err(AppError::validation(format!(
                    "Loop segment {} overlaps segment {} at source edge [{}, {}].",
                    segment_index + 1,
                    previous_segment + 1,
                    edge_key.0,
                    edge_key.1
                )));
            }
            blocked_edges.insert(edge_key);
        }

        segments.push(SurfaceTrimLoopSegmentPreview {
            segment_index: segment_index as u64,
            from_triangle_index: from.triangle_index,
            to_triangle_index: to.triangle_index,
            triangle_path: path.triangle_corridor,
            edge_segments: path.edge_segments,
            continuous_polyline: path.continuous_polyline,
        });
    }

    reject_geometric_loop_intersections(graph, &segments)?;

    Ok(SurfaceTrimClosedLoopPreview {
        loop_triangle_path,
        blocked_edges,
        segments,
    })
}

fn reject_geometric_loop_intersections(
    graph: &SurfaceMeshGraph,
    segments: &[SurfaceTrimLoopSegmentPreview],
) -> AppResult<()> {
    let mut by_triangle = BTreeMap::<u64, Vec<(usize, [f64; 3], [f64; 3])>>::new();
    for segment in segments {
        if segment.continuous_polyline.len() != segment.triangle_path.len() + 1 {
            return Err(AppError::validation(format!(
                "Loop segment {} has an invalid continuous path.",
                segment.segment_index + 1
            )));
        }
        for (index, triangle_index) in segment.triangle_path.iter().copied().enumerate() {
            by_triangle.entry(triangle_index).or_default().push((
                segment.segment_index as usize,
                segment.continuous_polyline[index].source_position,
                segment.continuous_polyline[index + 1].source_position,
            ));
        }
    }

    for (triangle_index, local_segments) in by_triangle {
        let triangle = graph
            .triangles
            .get(triangle_index as usize)
            .ok_or_else(|| {
                AppError::validation("Surface trim loop references an invalid triangle.")
            })?;
        let points = triangle.map(|index| graph.vertices[index as usize]);
        let projection_axis =
            dominant_axis(cross(sub(points[1], points[0]), sub(points[2], points[0])));
        for left_index in 0..local_segments.len() {
            for right_index in (left_index + 1)..local_segments.len() {
                let (left_owner, left_start, left_end) = local_segments[left_index];
                let (right_owner, right_start, right_end) = local_segments[right_index];
                if left_owner == right_owner {
                    continue;
                }
                let adjacent = loop_segments_are_adjacent(left_owner, right_owner, segments.len());
                let shared_endpoint = [left_start, left_end].into_iter().any(|left| {
                    [right_start, right_end]
                        .into_iter()
                        .any(|right| distance(left, right) <= ANCHOR_POSITION_TOLERANCE)
                });
                if segments_intersect_2d(
                    project_2d(left_start, projection_axis),
                    project_2d(left_end, projection_axis),
                    project_2d(right_start, projection_axis),
                    project_2d(right_end, projection_axis),
                ) && !(adjacent && shared_endpoint)
                {
                    return Err(AppError::validation(format!(
                        "Loop segments {} and {} self-intersect in source triangle {}.",
                        left_owner + 1,
                        right_owner + 1,
                        triangle_index
                    )));
                }
            }
        }
    }
    Ok(())
}

fn loop_segments_are_adjacent(left: usize, right: usize, count: usize) -> bool {
    left.abs_diff(right) == 1
        || (left == 0 && right + 1 == count)
        || (right == 0 && left + 1 == count)
}

fn dominant_axis(normal: [f64; 3]) -> usize {
    (0..3)
        .max_by(|left, right| normal[*left].abs().total_cmp(&normal[*right].abs()))
        .unwrap_or(2)
}

fn project_2d(point: [f64; 3], drop_axis: usize) -> [f64; 2] {
    match drop_axis {
        0 => [point[1], point[2]],
        1 => [point[0], point[2]],
        _ => [point[0], point[1]],
    }
}

fn segments_intersect_2d(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    fn orient(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    }
    fn within(a: [f64; 2], b: [f64; 2], point: [f64; 2]) -> bool {
        point[0] >= a[0].min(b[0]) - ANCHOR_POSITION_TOLERANCE
            && point[0] <= a[0].max(b[0]) + ANCHOR_POSITION_TOLERANCE
            && point[1] >= a[1].min(b[1]) - ANCHOR_POSITION_TOLERANCE
            && point[1] <= a[1].max(b[1]) + ANCHOR_POSITION_TOLERANCE
    }
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);
    if o1 * o2 < -ANCHOR_POSITION_TOLERANCE && o3 * o4 < -ANCHOR_POSITION_TOLERANCE {
        return true;
    }
    (o1.abs() <= ANCHOR_POSITION_TOLERANCE && within(a, b, c))
        || (o2.abs() <= ANCHOR_POSITION_TOLERANCE && within(a, b, d))
        || (o3.abs() <= ANCHOR_POSITION_TOLERANCE && within(c, d, a))
        || (o4.abs() <= ANCHOR_POSITION_TOLERANCE && within(c, d, b))
}

fn anchors_match(left: &CaptureSurfaceAnchor, right: &CaptureSurfaceAnchor) -> bool {
    left.source_mesh_content_digest == right.source_mesh_content_digest
        && left.triangle_index == right.triangle_index
        && left
            .barycentric
            .iter()
            .zip(right.barycentric.iter())
            .all(|(left, right)| (left - right).abs() <= VERTEX_EPSILON)
}

fn flood_fill_retained_region(
    graph: &SurfaceMeshGraph,
    seed_triangle: usize,
    blocked_edges: &HashSet<(u32, u32)>,
) -> AppResult<(BTreeSet<usize>, usize)> {
    let component = graph.components[seed_triangle];
    let component_size = graph
        .components
        .iter()
        .filter(|current| **current == component)
        .count();

    let mut retained = BTreeSet::new();
    let mut queue = VecDeque::new();
    retained.insert(seed_triangle);
    queue.push_back(seed_triangle);

    while let Some(current) = queue.pop_front() {
        for adjacency in &graph.adjacency[current] {
            if graph.components[adjacency.neighbor] != component {
                continue;
            }
            if blocked_edges.contains(&normalized_edge_key_u32(adjacency.shared_edge)) {
                continue;
            }
            if retained.insert(adjacency.neighbor) {
                queue.push_back(adjacency.neighbor);
            }
        }
    }

    Ok((retained, component_size))
}

fn normalized_edge_key(edge: [u64; 2]) -> (u32, u32) {
    let left = u32::try_from(edge[0]).unwrap_or(u32::MAX);
    let right = u32::try_from(edge[1]).unwrap_or(u32::MAX);
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn normalized_edge_key_u32(edge: [u32; 2]) -> (u32, u32) {
    if edge[0] <= edge[1] {
        (edge[0], edge[1])
    } else {
        (edge[1], edge[0])
    }
}

fn load_or_build_graph(
    path: &Path,
    source_mesh_content_digest: &str,
) -> AppResult<SurfaceMeshGraphCacheEntry> {
    {
        let cache = cache_mutex()
            .lock()
            .map_err(|_| AppError::internal("Failed to access surface trim graph cache."))?;
        if let Some(cached) = cache.get(source_mesh_content_digest) {
            return Ok(cached.clone());
        }
    }

    let mesh =
        IndexedMeshAsset::from_stl(crate::ecky_ir::mesh_asset::MeshAssetSource::Imported, path)?;
    if mesh.content_digest() != source_mesh_content_digest {
        return Err(AppError::conflict(
            "Capture source anchor mesh digest differs from selected source mesh.",
        ));
    }
    let graph = Arc::new(build_surface_mesh_graph(&mesh)?);
    let entry = SurfaceMeshGraphCacheEntry { graph };
    let mut cache = cache_mutex()
        .lock()
        .map_err(|_| AppError::internal("Failed to access surface trim graph cache."))?;
    cache.insert(source_mesh_content_digest.to_string(), entry.clone());
    Ok(entry)
}

fn build_surface_mesh_graph(mesh: &IndexedMeshAsset) -> AppResult<SurfaceMeshGraph> {
    let triangles = mesh.triangles();
    if triangles.is_empty() {
        return Err(AppError::validation("Source mesh has no triangles."));
    }

    let mut triangle_normals = Vec::with_capacity(triangles.len());
    for triangle in triangles {
        let a = mesh.vertices()[triangle[0] as usize];
        let b = mesh.vertices()[triangle[1] as usize];
        let c = mesh.vertices()[triangle[2] as usize];
        let raw_normal = cross(sub(b, a), sub(c, a));
        let normal = normalize(raw_normal)?;
        triangle_normals.push(normal);
    }

    let mut edge_to_triangles = BTreeMap::<(u32, u32), Vec<usize>>::new();
    for (triangle_index, triangle) in triangles.iter().enumerate() {
        for (left, right) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            let edge = if left <= right {
                (left, right)
            } else {
                (right, left)
            };
            edge_to_triangles
                .entry(edge)
                .or_default()
                .push(triangle_index);
        }
    }

    let mut adjacency = vec![Vec::new(); triangles.len()];
    let mut edge_incident_counts = BTreeMap::new();
    let mut non_manifold_edges = 0u64;
    let mut boundary_edges = 0u64;
    let mut edge_index = 0u64;

    for ((left, right), mut incident) in edge_to_triangles {
        incident.sort_unstable();
        edge_incident_counts.insert((left, right), incident.len());
        if incident.len() == 1 {
            boundary_edges += 1;
            continue;
        }
        if incident.len() > 2 {
            non_manifold_edges += 1;
        }
        let shared_length = distance(
            mesh.vertices()[left as usize],
            mesh.vertices()[right as usize],
        );
        let shared_edge = [left, right];

        for left_index in 0..incident.len() {
            for right_index in (left_index + 1)..incident.len() {
                let first = incident[left_index];
                let second = incident[right_index];
                let mut dot_product =
                    dot(triangle_normals[first], triangle_normals[second]).clamp(-1.0, 1.0);
                dot_product = dot_product.clamp(-1.0, 1.0);
                let dihedral = dot_product.acos();

                adjacency[first].push(MeshAdjacency {
                    neighbor: second,
                    shared_edge,
                    edge_index,
                    edge_length: shared_length,
                    dihedral,
                });
                adjacency[second].push(MeshAdjacency {
                    neighbor: first,
                    shared_edge,
                    edge_index,
                    edge_length: shared_length,
                    dihedral,
                });
            }
        }
        edge_index += 1;
    }

    for edges in &mut adjacency {
        edges.sort_by(|left, right| {
            (
                left.neighbor,
                left.edge_index,
                left.shared_edge[0],
                left.shared_edge[1],
                left.dihedral.to_bits(),
            )
                .cmp(&(
                    right.neighbor,
                    right.edge_index,
                    right.shared_edge[0],
                    right.shared_edge[1],
                    right.dihedral.to_bits(),
                ))
        });
    }

    let components = connected_components(&adjacency);
    let triangle_centroids = triangles
        .iter()
        .map(|triangle| {
            let a = mesh.vertices()[triangle[0] as usize];
            let b = mesh.vertices()[triangle[1] as usize];
            let c = mesh.vertices()[triangle[2] as usize];
            [
                (a[0] + b[0] + c[0]) / 3.0,
                (a[1] + b[1] + c[1]) / 3.0,
                (a[2] + b[2] + c[2]) / 3.0,
            ]
        })
        .collect();

    Ok(SurfaceMeshGraph {
        source_mesh_content_digest: mesh.content_digest().to_string(),
        vertices: mesh.vertices().to_vec(),
        triangles: mesh.triangles().to_vec(),
        triangle_centroids,
        adjacency,
        components,
        edge_incident_counts,
        boundary_edges,
        non_manifold_edges,
    })
}

fn build_continuous_polyline(
    graph: &SurfaceMeshGraph,
    start_position: [f64; 3],
    end_position: [f64; 3],
    edge_segments: &[SurfaceTrimPathSegment],
) -> AppResult<Vec<SurfaceTrimBoundaryPoint>> {
    if !position_is_finite(start_position) || !position_is_finite(end_position) {
        return Err(AppError::validation(
            "Surface trim path contains a non-finite anchor position.",
        ));
    }

    let mut polyline = Vec::with_capacity(edge_segments.len() + 2);
    polyline.push(SurfaceTrimBoundaryPoint {
        source_position: start_position,
        shared_edge: None,
    });

    for segment in edge_segments {
        let left = usize::try_from(segment.shared_edge[0]).map_err(|_| {
            AppError::validation("Surface trim path references an invalid mesh vertex index.")
        })?;
        let right = usize::try_from(segment.shared_edge[1]).map_err(|_| {
            AppError::validation("Surface trim path references an invalid mesh vertex index.")
        })?;
        let left_position = graph.vertices.get(left).copied().ok_or_else(|| {
            AppError::validation("Surface trim path references an invalid mesh vertex index.")
        })?;
        let right_position = graph.vertices.get(right).copied().ok_or_else(|| {
            AppError::validation("Surface trim path references an invalid mesh vertex index.")
        })?;
        let midpoint = scale(
            [
                left_position[0] + right_position[0],
                left_position[1] + right_position[1],
                left_position[2] + right_position[2],
            ],
            0.5,
        );
        if !position_is_finite(midpoint) {
            return Err(AppError::validation(
                "Surface trim path produced a non-finite boundary point.",
            ));
        }
        polyline.push(SurfaceTrimBoundaryPoint {
            source_position: midpoint,
            shared_edge: Some(segment.shared_edge),
        });
    }

    polyline.push(SurfaceTrimBoundaryPoint {
        source_position: end_position,
        shared_edge: None,
    });
    Ok(polyline)
}

fn position_is_finite(position: [f64; 3]) -> bool {
    position.into_iter().all(f64::is_finite)
}

impl SurfaceMeshGraph {
    fn component_count(&self) -> u64 {
        if self.components.is_empty() {
            0
        } else {
            (self.components.iter().copied().max().unwrap_or(0) + 1) as u64
        }
    }
}

fn connected_components(adjacency: &[Vec<MeshAdjacency>]) -> Vec<usize> {
    let mut component_of = vec![usize::MAX; adjacency.len()];
    let mut component = 0usize;
    let mut queue = VecDeque::new();

    for start in 0..adjacency.len() {
        if component_of[start] != usize::MAX {
            continue;
        }
        component_of[start] = component;
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            for neighbor in &adjacency[current] {
                let next = neighbor.neighbor;
                if component_of[next] != usize::MAX {
                    continue;
                }
                component_of[next] = component;
                queue.push_back(next);
            }
        }

        component += 1;
    }

    component_of
}

fn shortest_triangle_corridor(
    graph: &SurfaceMeshGraph,
    start_triangle: usize,
    end_triangle: usize,
    global_direction: &[f64; 3],
    path_mode: SurfaceTrimPathMode,
) -> AppResult<(Vec<u64>, Vec<SurfaceTrimPathSegment>, f64)> {
    if start_triangle == end_triangle {
        return Ok((vec![start_triangle as u64], Vec::new(), 0.0));
    }

    let triangle_count = graph.adjacency.len();
    let mut best_cost = vec![f64::INFINITY; triangle_count];
    let mut best_paths: Vec<Vec<usize>> = vec![Vec::new(); triangle_count];
    let mut best_edge_paths: Vec<Vec<MeshAdjacency>> = vec![Vec::new(); triangle_count];

    best_cost[start_triangle] = 0.0;
    best_paths[start_triangle] = vec![start_triangle];

    let mut open = BinaryHeap::<PathState>::new();
    open.push(PathState {
        estimated_total: heuristic_cost(graph, start_triangle, end_triangle, global_direction),
        cost_so_far: 0.0,
        node: start_triangle,
        path: vec![start_triangle],
        edge_path: Vec::new(),
    });

    while let Some(state) = open.pop() {
        if state.cost_so_far > best_cost[state.node] + COST_EPSILON {
            continue;
        }
        if state.path != best_paths[state.node] {
            continue;
        }

        if state.node == end_triangle {
            let segments = edge_path_segments(
                graph,
                state.path.as_slice(),
                &state.edge_path,
                path_mode,
                global_direction,
            );
            return Ok((
                state.path.iter().map(|value| *value as u64).collect(),
                segments,
                state.cost_so_far,
            ));
        }

        for adj in &graph.adjacency[state.node] {
            let step_cost = transition_cost(graph, state.node, adj, path_mode, global_direction);
            let next_cost = state.cost_so_far + step_cost;
            let mut next_path = state.path.clone();
            next_path.push(adj.neighbor);
            let mut next_edges = state.edge_path.clone();
            next_edges.push(adj.clone());

            let should_update = if next_cost < best_cost[adj.neighbor] - COST_EPSILON {
                true
            } else if (next_cost - best_cost[adj.neighbor]).abs() <= COST_EPSILON {
                better_path(
                    &next_path,
                    &next_edges,
                    &best_paths[adj.neighbor],
                    &best_edge_paths[adj.neighbor],
                )
            } else {
                false
            };

            if !should_update {
                continue;
            }

            best_cost[adj.neighbor] = next_cost;
            best_paths[adj.neighbor] = next_path.clone();
            best_edge_paths[adj.neighbor] = next_edges.clone();

            open.push(PathState {
                estimated_total: next_cost
                    + heuristic_cost(graph, adj.neighbor, end_triangle, global_direction),
                cost_so_far: next_cost,
                node: adj.neighbor,
                path: next_path,
                edge_path: next_edges,
            });
        }
    }

    Err(AppError::validation(
        "Could not find a surface path between selected anchors.",
    ))
}

fn heuristic_cost(
    _graph: &SurfaceMeshGraph,
    from: usize,
    to: usize,
    _global_direction: &[f64; 3],
) -> f64 {
    if from == to {
        0.0
    } else {
        0.0
    }
}

#[derive(Clone)]
struct PathState {
    estimated_total: f64,
    cost_so_far: f64,
    node: usize,
    path: Vec<usize>,
    edge_path: Vec<MeshAdjacency>,
}

impl Eq for PathState {}

impl Ord for PathState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .estimated_total
            .total_cmp(&self.estimated_total)
            .then_with(|| other.cost_so_far.total_cmp(&self.cost_so_far))
            .then_with(|| self.node.cmp(&other.node))
            .then_with(|| self.path.cmp(&other.path))
    }
}

impl PartialOrd for PathState {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PathState {
    fn eq(&self, other: &Self) -> bool {
        self.estimated_total.to_bits() == other.estimated_total.to_bits()
            && self.cost_so_far.to_bits() == other.cost_so_far.to_bits()
            && self.node == other.node
            && self.path == other.path
            && self.edge_path.len() == other.edge_path.len()
    }
}

fn better_path(
    candidate_nodes: &[usize],
    candidate_edges: &[MeshAdjacency],
    existing_nodes: &[usize],
    existing_edges: &[MeshAdjacency],
) -> bool {
    if existing_nodes.is_empty() {
        return true;
    }

    if candidate_nodes.len() != existing_nodes.len() {
        return candidate_nodes.len() < existing_nodes.len();
    }

    if candidate_nodes != existing_nodes {
        return candidate_nodes < existing_nodes;
    }

    let candidate_edge_keys: Vec<u64> =
        candidate_edges.iter().map(|edge| edge.edge_index).collect();
    let existing_edge_keys: Vec<u64> = existing_edges.iter().map(|edge| edge.edge_index).collect();

    if candidate_edge_keys != existing_edge_keys {
        return candidate_edge_keys < existing_edge_keys;
    }

    false
}

fn transition_cost(
    graph: &SurfaceMeshGraph,
    from_triangle: usize,
    adjacency: &MeshAdjacency,
    path_mode: SurfaceTrimPathMode,
    global_direction: &[f64; 3],
) -> f64 {
    let base = adjacency.edge_length * EDGE_LENGTH_WEIGHT;
    let direction_penalty = normalize(sub(
        graph.triangle_centroids[adjacency.neighbor],
        graph.triangle_centroids[from_triangle],
    ))
    .ok()
    .zip(normalize(*global_direction).ok())
    .map(|(step, target)| {
        (1.0 - dot(step, target).clamp(-1.0, 1.0))
            * adjacency.edge_length
            * DIRECTION_DEVIATION_WEIGHT
    })
    .unwrap_or(0.0);
    if matches!(path_mode, SurfaceTrimPathMode::Feature) {
        let flatness = 1.0 - (adjacency.dihedral / std::f64::consts::PI).clamp(0.0, 1.0);
        (base + direction_penalty + flatness * EDGE_FLATNESS_WEIGHT
            - (adjacency.dihedral / std::f64::consts::PI) * CREASE_REWARD_WEIGHT)
            .max(COST_EPSILON)
    } else {
        (base + direction_penalty).max(COST_EPSILON)
    }
}

fn edge_path_segments(
    graph: &SurfaceMeshGraph,
    triangle_path: &[usize],
    edge_path: &[MeshAdjacency],
    path_mode: SurfaceTrimPathMode,
    global_direction: &[f64; 3],
) -> Vec<SurfaceTrimPathSegment> {
    if triangle_path.len() < 2 || edge_path.len() != triangle_path.len() - 1 {
        return Vec::new();
    }

    let mut segments = Vec::with_capacity(edge_path.len());
    for (index, edge) in edge_path.iter().enumerate() {
        let from = triangle_path[index];
        let to = triangle_path[index + 1];
        debug_assert_eq!(edge.neighbor, to);
        segments.push(SurfaceTrimPathSegment {
            from_triangle: from as u64,
            to_triangle: to as u64,
            shared_edge: [
                u64::from(edge.shared_edge[0]),
                u64::from(edge.shared_edge[1]),
            ],
            cost: transition_cost(graph, from, edge, path_mode, global_direction),
        });
    }

    segments
}

fn normalize(value: [f64; 3]) -> AppResult<[f64; 3]> {
    let magnitude = norm(value);
    if !magnitude.is_finite() || magnitude <= VERTEX_EPSILON {
        return Err(AppError::validation(
            "Triangle normal is degenerate and cannot be normalized.",
        ));
    }
    Ok(scale(value, 1.0 / magnitude))
}

fn sub(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn scale(value: [f64; 3], amount: f64) -> [f64; 3] {
    [value[0] * amount, value[1] * amount, value[2] * amount]
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn norm(value: [f64; 3]) -> f64 {
    (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt()
}

fn distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    norm(sub(left, right))
}

#[cfg(test)]
fn ascii_stl(path: &Path, vertices: &[[f64; 3]], triangles: &[[u32; 3]]) -> std::io::Result<()> {
    let mut content = String::new();
    content.push_str("solid surface_trim_graph\n");
    for triangle in triangles {
        let a = vertices[triangle[0] as usize];
        let b = vertices[triangle[1] as usize];
        let c = vertices[triangle[2] as usize];
        let raw_normal = cross(sub(b, a), sub(c, a));
        let normal = normalize(raw_normal).unwrap_or([0.0, 0.0, 0.0]);
        content.push_str(&format!(
            "facet normal {} {} {}\n  outer loop\n    vertex {} {} {}\n    vertex {} {} {}\n    vertex {} {} {}\n  endloop\nendfacet\n",
            normal[0], normal[1], normal[2], a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2]
        ));
    }
    content.push_str("endsolid surface_trim_graph\n");
    std::fs::write(path, content)
}

#[cfg(test)]
mod loop_region_tests {
    use super::*;
    use std::env;

    fn temp_root(label: &str) -> std::path::PathBuf {
        env::temp_dir().join(format!(
            "ecky-surface-trim-loop-region-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }

    fn write_mesh(
        label: &str,
        vertices: &[[f64; 3]],
        triangles: &[[u32; 3]],
    ) -> std::path::PathBuf {
        let path = temp_root(label).with_extension("stl");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("temp dir");
        }
        ascii_stl(&path, vertices, triangles).expect("stl write");
        path
    }

    fn grid_mesh(width: usize, height: usize) -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
        let mut vertices = Vec::new();
        for y in 0..=height {
            for x in 0..=width {
                vertices.push([x as f64, y as f64, 0.0]);
            }
        }

        let index = |x: usize, y: usize| -> u32 { (y * (width + 1) + x) as u32 };
        let mut triangles = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let bl = index(x, y);
                let br = index(x + 1, y);
                let tl = index(x, y + 1);
                let tr = index(x + 1, y + 1);
                triangles.push([bl, br, tr]);
                triangles.push([bl, tr, tl]);
            }
        }

        (vertices, triangles)
    }

    fn chain_mesh() -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
            [3.0, 0.0, 0.0],
            [3.0, 1.0, 0.0],
        ];
        let triangles = vec![[0, 1, 2], [1, 3, 2], [3, 5, 2]];
        (vertices, triangles)
    }

    fn disconnected_mesh() -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
        let (mut vertices, mut triangles) = chain_mesh();
        let offset = vertices.len() as u32;
        vertices.extend([
            [10.0, 0.0, 0.0],
            [11.0, 0.0, 0.0],
            [10.0, 1.0, 0.0],
            [12.0, 0.0, 0.0],
            [12.0, 1.0, 0.0],
            [13.0, 0.0, 0.0],
            [13.0, 1.0, 0.0],
        ]);
        triangles.extend([
            [offset, offset + 1, offset + 2],
            [offset + 1, offset + 3, offset + 2],
            [offset + 3, offset + 5, offset + 2],
        ]);
        (vertices, triangles)
    }

    fn mesh_anchor(mesh: &IndexedMeshAsset, triangle_index: usize) -> CaptureSurfaceAnchor {
        mesh_anchor_at(mesh, triangle_index, [1.0, 0.0, 0.0])
    }

    fn mesh_anchor_at(
        mesh: &IndexedMeshAsset,
        triangle_index: usize,
        barycentric: [f64; 3],
    ) -> CaptureSurfaceAnchor {
        let triangle = mesh.triangles()[triangle_index];
        let a = mesh.vertices()[triangle[0] as usize];
        let b = mesh.vertices()[triangle[1] as usize];
        let c = mesh.vertices()[triangle[2] as usize];
        let normal = normalize(cross(sub(b, a), sub(c, a))).expect("triangle normal");
        let source_position = [
            a[0] * barycentric[0] + b[0] * barycentric[1] + c[0] * barycentric[2],
            a[1] * barycentric[0] + b[1] * barycentric[1] + c[1] * barycentric[2],
            a[2] * barycentric[0] + b[2] * barycentric[1] + c[2] * barycentric[2],
        ];
        CaptureSurfaceAnchor {
            source_mesh_content_digest: mesh.content_digest().to_string(),
            triangle_index: triangle_index as u64,
            barycentric,
            source_position,
            source_normal: normal,
        }
    }

    #[test]
    fn closes_loop_and_returns_preview_region() {
        let (vertices, triangles) = grid_mesh(3, 3);
        let path = write_mesh("closed-loop", &vertices, &triangles);
        let mesh = IndexedMeshAsset::from_stl(
            crate::ecky_ir::mesh_asset::MeshAssetSource::Imported,
            &path,
        )
        .expect("mesh");
        let anchors = vec![
            mesh_anchor(&mesh, 15),
            mesh_anchor(&mesh, 10),
            mesh_anchor(&mesh, 3),
            mesh_anchor(&mesh, 6),
        ];
        let seed = mesh_anchor(&mesh, 8);

        let preview =
            preview_surface_trim_region(&path, &anchors, &seed, SurfaceTrimPathMode::Shortest)
                .expect("preview");

        assert_eq!(preview.loop_segment_count, anchors.len() as u64);
        let last_segment = preview.loop_segments.last().expect("closing segment");
        assert_eq!(
            last_segment.from_triangle_index,
            anchors.last().unwrap().triangle_index
        );
        assert_eq!(last_segment.to_triangle_index, anchors[0].triangle_index);
        assert_eq!(preview.keep_seed_triangle_index, seed.triangle_index);
        assert!(preview
            .retained_triangle_indices
            .contains(&seed.triangle_index));
        assert!(!preview.retained_triangle_indices.is_empty());
    }

    #[test]
    fn seed_region_excludes_other_disconnected_shells() {
        let (mut vertices, mut triangles) = grid_mesh(3, 3);
        let detached_vertex = vertices.len() as u32;
        vertices.extend([[10.0, 0.0, 0.0], [11.0, 0.0, 0.0], [10.0, 1.0, 0.0]]);
        let detached_triangle = triangles.len() as u64;
        triangles.push([detached_vertex, detached_vertex + 1, detached_vertex + 2]);
        let path = write_mesh("seed-component-only", &vertices, &triangles);
        let mesh = IndexedMeshAsset::from_stl(
            crate::ecky_ir::mesh_asset::MeshAssetSource::Imported,
            &path,
        )
        .expect("mesh");
        let anchors = vec![
            mesh_anchor(&mesh, 15),
            mesh_anchor(&mesh, 10),
            mesh_anchor(&mesh, 3),
            mesh_anchor(&mesh, 6),
        ];
        let seed = mesh_anchor(&mesh, 8);

        let preview =
            preview_surface_trim_region(&path, &anchors, &seed, SurfaceTrimPathMode::Shortest)
                .expect("preview");

        assert!(preview
            .retained_triangle_indices
            .contains(&seed.triangle_index));
        assert!(!preview
            .retained_triangle_indices
            .contains(&detached_triangle));
    }

    #[test]
    fn rejects_duplicate_anchor_triangle() {
        let (vertices, triangles) = grid_mesh(2, 2);
        let path = write_mesh("duplicate-anchor", &vertices, &triangles);
        let mesh = IndexedMeshAsset::from_stl(
            crate::ecky_ir::mesh_asset::MeshAssetSource::Imported,
            &path,
        )
        .expect("mesh");
        let anchors = vec![
            mesh_anchor(&mesh, 0),
            mesh_anchor(&mesh, 1),
            mesh_anchor(&mesh, 1),
        ];
        let seed = mesh_anchor(&mesh, 2);

        let error =
            preview_surface_trim_region(&path, &anchors, &seed, SurfaceTrimPathMode::Shortest)
                .expect_err("duplicate anchor rejected");

        assert!(error.to_string().contains("duplicated"));
    }

    #[test]
    fn rejects_disconnected_loop_segment_with_segment_id() {
        let (vertices, triangles) = disconnected_mesh();
        let path = write_mesh("disconnected-segment", &vertices, &triangles);
        let mesh = IndexedMeshAsset::from_stl(
            crate::ecky_ir::mesh_asset::MeshAssetSource::Imported,
            &path,
        )
        .expect("mesh");
        let anchors = vec![
            mesh_anchor(&mesh, 0),
            mesh_anchor(&mesh, 1),
            mesh_anchor(&mesh, 4),
        ];
        let seed = mesh_anchor(&mesh, 0);

        let error =
            preview_surface_trim_region(&path, &anchors, &seed, SurfaceTrimPathMode::Shortest)
                .expect_err("disconnected loop rejected");

        assert!(error.to_string().contains("Loop segment 2"));
    }

    #[test]
    fn rejects_revisited_triangle_in_loop_path() {
        let (vertices, triangles) = chain_mesh();
        let path = write_mesh("self-intersection", &vertices, &triangles);
        let mesh = IndexedMeshAsset::from_stl(
            crate::ecky_ir::mesh_asset::MeshAssetSource::Imported,
            &path,
        )
        .expect("mesh");
        let anchors = vec![
            mesh_anchor(&mesh, 0),
            mesh_anchor(&mesh, 2),
            mesh_anchor(&mesh, 1),
        ];
        let seed = mesh_anchor(&mesh, 0);

        let error =
            preview_surface_trim_region(&path, &anchors, &seed, SurfaceTrimPathMode::Shortest)
                .expect_err("revisited triangle rejected");

        assert!(error
            .to_string()
            .contains("Loop segment 2 overlaps segment 1"));
    }

    #[test]
    fn rejects_keep_seed_digest_mismatch() {
        let (vertices, triangles) = grid_mesh(2, 2);
        let path = write_mesh("seed-digest", &vertices, &triangles);
        let mesh = IndexedMeshAsset::from_stl(
            crate::ecky_ir::mesh_asset::MeshAssetSource::Imported,
            &path,
        )
        .expect("mesh");
        let anchors = vec![
            mesh_anchor(&mesh, 0),
            mesh_anchor(&mesh, 1),
            mesh_anchor(&mesh, 2),
        ];
        let mut seed = mesh_anchor(&mesh, 3);
        seed.source_mesh_content_digest = "sha256:wrong".to_string();

        let error =
            preview_surface_trim_region(&path, &anchors, &seed, SurfaceTrimPathMode::Shortest)
                .expect_err("digest mismatch rejected");

        assert!(error
            .to_string()
            .contains("Keep seed references a different source mesh digest"));
    }

    #[test]
    fn rejects_non_partitioning_loop_on_open_strip() {
        let (vertices, triangles) = grid_mesh(2, 2);
        let path = write_mesh("non-partitioning", &vertices, &triangles);
        let mesh = IndexedMeshAsset::from_stl(
            crate::ecky_ir::mesh_asset::MeshAssetSource::Imported,
            &path,
        )
        .expect("mesh");
        let anchors = vec![
            mesh_anchor_at(&mesh, 0, [0.6, 0.2, 0.2]),
            mesh_anchor_at(&mesh, 0, [0.2, 0.6, 0.2]),
            mesh_anchor_at(&mesh, 0, [0.2, 0.2, 0.6]),
        ];

        let error = preview_surface_trim_loop(&path, &anchors, SurfaceTrimPathMode::Shortest, 1)
            .expect_err("non-partitioning loop rejected");

        assert!(
            error.to_string().contains("cannot partition"),
            "unexpected error: {error}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_root(label: &str) -> std::path::PathBuf {
        env::temp_dir().join(format!(
            "ecky-surface-trim-external-shapes-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }

    fn triangle_anchor(
        mesh: &IndexedMeshAsset,
        triangle_index: usize,
        vertex_index: usize,
    ) -> (CaptureSurfaceAnchor, [f64; 3]) {
        let triangle = mesh.triangles()[triangle_index];
        let raw_normal = cross(
            sub(
                mesh.vertices()[triangle[1] as usize],
                mesh.vertices()[triangle[0] as usize],
            ),
            sub(
                mesh.vertices()[triangle[2] as usize],
                mesh.vertices()[triangle[0] as usize],
            ),
        );
        let source_normal = normalize(raw_normal).unwrap_or([0.0, 0.0, 1.0]);
        (
            CaptureSurfaceAnchor {
                source_mesh_content_digest: mesh.content_digest().to_string(),
                triangle_index: triangle_index as u64,
                source_position: mesh.vertices()[triangle[vertex_index] as usize],
                barycentric: match vertex_index {
                    0 => [1.0, 0.0, 0.0],
                    1 => [0.0, 1.0, 0.0],
                    _ => [0.0, 0.0, 1.0],
                },
                source_normal,
            },
            source_normal,
        )
    }

    fn fixture_with_two_equal_routes() -> (std::path::PathBuf, IndexedMeshAsset) {
        let root = temp_root("equal-routes");
        std::fs::create_dir_all(&root).expect("mkdir");
        let path = root.join("equal-routes.stl");
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, -1.0, 0.0],
            [0.5, 0.5, 0.5],
        ];
        let triangles = vec![
            [0, 1, 2],
            [0, 2, 5],
            [0, 5, 4],
            [3, 4, 5],
            [0, 1, 5],
            [1, 4, 5],
        ];
        ascii_stl(&path, &vertices, &triangles).expect("write stl");
        let mesh = IndexedMeshAsset::from_stl(
            crate::ecky_ir::mesh_asset::MeshAssetSource::Imported,
            &path,
        )
        .expect("mesh");
        clear_surface_trim_graph_cache();
        (path, mesh)
    }

    fn fixture_with_feature_pref() -> (std::path::PathBuf, IndexedMeshAsset) {
        let root = temp_root("feature-pref");
        std::fs::create_dir_all(&root).expect("mkdir");
        let path = root.join("feature-pref.stl");
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, -1.0, 0.0],
            [0.5, 0.5, 0.5],
        ];
        let triangles = vec![
            [0, 1, 2],
            [0, 2, 5],
            [0, 5, 4],
            [3, 4, 5],
            [0, 1, 5],
            [1, 4, 5],
        ];
        ascii_stl(&path, &vertices, &triangles).expect("write stl");
        let mesh = IndexedMeshAsset::from_stl(
            crate::ecky_ir::mesh_asset::MeshAssetSource::Imported,
            &path,
        )
        .expect("mesh");
        clear_surface_trim_graph_cache();
        (path, mesh)
    }

    #[test]
    fn bdd_deterministic_shortest_path_has_stable_tie_break_by_triangle_index() {
        let (path, mesh) = fixture_with_two_equal_routes();
        let (start, _) = triangle_anchor(&mesh, 0, 0);
        let (end, _) = triangle_anchor(&mesh, 3, 0);

        let path_result =
            surface_trim_path(&path, &start, &end, SurfaceTrimPathMode::Shortest).expect("path");

        assert_eq!(path_result.path_mode, SurfaceTrimPathMode::Shortest);
        assert_eq!(
            path_result.triangle_corridor,
            vec![0, 1, 2, 3],
            "tie must resolve to the lower triangle path deterministically"
        );
        assert_eq!(path_result.diagnostics.connected_components, 1);

        std::fs::remove_dir_all(path.parent().unwrap()).expect("cleanup");
    }

    #[test]
    fn bdd_source_anchor_with_wrong_digest_is_rejected_before_path_search() {
        let (path, mesh) = fixture_with_two_equal_routes();
        let (start, _) = triangle_anchor(&mesh, 0, 0);
        let (end, _) = triangle_anchor(&mesh, 3, 0);

        let mut wrong = end;
        wrong.source_mesh_content_digest = "sha256:does-not-exist".to_string();

        let error = surface_trim_path(&path, &start, &wrong, SurfaceTrimPathMode::Shortest)
            .expect_err("wrong end digest");
        assert!(error
            .message
            .contains("Capture anchor mesh digest differs from selected source mesh."),);

        std::fs::remove_dir_all(path.parent().unwrap()).expect("cleanup");
    }

    #[test]
    fn bdd_stale_triangle_anchor_is_rejected_with_validation_error() {
        let (path, mesh) = fixture_with_two_equal_routes();
        let (start, _) = triangle_anchor(&mesh, 0, 0);
        let mut bad = start.clone();
        bad.triangle_index = 9_999;
        let (end, _) = triangle_anchor(&mesh, 3, 0);

        let error = surface_trim_path(&path, &bad, &end, SurfaceTrimPathMode::Shortest)
            .expect_err("stale anchor");
        assert!(
            error
                .message
                .contains("Capture anchor triangle index is out of bounds."),
            "anchor stale should fail with triangle bound error"
        );

        std::fs::remove_dir_all(path.parent().unwrap()).expect("cleanup");
    }

    #[test]
    fn bdd_feature_mode_prefers_crease_over_near_equal_path() {
        let (path, mesh) = fixture_with_feature_pref();
        let (start, _) = triangle_anchor(&mesh, 0, 0);
        let (end, _) = triangle_anchor(&mesh, 3, 0);

        let shortest = surface_trim_path(&path, &start, &end, SurfaceTrimPathMode::Shortest)
            .expect("shortest");
        let feature =
            surface_trim_path(&path, &start, &end, SurfaceTrimPathMode::Feature).expect("feature");

        assert_eq!(shortest.triangle_corridor, vec![0, 1, 2, 3]);
        assert_eq!(feature.triangle_corridor, vec![0, 4, 5, 3]);
        assert!(feature.total_cost < shortest.total_cost);

        std::fs::remove_dir_all(path.parent().unwrap()).expect("cleanup");
    }

    #[test]
    fn bdd_repeat_path_request_reuses_digest_graph_cache() {
        let (path, mesh) = fixture_with_two_equal_routes();
        let (start, _) = triangle_anchor(&mesh, 0, 0);
        let (end, _) = triangle_anchor(&mesh, 3, 0);

        let first = surface_trim_path(&path, &start, &end, SurfaceTrimPathMode::Shortest)
            .expect("first path");
        let second = surface_trim_path(&path, &start, &end, SurfaceTrimPathMode::Shortest)
            .expect("cached path");
        let first_cached = load_or_build_graph(&path, mesh.content_digest()).expect("first cache");
        let second_cached =
            load_or_build_graph(&path, mesh.content_digest()).expect("second cache");

        assert_eq!(first, second);
        assert!(Arc::ptr_eq(&first_cached.graph, &second_cached.graph));
        std::fs::remove_dir_all(path.parent().unwrap()).expect("cleanup");
    }
}
