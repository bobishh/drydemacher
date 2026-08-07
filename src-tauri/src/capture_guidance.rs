use crate::contracts::{
    AppError, AppResult, CaptureCalibrationMethod, CaptureReconstructionGuide, CaptureSurfaceAnchor,
};
use crate::ecky_ir::mesh_asset::{IndexedMeshAsset, MeshAssetSource};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::Path;

const VECTOR_EPSILON: f64 = 1.0e-12;

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedSurfaceAnchor {
    pub source_position: [f64; 3],
    pub source_normal: [f64; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KnownDistanceObservation {
    pub source_distance: f64,
    pub known_distance_mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KnownDistanceFit {
    pub millimetres_per_source_unit: f64,
    pub residuals_mm: Vec<f64>,
    pub rms_residual_mm: f64,
    pub max_residual_mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FittedReconstructionFrame {
    pub origin_mm: [f64; 3],
    pub x_axis: [f64; 3],
    pub y_axis: [f64; 3],
    pub z_axis: [f64; 3],
}

impl FittedReconstructionFrame {
    pub fn to_local_mm(&self, world_mm: [f64; 3]) -> [f64; 3] {
        let offset = sub(world_mm, self.origin_mm);
        [
            dot(offset, self.x_axis),
            dot(offset, self.y_axis),
            dot(offset, self.z_axis),
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AxisFit {
    pub origin_mm: [f64; 3],
    pub direction: [f64; 3],
    pub rms_mm: f64,
    pub max_mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlaneFit {
    pub origin_mm: [f64; 3],
    pub normal: [f64; 3],
    pub rms_mm: f64,
    pub max_mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CircleFit {
    pub center_mm: [f64; 3],
    pub normal: [f64; 3],
    pub radius_mm: f64,
    pub rms_mm: f64,
    pub max_mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CylinderFit {
    pub origin_mm: [f64; 3],
    pub axis_direction: [f64; 3],
    pub radius_mm: f64,
    pub min_axis_mm: f64,
    pub max_axis_mm: f64,
    pub rms_mm: f64,
    pub max_mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConeFit {
    pub apex_mm: [f64; 3],
    pub axis_direction: [f64; 3],
    pub half_angle_deg: f64,
    pub min_axis_mm: f64,
    pub max_axis_mm: f64,
    pub rms_mm: f64,
    pub max_mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SphereFit {
    pub center_mm: [f64; 3],
    pub radius_mm: f64,
    pub rms_mm: f64,
    pub max_mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct ProfileArcFit {
    center_mm: [f64; 3],
    normal: [f64; 3],
    radius_mm: f64,
    start_angle_deg: f64,
    end_angle_deg: f64,
    rms_mm: f64,
    max_mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct ProfileSplineFit {
    degree: u32,
    control_points_mm: Vec<[f64; 3]>,
    knots: Vec<f64>,
    rms_mm: f64,
    max_mm: f64,
}

pub fn source_mesh_content_digest(path: &Path) -> AppResult<String> {
    let mesh = IndexedMeshAsset::from_stl(MeshAssetSource::Imported, path)?;
    Ok(mesh.content_digest().to_string())
}

pub fn inspect_capture_source_mesh(
    path: &Path,
    selection: crate::contracts::CaptureMeshSelection,
) -> AppResult<crate::contracts::CaptureGuideSourceMesh> {
    let bytes = std::fs::read(path).map_err(|error| {
        AppError::not_found(format!(
            "Capture source mesh could not be read '{}': {error}",
            path.display()
        ))
    })?;
    let mesh = IndexedMeshAsset::from_stl(MeshAssetSource::Imported, path)?;
    let mut minimum = [f64::INFINITY; 3];
    let mut maximum = [f64::NEG_INFINITY; 3];
    for vertex in mesh.vertices() {
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(vertex[axis]);
            maximum[axis] = maximum[axis].max(vertex[axis]);
        }
    }
    let artifact_digest = format!("sha256:{:x}", Sha256::digest(bytes));
    Ok(crate::contracts::CaptureGuideSourceMesh {
        artifact_digest: artifact_digest.clone(),
        content_digest: mesh.content_digest().to_string(),
        crop_digest: matches!(selection, crate::contracts::CaptureMeshSelection::Crop)
            .then_some(artifact_digest),
        selection,
        triangle_count: mesh.triangles().len() as u64,
        source_bounds: crate::contracts::CaptureSourceBounds {
            min: minimum,
            max: maximum,
        },
    })
}

pub fn validate_surface_anchor_from_stl(
    path: &Path,
    anchor: &CaptureSurfaceAnchor,
    position_tolerance: f64,
) -> AppResult<ValidatedSurfaceAnchor> {
    if !position_tolerance.is_finite() || position_tolerance < 0.0 {
        return Err(AppError::validation(
            "Capture anchor position tolerance must be finite and non-negative.",
        ));
    }
    let mesh = IndexedMeshAsset::from_stl(MeshAssetSource::Imported, path)?;
    if anchor.source_mesh_content_digest != mesh.content_digest() {
        return Err(AppError::conflict(
            "Capture anchor mesh digest differs from selected source mesh.",
        ));
    }
    let triangle = mesh
        .triangles()
        .get(anchor.triangle_index as usize)
        .ok_or_else(|| AppError::validation("Capture anchor triangle index is out of bounds."))?;
    if anchor
        .barycentric
        .iter()
        .any(|weight| !weight.is_finite() || *weight < -1.0e-9 || *weight > 1.0 + 1.0e-9)
        || (anchor.barycentric.iter().sum::<f64>() - 1.0).abs() > 1.0e-8
    {
        return Err(AppError::validation(
            "Capture anchor barycentric weights must be finite, bounded, and sum to one.",
        ));
    }
    let a = mesh.vertices()[triangle[0] as usize];
    let b = mesh.vertices()[triangle[1] as usize];
    let c = mesh.vertices()[triangle[2] as usize];
    let raw_normal = cross(sub(b, a), sub(c, a));
    let twice_area = norm(raw_normal);
    if !twice_area.is_finite() || twice_area <= VECTOR_EPSILON {
        return Err(AppError::validation(
            "Capture anchor selected triangle is degenerate.",
        ));
    }
    let normal = scale(raw_normal, 1.0 / twice_area);
    let interpolated = add(
        add(
            scale(a, anchor.barycentric[0]),
            scale(b, anchor.barycentric[1]),
        ),
        scale(c, anchor.barycentric[2]),
    );
    let position_error = norm(sub(interpolated, anchor.source_position));
    if !position_error.is_finite() || position_error > position_tolerance {
        return Err(AppError::validation(format!(
            "Capture anchor source position differs from triangle interpolation by {position_error:.6} source units."
        )));
    }
    let supplied_normal = normalize(anchor.source_normal).ok_or_else(|| {
        AppError::validation("Capture anchor source normal must be finite and non-zero.")
    })?;
    if dot(supplied_normal, normal) < 1.0 - 1.0e-6 {
        return Err(AppError::validation(
            if dot(supplied_normal, normal) < 0.0 {
                "Capture anchor normal opposes selected triangle orientation."
            } else {
                "Capture anchor normal differs from selected triangle orientation."
            },
        ));
    }
    Ok(ValidatedSurfaceAnchor {
        source_position: interpolated,
        source_normal: normal,
    })
}

pub fn extract_surface_neighborhood_from_stl(
    path: &Path,
    landmark_id: &str,
    anchor: &CaptureSurfaceAnchor,
    radius_source_units: f64,
    max_triangles: usize,
) -> AppResult<crate::contracts::CaptureSurfaceNeighborhood> {
    if landmark_id.trim().is_empty() {
        return Err(AppError::validation(
            "Capture surface neighborhood needs a landmark ID.",
        ));
    }
    if !radius_source_units.is_finite() || radius_source_units <= 0.0 {
        return Err(AppError::validation(
            "Capture surface neighborhood radius must be finite and positive.",
        ));
    }
    if !(1..=256).contains(&max_triangles) {
        return Err(AppError::validation(
            "Capture surface neighborhood triangle budget must be between 1 and 256.",
        ));
    }

    let validated = validate_surface_anchor_from_stl(path, anchor, 1.0e-6)?;
    let mesh = IndexedMeshAsset::from_stl(MeshAssetSource::Imported, path)?;
    extract_surface_neighborhood_from_mesh(
        &mesh,
        landmark_id,
        anchor,
        validated.source_position,
        radius_source_units,
        max_triangles,
    )
}

fn extract_surface_neighborhood_from_mesh(
    mesh: &IndexedMeshAsset,
    landmark_id: &str,
    anchor: &CaptureSurfaceAnchor,
    source_position: [f64; 3],
    radius_source_units: f64,
    max_triangles: usize,
) -> AppResult<crate::contracts::CaptureSurfaceNeighborhood> {
    if anchor.source_mesh_content_digest != mesh.content_digest() {
        return Err(AppError::conflict(
            "Capture anchor mesh digest differs from selected source mesh.",
        ));
    }
    let seed_triangle_index = usize::try_from(anchor.triangle_index)
        .map_err(|_| AppError::validation("Capture anchor triangle index is out of bounds."))?;
    if seed_triangle_index >= mesh.triangles().len() {
        return Err(AppError::validation(
            "Capture anchor triangle index is out of bounds.",
        ));
    }

    let mut edge_triangles = BTreeMap::<(u32, u32), Vec<usize>>::new();
    for (triangle_index, triangle) in mesh.triangles().iter().enumerate() {
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
            edge_triangles.entry(edge).or_default().push(triangle_index);
        }
    }
    for triangles in edge_triangles.values_mut() {
        triangles.sort_unstable();
        triangles.dedup();
    }

    let within_radius = |triangle_index: usize| {
        mesh.triangles()[triangle_index].iter().any(|vertex_index| {
            norm(sub(
                mesh.vertices()[*vertex_index as usize],
                source_position,
            )) <= radius_source_units + VECTOR_EPSILON
        })
    };
    let mut selected = BTreeSet::<usize>::new();
    let mut queued = BTreeSet::<usize>::new();
    let mut queue = VecDeque::from([seed_triangle_index]);
    queued.insert(seed_triangle_index);
    let mut truncated_by_budget = false;
    while let Some(triangle_index) = queue.pop_front() {
        if !within_radius(triangle_index) && triangle_index != seed_triangle_index {
            continue;
        }
        if selected.len() == max_triangles {
            truncated_by_budget = true;
            break;
        }
        selected.insert(triangle_index);
        let triangle = mesh.triangles()[triangle_index];
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
            for neighbor in &edge_triangles[&edge] {
                if queued.insert(*neighbor) {
                    queue.push_back(*neighbor);
                }
            }
        }
    }
    if !queue.is_empty() {
        truncated_by_budget = true;
    }

    let mut vertex_indices = BTreeSet::<u32>::new();
    let mut normal_sum = [0.0; 3];
    let mut triangle_normals = Vec::with_capacity(selected.len());
    let mut triangle_centroids = BTreeMap::<usize, [f64; 3]>::new();
    let mut sampled_area_source_units_squared = 0.0;
    let mut reached_mesh_boundary = false;
    for triangle_index in &selected {
        let triangle = mesh.triangles()[*triangle_index];
        vertex_indices.extend(triangle);
        let a = mesh.vertices()[triangle[0] as usize];
        let b = mesh.vertices()[triangle[1] as usize];
        let c = mesh.vertices()[triangle[2] as usize];
        let raw_normal = cross(sub(b, a), sub(c, a));
        let area_weight = norm(raw_normal);
        if area_weight <= VECTOR_EPSILON || !area_weight.is_finite() {
            return Err(AppError::validation(
                "Capture surface neighborhood contains a degenerate triangle.",
            ));
        }
        sampled_area_source_units_squared += area_weight * 0.5;
        normal_sum = add(normal_sum, raw_normal);
        triangle_normals.push(scale(raw_normal, 1.0 / area_weight));
        triangle_centroids.insert(*triangle_index, scale(add(add(a, b), c), 1.0 / 3.0));
        reached_mesh_boundary |= [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ]
        .into_iter()
        .map(|(left, right)| {
            if left <= right {
                (left, right)
            } else {
                (right, left)
            }
        })
        .any(|edge| {
            edge_triangles
                .get(&edge)
                .is_some_and(|triangles| triangles.len() == 1)
        });
    }
    let mean_normal = normalize(normal_sum).ok_or_else(|| {
        AppError::validation("Capture surface neighborhood normals cancel or are non-finite.")
    })?;
    let normal_spread_deg = triangle_normals
        .iter()
        .map(|normal| {
            dot(*normal, mean_normal)
                .clamp(-1.0, 1.0)
                .acos()
                .to_degrees()
        })
        .fold(0.0, f64::max);
    let normal_variation_rms_deg = (triangle_normals
        .iter()
        .map(|normal| {
            let angle = dot(*normal, mean_normal)
                .clamp(-1.0, 1.0)
                .acos()
                .to_degrees();
            angle * angle
        })
        .sum::<f64>()
        / triangle_normals.len() as f64)
        .sqrt();
    let vertices = vertex_indices
        .iter()
        .map(|index| mesh.vertices()[*index as usize])
        .collect::<Vec<_>>();
    let centroid_source = centroid(&vertices);
    let radial_coverage_ratio = vertices
        .iter()
        .map(|vertex| norm(sub(*vertex, source_position)) / radius_source_units)
        .fold(0.0, f64::max)
        .clamp(0.0, 1.0);
    let position_rms_source_units = (vertices
        .iter()
        .map(|vertex| {
            let distance = norm(sub(*vertex, centroid_source));
            distance * distance
        })
        .sum::<f64>()
        / vertices.len() as f64)
        .sqrt();
    let (plane_values, plane_vectors) = symmetric_eigen(covariance(&vertices, centroid_source));
    let smallest = if plane_values[0] <= plane_values[1] && plane_values[0] <= plane_values[2] {
        0
    } else if plane_values[1] <= plane_values[2] {
        1
    } else {
        2
    };
    let mut plane_normal = normalize(column(plane_vectors, smallest)).ok_or_else(|| {
        AppError::validation("Capture surface neighborhood plane fit is degenerate.")
    })?;
    if dot(plane_normal, mean_normal) < 0.0 {
        plane_normal = scale(plane_normal, -1.0);
    }
    let plane_residuals = vertices
        .iter()
        .map(|vertex| dot(sub(*vertex, centroid_source), plane_normal).abs())
        .collect::<Vec<_>>();
    let (planarity_rms_source_units, planarity_max_source_units) =
        residual_summary(&plane_residuals);
    let selected_lookup = selected.iter().copied().collect::<BTreeSet<_>>();
    let mut adjacency_edges = BTreeSet::<[u64; 2]>::new();
    let mut curvature_samples = Vec::new();
    for triangles in edge_triangles.values() {
        let selected_incident = triangles
            .iter()
            .copied()
            .filter(|triangle| selected_lookup.contains(triangle))
            .collect::<Vec<_>>();
        for left_index in 0..selected_incident.len() {
            for right_index in left_index + 1..selected_incident.len() {
                let left = selected_incident[left_index];
                let right = selected_incident[right_index];
                adjacency_edges.insert([left as u64, right as u64]);
                let left_triangle = mesh.triangles()[left];
                let right_triangle = mesh.triangles()[right];
                let left_raw = cross(
                    sub(
                        mesh.vertices()[left_triangle[1] as usize],
                        mesh.vertices()[left_triangle[0] as usize],
                    ),
                    sub(
                        mesh.vertices()[left_triangle[2] as usize],
                        mesh.vertices()[left_triangle[0] as usize],
                    ),
                );
                let right_raw = cross(
                    sub(
                        mesh.vertices()[right_triangle[1] as usize],
                        mesh.vertices()[right_triangle[0] as usize],
                    ),
                    sub(
                        mesh.vertices()[right_triangle[2] as usize],
                        mesh.vertices()[right_triangle[0] as usize],
                    ),
                );
                let left_normal = normalize(left_raw).ok_or_else(|| {
                    AppError::validation(
                        "Capture surface neighborhood has invalid adjacency normal.",
                    )
                })?;
                let right_normal = normalize(right_raw).ok_or_else(|| {
                    AppError::validation(
                        "Capture surface neighborhood has invalid adjacency normal.",
                    )
                })?;
                let distance = norm(sub(triangle_centroids[&left], triangle_centroids[&right]));
                if distance > VECTOR_EPSILON {
                    curvature_samples
                        .push(dot(left_normal, right_normal).clamp(-1.0, 1.0).acos() / distance);
                }
            }
        }
    }
    let estimated_curvature_per_source_unit = if curvature_samples.is_empty() {
        0.0
    } else {
        (curvature_samples
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            / curvature_samples.len() as f64)
            .sqrt()
    };
    let curvature_excursion =
        radius_source_units * (normal_variation_rms_deg.to_radians() * 0.5).sin().abs();
    let position_uncertainty_source_units = planarity_rms_source_units.hypot(curvature_excursion);

    Ok(crate::contracts::CaptureSurfaceNeighborhood {
        neighborhood_id: format!("neighborhood:{landmark_id}"),
        landmark_id: landmark_id.to_string(),
        source_mesh_content_digest: mesh.content_digest().to_string(),
        seed_triangle_index: anchor.triangle_index,
        triangle_indices: selected.into_iter().map(|index| index as u64).collect(),
        adjacency_edges: adjacency_edges.into_iter().collect(),
        vertex_indices: vertex_indices.into_iter().map(u64::from).collect(),
        sample_count: vertices.len() as u64,
        radius_source_units,
        sampled_area_source_units_squared,
        radial_coverage_ratio,
        centroid_source,
        mean_normal,
        normal_spread_deg,
        normal_variation_rms_deg,
        estimated_curvature_per_source_unit,
        position_rms_source_units,
        planarity_rms_source_units,
        planarity_max_source_units,
        position_uncertainty_source_units,
        reached_mesh_boundary,
        truncated_by_budget,
    })
}

pub fn propose_capture_anchor_remap(
    new_source_stl_path: &Path,
    guide: &CaptureReconstructionGuide,
    landmark_id: &str,
    mut new_anchor: CaptureSurfaceAnchor,
) -> AppResult<crate::contracts::CaptureAnchorRemapProposal> {
    let landmark = guide
        .landmarks
        .iter()
        .find(|landmark| landmark.landmark_id == landmark_id)
        .ok_or_else(|| {
            AppError::validation(format!(
                "Capture remap references missing landmark '{landmark_id}'."
            ))
        })?;
    if new_anchor.source_mesh_content_digest == landmark.anchor.source_mesh_content_digest {
        return Err(AppError::validation(
            "Capture remap candidate must reference a different source mesh digest.",
        ));
    }
    let validated = validate_surface_anchor_from_stl(new_source_stl_path, &new_anchor, 1.0e-6)?;
    new_anchor.source_position = validated.source_position;
    new_anchor.source_normal = validated.source_normal;
    let scale_mm = guide.calibration.millimetres_per_source_unit;
    if !scale_mm.is_finite() || scale_mm <= 0.0 {
        return Err(AppError::validation(
            "Capture remap requires finite positive guide calibration scale.",
        ));
    }
    let frame = FittedReconstructionFrame {
        origin_mm: guide.reconstruction_frame.origin_mm,
        x_axis: guide.reconstruction_frame.x_axis,
        y_axis: guide.reconstruction_frame.y_axis,
        z_axis: guide.reconstruction_frame.z_axis,
    };
    let candidate_local_mm = frame.to_local_mm(scale(new_anchor.source_position, scale_mm));
    let residual_mm = norm(sub(candidate_local_mm, landmark.local_position_mm));
    if !residual_mm.is_finite() {
        return Err(AppError::validation(
            "Capture remap residual is non-finite.",
        ));
    }
    let identity = serde_json::to_vec(&(
        guide.guide_id.as_str(),
        guide.revision,
        landmark_id,
        &landmark.anchor,
        &new_anchor,
    ))
    .map_err(|error| AppError::internal(format!("Capture remap identity failed: {error}")))?;
    Ok(crate::contracts::CaptureAnchorRemapProposal {
        proposal_id: format!("remap-{:x}", Sha256::digest(identity)),
        landmark_id: landmark_id.to_string(),
        old_anchor: landmark.anchor.clone(),
        new_anchor,
        residual_mm,
        confirmed: false,
    })
}

pub fn apply_confirmed_capture_anchor_remaps(
    new_source_stl_path: &Path,
    guide: &mut CaptureReconstructionGuide,
) -> AppResult<()> {
    let new_source =
        inspect_capture_source_mesh(new_source_stl_path, guide.source_mesh.selection.clone())?;
    if new_source.content_digest == guide.source_mesh.content_digest {
        return Err(AppError::validation(
            "Capture remap apply requires a changed source mesh digest.",
        ));
    }
    let mut next = guide.clone();
    for landmark in &mut next.landmarks {
        let proposal = next
            .remap_proposals
            .iter()
            .find(|proposal| {
                proposal.confirmed
                    && proposal.landmark_id == landmark.landmark_id
                    && proposal.old_anchor == landmark.anchor
                    && proposal.new_anchor.source_mesh_content_digest == new_source.content_digest
            })
            .ok_or_else(|| {
                AppError::conflict(format!(
                    "Capture landmark '{}' has no confirmed remap for the selected source mesh.",
                    landmark.landmark_id
                ))
            })?;
        validate_surface_anchor_from_stl(new_source_stl_path, &proposal.new_anchor, 1.0e-6)?;
        landmark.anchor = proposal.new_anchor.clone();
    }
    next.source_mesh = new_source;
    next.remap_proposals.clear();
    recompute_guide_geometry_from_stl(new_source_stl_path, &mut next)?;
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or_else(|| AppError::validation("Capture guide revision overflow."))?;
    next.canonical_digest = next
        .compute_canonical_digest()
        .map_err(AppError::validation)?;
    *guide = next;
    Ok(())
}

pub fn fit_known_distance_scale(
    observations: &[KnownDistanceObservation],
    accepted_tolerance_mm: f64,
) -> AppResult<KnownDistanceFit> {
    if observations.is_empty() {
        return Err(AppError::validation(
            "Known-distance calibration needs at least one measurement.",
        ));
    }
    if !accepted_tolerance_mm.is_finite() || accepted_tolerance_mm < 0.0 {
        return Err(AppError::validation(
            "Known-distance tolerance must be finite and non-negative.",
        ));
    }
    for observation in observations {
        if !observation.source_distance.is_finite() || observation.source_distance <= VECTOR_EPSILON
        {
            return Err(AppError::validation(
                "Known-distance endpoints must be distinct and finite.",
            ));
        }
        if !observation.known_distance_mm.is_finite() || observation.known_distance_mm <= 0.0 {
            return Err(AppError::validation(
                "Known physical distance must be finite and positive.",
            ));
        }
    }
    let numerator = observations
        .iter()
        .map(|observation| observation.source_distance * observation.known_distance_mm)
        .sum::<f64>();
    let denominator = observations
        .iter()
        .map(|observation| observation.source_distance * observation.source_distance)
        .sum::<f64>();
    let fitted_scale = numerator / denominator;
    if !fitted_scale.is_finite() || fitted_scale <= 0.0 {
        return Err(AppError::validation(
            "Known-distance calibration produced invalid scale.",
        ));
    }
    let residuals_mm = observations
        .iter()
        .map(|observation| {
            fitted_scale * observation.source_distance - observation.known_distance_mm
        })
        .collect::<Vec<_>>();
    let max_residual_mm = residuals_mm
        .iter()
        .map(|value| value.abs())
        .fold(0.0, f64::max);
    let rms_residual_mm = (residuals_mm.iter().map(|value| value * value).sum::<f64>()
        / residuals_mm.len() as f64)
        .sqrt();
    if max_residual_mm > accepted_tolerance_mm {
        return Err(AppError::validation(format!(
            "Known-distance evidence conflicts: maximum residual {max_residual_mm:.6} mm exceeds tolerance {accepted_tolerance_mm:.6} mm."
        )));
    }
    Ok(KnownDistanceFit {
        millimetres_per_source_unit: fitted_scale,
        residuals_mm,
        rms_residual_mm,
        max_residual_mm,
    })
}

pub fn construct_reconstruction_frame(
    origin_mm: [f64; 3],
    x_direction_point_mm: [f64; 3],
    xy_plane_point_mm: [f64; 3],
) -> AppResult<FittedReconstructionFrame> {
    if !finite3(origin_mm) || !finite3(x_direction_point_mm) || !finite3(xy_plane_point_mm) {
        return Err(AppError::validation(
            "Frame evidence contains non-finite coordinates.",
        ));
    }
    let x_raw = sub(x_direction_point_mm, origin_mm);
    let x_axis = normalize(x_raw).ok_or_else(|| {
        AppError::validation("Frame evidence is degenerate: origin and X landmark coincide.")
    })?;
    let y_raw = sub(xy_plane_point_mm, origin_mm);
    if norm(y_raw) <= VECTOR_EPSILON {
        return Err(AppError::validation(
            "Frame evidence is degenerate: origin and Y landmark coincide.",
        ));
    }
    let y_orthogonal = sub(y_raw, scale(x_axis, dot(y_raw, x_axis)));
    let y_axis = normalize(y_orthogonal).ok_or_else(|| {
        AppError::validation(
            "Frame evidence is degenerate: origin, X, and Y landmarks are collinear.",
        )
    })?;
    let z_axis = normalize(cross(x_axis, y_axis))
        .ok_or_else(|| AppError::validation("Frame evidence cannot form a right-handed basis."))?;
    if dot(cross(x_axis, y_axis), z_axis) <= 0.0 {
        return Err(AppError::validation(
            "Frame evidence produced a left-handed basis.",
        ));
    }
    Ok(FittedReconstructionFrame {
        origin_mm,
        x_axis,
        y_axis,
        z_axis,
    })
}

pub fn fit_named_axis(points: &[[f64; 3]], tolerance_mm: f64) -> AppResult<AxisFit> {
    if points.len() < 2 {
        return Err(AppError::validation(
            "Axis evidence needs at least two landmarks.",
        ));
    }
    validate_points_and_tolerance(points, tolerance_mm, "Axis")?;
    let origin = centroid(points);
    let (values, vectors) = symmetric_eigen(covariance(points, origin));
    let largest = largest_index(values);
    if values[largest] <= VECTOR_EPSILON {
        return Err(AppError::validation(
            "Axis evidence is degenerate: landmarks coincide.",
        ));
    }
    let direction = canonical_direction(column(vectors, largest));
    let residuals = points
        .iter()
        .map(|point| norm(cross(sub(*point, origin), direction)))
        .collect::<Vec<_>>();
    let (rms_mm, max_mm) = residual_summary(&residuals);
    if max_mm > tolerance_mm {
        return Err(AppError::validation(format!(
            "Axis fit residual {max_mm:.6} mm exceeds tolerance {tolerance_mm:.6} mm."
        )));
    }
    Ok(AxisFit {
        origin_mm: origin,
        direction,
        rms_mm,
        max_mm,
    })
}

pub fn fit_named_plane(points: &[[f64; 3]], tolerance_mm: f64) -> AppResult<PlaneFit> {
    if points.len() < 3 {
        return Err(AppError::validation(
            "Plane evidence needs at least three landmarks.",
        ));
    }
    validate_points_and_tolerance(points, tolerance_mm, "Plane")?;
    let origin = centroid(points);
    let (values, vectors) = symmetric_eigen(covariance(points, origin));
    let mut ordered = [0usize, 1, 2];
    ordered.sort_by(|a, b| values[*a].total_cmp(&values[*b]));
    if values[ordered[1]] <= VECTOR_EPSILON {
        return Err(AppError::validation(
            "Plane evidence is degenerate: landmarks are collinear.",
        ));
    }
    let normal = canonical_direction(column(vectors, ordered[0]));
    let residuals = points
        .iter()
        .map(|point| dot(sub(*point, origin), normal).abs())
        .collect::<Vec<_>>();
    let (rms_mm, max_mm) = residual_summary(&residuals);
    if max_mm > tolerance_mm {
        return Err(AppError::validation(format!(
            "Plane fit residual {max_mm:.6} mm exceeds tolerance {tolerance_mm:.6} mm."
        )));
    }
    Ok(PlaneFit {
        origin_mm: origin,
        normal,
        rms_mm,
        max_mm,
    })
}

pub fn fit_circle_primitive(points: &[[f64; 3]], tolerance_mm: f64) -> AppResult<CircleFit> {
    if points.len() < 3 {
        return Err(AppError::validation(
            "Circle evidence needs at least three samples.",
        ));
    }
    validate_points_and_tolerance(points, tolerance_mm, "Circle")?;
    let plane = fit_named_plane(points, tolerance_mm)?;
    let (basis_x, basis_y) = plane_basis(plane.normal)?;
    let projected = points
        .iter()
        .map(|point| {
            let offset = sub(*point, plane.origin_mm);
            [dot(offset, basis_x), dot(offset, basis_y)]
        })
        .collect::<Vec<_>>();
    let mut normal = [[0.0; 3]; 3];
    let mut rhs = [0.0; 3];
    for point in &projected {
        let row = [point[0], point[1], 1.0];
        let target = -(point[0] * point[0] + point[1] * point[1]);
        for i in 0..3 {
            rhs[i] += row[i] * target;
            for j in 0..3 {
                normal[i][j] += row[i] * row[j];
            }
        }
    }
    let solution = solve_linear_system(normal, rhs, "Circle")?;
    let center_2d = [-0.5 * solution[0], -0.5 * solution[1]];
    let radius_squared = center_2d[0] * center_2d[0] + center_2d[1] * center_2d[1] - solution[2];
    if !radius_squared.is_finite() || radius_squared <= VECTOR_EPSILON {
        return Err(AppError::validation(
            "Circle evidence produced a non-positive radius.",
        ));
    }
    let radius_mm = radius_squared.sqrt();
    let center_mm = add(
        plane.origin_mm,
        add(scale(basis_x, center_2d[0]), scale(basis_y, center_2d[1])),
    );
    let residuals = points
        .iter()
        .map(|point| {
            let offset = sub(*point, center_mm);
            let plane_distance = dot(offset, plane.normal);
            let radial = norm(sub(offset, scale(plane.normal, plane_distance)));
            (radial - radius_mm).hypot(plane_distance)
        })
        .collect::<Vec<_>>();
    let (rms_mm, max_mm) = residual_summary(&residuals);
    reject_fit_over_tolerance("Circle", max_mm, tolerance_mm)?;
    Ok(CircleFit {
        center_mm,
        normal: plane.normal,
        radius_mm,
        rms_mm,
        max_mm,
    })
}

pub fn fit_cylinder_primitive(
    points: &[[f64; 3]],
    axis_direction: [f64; 3],
    tolerance_mm: f64,
) -> AppResult<CylinderFit> {
    if points.len() < 6 {
        return Err(AppError::validation(
            "Cylinder evidence needs at least six samples.",
        ));
    }
    validate_points_and_tolerance(points, tolerance_mm, "Cylinder")?;
    let axis_direction = canonical_direction(normalize(axis_direction).ok_or_else(|| {
        AppError::validation("Cylinder axis direction must be finite and non-zero.")
    })?);
    let origin_mm = centroid(points);
    let samples = points
        .iter()
        .map(|point| {
            let offset = sub(*point, origin_mm);
            let axial = dot(offset, axis_direction);
            let radial = norm(sub(offset, scale(axis_direction, axial)));
            (axial, radial)
        })
        .collect::<Vec<_>>();
    let radius_mm = samples.iter().map(|(_, radial)| radial).sum::<f64>() / samples.len() as f64;
    if !radius_mm.is_finite() || radius_mm <= VECTOR_EPSILON {
        return Err(AppError::validation(
            "Cylinder evidence produced a non-positive radius.",
        ));
    }
    let residuals = samples
        .iter()
        .map(|(_, radial)| (radial - radius_mm).abs())
        .collect::<Vec<_>>();
    let (rms_mm, max_mm) = residual_summary(&residuals);
    reject_fit_over_tolerance("Cylinder", max_mm, tolerance_mm)?;
    Ok(CylinderFit {
        origin_mm,
        axis_direction,
        radius_mm,
        min_axis_mm: samples
            .iter()
            .map(|(axial, _)| *axial)
            .fold(f64::INFINITY, f64::min),
        max_axis_mm: samples
            .iter()
            .map(|(axial, _)| *axial)
            .fold(f64::NEG_INFINITY, f64::max),
        rms_mm,
        max_mm,
    })
}

pub fn fit_cone_primitive(
    points: &[[f64; 3]],
    axis_direction: [f64; 3],
    tolerance_mm: f64,
) -> AppResult<ConeFit> {
    if points.len() < 6 {
        return Err(AppError::validation(
            "Cone evidence needs at least six samples.",
        ));
    }
    validate_points_and_tolerance(points, tolerance_mm, "Cone")?;
    let axis_direction =
        canonical_direction(normalize(axis_direction).ok_or_else(|| {
            AppError::validation("Cone axis direction must be finite and non-zero.")
        })?);
    let origin = centroid(points);
    let samples = points
        .iter()
        .map(|point| {
            let offset = sub(*point, origin);
            let axial = dot(offset, axis_direction);
            let radial = norm(sub(offset, scale(axis_direction, axial)));
            (axial, radial)
        })
        .collect::<Vec<_>>();
    let mean_axis = samples.iter().map(|(axis, _)| axis).sum::<f64>() / samples.len() as f64;
    let mean_radius = samples.iter().map(|(_, radius)| radius).sum::<f64>() / samples.len() as f64;
    let denominator = samples
        .iter()
        .map(|(axis, _)| (axis - mean_axis).powi(2))
        .sum::<f64>();
    if denominator <= VECTOR_EPSILON {
        return Err(AppError::validation("Cone evidence has no axial extent."));
    }
    let slope = samples
        .iter()
        .map(|(axis, radius)| (axis - mean_axis) * (radius - mean_radius))
        .sum::<f64>()
        / denominator;
    if !slope.is_finite() || slope.abs() <= VECTOR_EPSILON {
        return Err(AppError::validation(
            "Cone evidence is indistinguishable from a cylinder.",
        ));
    }
    let intercept = mean_radius - slope * mean_axis;
    let apex_axis = -intercept / slope;
    let apex_mm = add(origin, scale(axis_direction, apex_axis));
    let residuals = samples
        .iter()
        .map(|(axis, radius)| (radius - (intercept + slope * axis)).abs())
        .collect::<Vec<_>>();
    let (rms_mm, max_mm) = residual_summary(&residuals);
    reject_fit_over_tolerance("Cone", max_mm, tolerance_mm)?;
    Ok(ConeFit {
        apex_mm,
        axis_direction,
        half_angle_deg: slope.abs().atan().to_degrees(),
        min_axis_mm: samples
            .iter()
            .map(|(axis, _)| axis - apex_axis)
            .fold(f64::INFINITY, f64::min),
        max_axis_mm: samples
            .iter()
            .map(|(axis, _)| axis - apex_axis)
            .fold(f64::NEG_INFINITY, f64::max),
        rms_mm,
        max_mm,
    })
}

pub fn fit_sphere_primitive(points: &[[f64; 3]], tolerance_mm: f64) -> AppResult<SphereFit> {
    if points.len() < 4 {
        return Err(AppError::validation(
            "Sphere evidence needs at least four samples.",
        ));
    }
    validate_points_and_tolerance(points, tolerance_mm, "Sphere")?;
    let mut normal = [[0.0; 4]; 4];
    let mut rhs = [0.0; 4];
    for point in points {
        let row = [2.0 * point[0], 2.0 * point[1], 2.0 * point[2], 1.0];
        let target = dot(*point, *point);
        for i in 0..4 {
            rhs[i] += row[i] * target;
            for j in 0..4 {
                normal[i][j] += row[i] * row[j];
            }
        }
    }
    let solution = solve_linear_system(normal, rhs, "Sphere")?;
    let center_mm = [solution[0], solution[1], solution[2]];
    let radius_squared = dot(center_mm, center_mm) + solution[3];
    if !radius_squared.is_finite() || radius_squared <= VECTOR_EPSILON {
        return Err(AppError::validation(
            "Sphere evidence produced a non-positive radius.",
        ));
    }
    let radius_mm = radius_squared.sqrt();
    let residuals = points
        .iter()
        .map(|point| (norm(sub(*point, center_mm)) - radius_mm).abs())
        .collect::<Vec<_>>();
    let (rms_mm, max_mm) = residual_summary(&residuals);
    reject_fit_over_tolerance("Sphere", max_mm, tolerance_mm)?;
    Ok(SphereFit {
        center_mm,
        radius_mm,
        rms_mm,
        max_mm,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct RobustPrimitiveFit<T> {
    pub fit: T,
    pub excluded_sample_indices: Vec<usize>,
}

fn robust_leave_one_out<T, F, S>(
    points: &[[f64; 3]],
    minimum_samples: usize,
    mut fit: F,
    score: S,
) -> AppResult<RobustPrimitiveFit<T>>
where
    F: FnMut(&[[f64; 3]]) -> AppResult<T>,
    S: Fn(&T) -> (f64, f64),
{
    match fit(points) {
        Ok(fit) => Ok(RobustPrimitiveFit {
            fit,
            excluded_sample_indices: Vec::new(),
        }),
        Err(full_error) if points.len() > minimum_samples => {
            let mut candidates = Vec::new();
            for excluded in 0..points.len() {
                let inliers = points
                    .iter()
                    .enumerate()
                    .filter_map(|(index, point)| (index != excluded).then_some(*point))
                    .collect::<Vec<_>>();
                if let Ok(candidate) = fit(&inliers) {
                    candidates.push((score(&candidate), excluded, candidate));
                }
            }
            candidates.sort_by(|left, right| {
                left.0
                     .0
                    .total_cmp(&right.0 .0)
                    .then_with(|| left.0 .1.total_cmp(&right.0 .1))
                    .then_with(|| left.1.cmp(&right.1))
            });
            let Some((_, excluded, fit)) = candidates.into_iter().next() else {
                return Err(full_error);
            };
            Ok(RobustPrimitiveFit {
                fit,
                excluded_sample_indices: vec![excluded],
            })
        }
        Err(error) => Err(error),
    }
}

pub fn robust_fit_named_axis(
    points: &[[f64; 3]],
    tolerance_mm: f64,
) -> AppResult<RobustPrimitiveFit<AxisFit>> {
    robust_leave_one_out(
        points,
        2,
        |samples| fit_named_axis(samples, tolerance_mm),
        |fit| (fit.max_mm, fit.rms_mm),
    )
}

pub fn robust_fit_named_plane(
    points: &[[f64; 3]],
    tolerance_mm: f64,
) -> AppResult<RobustPrimitiveFit<PlaneFit>> {
    robust_leave_one_out(
        points,
        3,
        |samples| fit_named_plane(samples, tolerance_mm),
        |fit| (fit.max_mm, fit.rms_mm),
    )
}

pub fn robust_fit_circle_primitive(
    points: &[[f64; 3]],
    tolerance_mm: f64,
) -> AppResult<RobustPrimitiveFit<CircleFit>> {
    robust_leave_one_out(
        points,
        3,
        |samples| fit_circle_primitive(samples, tolerance_mm),
        |fit| (fit.max_mm, fit.rms_mm),
    )
}

pub fn robust_fit_cylinder_primitive(
    points: &[[f64; 3]],
    axis_direction: [f64; 3],
    tolerance_mm: f64,
) -> AppResult<RobustPrimitiveFit<CylinderFit>> {
    robust_leave_one_out(
        points,
        6,
        |samples| fit_cylinder_primitive(samples, axis_direction, tolerance_mm),
        |fit| (fit.max_mm, fit.rms_mm),
    )
}

pub fn robust_fit_cone_primitive(
    points: &[[f64; 3]],
    axis_direction: [f64; 3],
    tolerance_mm: f64,
) -> AppResult<RobustPrimitiveFit<ConeFit>> {
    robust_leave_one_out(
        points,
        6,
        |samples| fit_cone_primitive(samples, axis_direction, tolerance_mm),
        |fit| (fit.max_mm, fit.rms_mm),
    )
}

pub fn robust_fit_sphere_primitive(
    points: &[[f64; 3]],
    tolerance_mm: f64,
) -> AppResult<RobustPrimitiveFit<SphereFit>> {
    robust_leave_one_out(
        points,
        4,
        |samples| fit_sphere_primitive(samples, tolerance_mm),
        |fit| (fit.max_mm, fit.rms_mm),
    )
}

fn fit_profile_arc(
    points: &[[f64; 3]],
    support_normal: [f64; 3],
    tolerance_mm: f64,
) -> AppResult<ProfileArcFit> {
    if points.len() < 3 {
        return Err(AppError::validation(
            "Arc evidence needs at least three ordered samples.",
        ));
    }
    validate_points_and_tolerance(points, tolerance_mm, "Arc")?;
    let normal =
        canonical_direction(normalize(support_normal).ok_or_else(|| {
            AppError::validation("Arc support normal must be finite and non-zero.")
        })?);
    let (basis_u, basis_v) = plane_basis(normal)?;
    let origin = centroid(points);
    let coordinates = points
        .iter()
        .map(|point| {
            let offset = sub(*point, origin);
            [dot(offset, basis_u), dot(offset, basis_v)]
        })
        .collect::<Vec<_>>();
    let first = coordinates[0];
    let middle = coordinates[coordinates.len() / 2];
    let last = coordinates[coordinates.len() - 1];
    let denominator = 2.0
        * (first[0] * (middle[1] - last[1])
            + middle[0] * (last[1] - first[1])
            + last[0] * (first[1] - middle[1]));
    if !denominator.is_finite() || denominator.abs() <= VECTOR_EPSILON {
        return Err(AppError::validation(
            "Arc evidence is collinear or ill-conditioned.",
        ));
    }
    let first_squared = first[0] * first[0] + first[1] * first[1];
    let middle_squared = middle[0] * middle[0] + middle[1] * middle[1];
    let last_squared = last[0] * last[0] + last[1] * last[1];
    let center_2d = [
        (first_squared * (middle[1] - last[1])
            + middle_squared * (last[1] - first[1])
            + last_squared * (first[1] - middle[1]))
            / denominator,
        (first_squared * (last[0] - middle[0])
            + middle_squared * (first[0] - last[0])
            + last_squared * (middle[0] - first[0]))
            / denominator,
    ];
    let center_mm = add(
        origin,
        add(scale(basis_u, center_2d[0]), scale(basis_v, center_2d[1])),
    );
    let radii = points
        .iter()
        .map(|point| norm(sub(*point, center_mm)))
        .collect::<Vec<_>>();
    let radius_mm = radii.iter().sum::<f64>() / radii.len() as f64;
    if !radius_mm.is_finite() || radius_mm <= VECTOR_EPSILON {
        return Err(AppError::validation("Arc evidence produced zero radius."));
    }
    let residuals = points
        .iter()
        .zip(&radii)
        .map(|(point, radius)| {
            let radial = (radius - radius_mm).abs();
            let plane = dot(sub(*point, origin), normal).abs();
            radial.hypot(plane)
        })
        .collect::<Vec<_>>();
    let (rms_mm, max_mm) = residual_summary(&residuals);
    reject_fit_over_tolerance("Arc", max_mm, tolerance_mm)?;
    let mut angles = Vec::with_capacity(points.len());
    for point in points {
        let offset = sub(*point, center_mm);
        let raw = dot(offset, basis_v)
            .atan2(dot(offset, basis_u))
            .to_degrees();
        if let Some(previous) = angles.last().copied() {
            let mut unwrapped = raw;
            while unwrapped - previous > 180.0 {
                unwrapped -= 360.0;
            }
            while unwrapped - previous <= -180.0 {
                unwrapped += 360.0;
            }
            angles.push(unwrapped);
        } else {
            angles.push(raw);
        }
    }
    if (angles[angles.len() - 1] - angles[0]).abs() <= 1.0e-9 {
        return Err(AppError::validation(
            "Arc evidence has zero angular domain.",
        ));
    }
    Ok(ProfileArcFit {
        center_mm,
        normal,
        radius_mm,
        start_angle_deg: angles[0],
        end_angle_deg: angles[angles.len() - 1],
        rms_mm,
        max_mm,
    })
}

fn fit_interpolating_profile_spline(
    points: &[[f64; 3]],
    tolerance_mm: f64,
) -> AppResult<ProfileSplineFit> {
    if points.len() < 4 || points.len() > 64 {
        return Err(AppError::validation(
            "Spline evidence needs between four and 64 ordered samples.",
        ));
    }
    validate_points_and_tolerance(points, tolerance_mm, "Spline")?;
    let degree = 3_usize.min(points.len() - 1);
    let mut parameters = vec![0.0; points.len()];
    for index in 1..points.len() {
        parameters[index] = parameters[index - 1] + norm(sub(points[index], points[index - 1]));
    }
    let total = *parameters.last().unwrap_or(&0.0);
    if !total.is_finite() || total <= VECTOR_EPSILON {
        return Err(AppError::validation(
            "Spline evidence has zero chord-length domain.",
        ));
    }
    for parameter in &mut parameters {
        *parameter /= total;
    }
    let control_count = points.len();
    let knot_count = control_count + degree + 1;
    let mut knots = vec![0.0; knot_count];
    for knot in knots.iter_mut().skip(knot_count - degree - 1) {
        *knot = 1.0;
    }
    for interior in 1..=(control_count - degree - 1) {
        knots[interior + degree] =
            parameters[interior..interior + degree].iter().sum::<f64>() / degree as f64;
    }
    let matrix = parameters
        .iter()
        .map(|parameter| {
            (0..control_count)
                .map(|index| bspline_basis(index, degree, *parameter, &knots, control_count))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut controls = vec![[0.0; 3]; control_count];
    for axis in 0..3 {
        let rhs = points.iter().map(|point| point[axis]).collect::<Vec<_>>();
        let solution = solve_dense_system(matrix.clone(), rhs, "Spline")?;
        for (control, value) in controls.iter_mut().zip(solution) {
            control[axis] = value;
        }
    }
    let residuals = parameters
        .iter()
        .zip(points)
        .map(|(parameter, expected)| {
            let observed = controls
                .iter()
                .enumerate()
                .fold([0.0; 3], |sum, (index, control)| {
                    add(
                        sum,
                        scale(
                            *control,
                            bspline_basis(index, degree, *parameter, &knots, control_count),
                        ),
                    )
                });
            norm(sub(observed, *expected))
        })
        .collect::<Vec<_>>();
    let (rms_mm, max_mm) = residual_summary(&residuals);
    reject_fit_over_tolerance("Spline", max_mm, tolerance_mm)?;
    Ok(ProfileSplineFit {
        degree: degree as u32,
        control_points_mm: controls,
        knots,
        rms_mm,
        max_mm,
    })
}

fn bspline_basis(
    index: usize,
    degree: usize,
    parameter: f64,
    knots: &[f64],
    control_count: usize,
) -> f64 {
    if degree == 0 {
        if (knots[index] <= parameter && parameter < knots[index + 1])
            || parameter == 1.0 && index + 1 == control_count
        {
            return 1.0;
        }
        return 0.0;
    }
    let left_denominator = knots[index + degree] - knots[index];
    let right_denominator = knots[index + degree + 1] - knots[index + 1];
    let left = if left_denominator.abs() <= VECTOR_EPSILON {
        0.0
    } else {
        (parameter - knots[index]) / left_denominator
            * bspline_basis(index, degree - 1, parameter, knots, control_count)
    };
    let right = if right_denominator.abs() <= VECTOR_EPSILON {
        0.0
    } else {
        (knots[index + degree + 1] - parameter) / right_denominator
            * bspline_basis(index + 1, degree - 1, parameter, knots, control_count)
    };
    left + right
}

fn solve_dense_system(
    mut matrix: Vec<Vec<f64>>,
    mut rhs: Vec<f64>,
    label: &str,
) -> AppResult<Vec<f64>> {
    let size = rhs.len();
    if matrix.len() != size || matrix.iter().any(|row| row.len() != size) {
        return Err(AppError::internal(format!(
            "{label} dense solve received inconsistent dimensions."
        )));
    }
    for pivot in 0..size {
        let pivot_row = (pivot..size)
            .max_by(|left, right| {
                matrix[*left][pivot]
                    .abs()
                    .total_cmp(&matrix[*right][pivot].abs())
            })
            .unwrap_or(pivot);
        if matrix[pivot_row][pivot].abs() <= VECTOR_EPSILON {
            return Err(AppError::validation(format!(
                "{label} evidence is singular or ill-conditioned."
            )));
        }
        matrix.swap(pivot, pivot_row);
        rhs.swap(pivot, pivot_row);
        let diagonal = matrix[pivot][pivot];
        for column in pivot..size {
            matrix[pivot][column] /= diagonal;
        }
        rhs[pivot] /= diagonal;
        for row in 0..size {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for column in pivot..size {
                matrix[row][column] -= factor * matrix[pivot][column];
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    if rhs.iter().any(|value| !value.is_finite()) {
        return Err(AppError::validation(format!(
            "{label} dense solve produced non-finite controls."
        )));
    }
    Ok(rhs)
}

fn reject_fit_over_tolerance(kind: &str, max_mm: f64, tolerance_mm: f64) -> AppResult<()> {
    if max_mm > tolerance_mm {
        Err(AppError::validation(format!(
            "{kind} fit residual {max_mm:.6} mm exceeds tolerance {tolerance_mm:.6} mm."
        )))
    } else {
        Ok(())
    }
}

fn plane_basis(normal: [f64; 3]) -> AppResult<([f64; 3], [f64; 3])> {
    let reference = if normal[0].abs() <= 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let basis_x = normalize(cross(reference, normal))
        .ok_or_else(|| AppError::validation("Primitive fit could not construct plane basis."))?;
    let basis_y = cross(normal, basis_x);
    Ok((basis_x, basis_y))
}

fn solve_linear_system<const N: usize>(
    mut matrix: [[f64; N]; N],
    mut rhs: [f64; N],
    kind: &str,
) -> AppResult<[f64; N]> {
    for pivot in 0..N {
        let selected = (pivot..N)
            .max_by(|left, right| {
                matrix[*left][pivot]
                    .abs()
                    .total_cmp(&matrix[*right][pivot].abs())
            })
            .unwrap_or(pivot);
        if matrix[selected][pivot].abs() <= VECTOR_EPSILON {
            return Err(AppError::validation(format!(
                "{kind} evidence is degenerate or rank-deficient."
            )));
        }
        matrix.swap(pivot, selected);
        rhs.swap(pivot, selected);
        let divisor = matrix[pivot][pivot];
        for col in pivot..N {
            matrix[pivot][col] /= divisor;
        }
        rhs[pivot] /= divisor;
        for row in 0..N {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for col in pivot..N {
                matrix[row][col] -= factor * matrix[pivot][col];
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    if rhs.iter().any(|value| !value.is_finite()) {
        return Err(AppError::validation(format!(
            "{kind} fit produced non-finite parameters."
        )));
    }
    Ok(rhs)
}

pub fn recompute_guide_geometry_from_stl(
    path: &Path,
    guide: &mut CaptureReconstructionGuide,
) -> AppResult<()> {
    let mut source_positions = HashMap::<String, [f64; 3]>::new();
    let mut source_normals = HashMap::<String, [f64; 3]>::new();
    for landmark in &mut guide.landmarks {
        let validated = validate_surface_anchor_from_stl(path, &landmark.anchor, 1.0e-6)?;
        landmark.anchor.source_position = validated.source_position;
        landmark.anchor.source_normal = validated.source_normal;
        source_positions.insert(landmark.landmark_id.clone(), validated.source_position);
        source_normals.insert(landmark.landmark_id.clone(), validated.source_normal);
    }

    let scale_mm = match &guide.calibration.method {
        CaptureCalibrationMethod::KnownDistance => {
            if guide.calibration.measurements.is_empty() {
                return Err(AppError::validation(
                    "Known-distance calibration needs at least one measurement.",
                ));
            }
            let observations = guide
                .calibration
                .measurements
                .iter()
                .map(|measurement| {
                    let first = source_positions
                        .get(&measurement.first_landmark_id)
                        .ok_or_else(|| {
                            AppError::validation(format!(
                                "Calibration measurement '{}' references missing landmark '{}'.",
                                measurement.measurement_id, measurement.first_landmark_id
                            ))
                        })?;
                    let second = source_positions
                        .get(&measurement.second_landmark_id)
                        .ok_or_else(|| {
                            AppError::validation(format!(
                                "Calibration measurement '{}' references missing landmark '{}'.",
                                measurement.measurement_id, measurement.second_landmark_id
                            ))
                        })?;
                    Ok(KnownDistanceObservation {
                        source_distance: norm(sub(*second, *first)),
                        known_distance_mm: measurement.known_distance_mm,
                    })
                })
                .collect::<AppResult<Vec<_>>>()?;
            let tolerance = guide
                .calibration
                .measurements
                .iter()
                .map(|measurement| measurement.accepted_tolerance_mm)
                .fold(f64::INFINITY, f64::min);
            let fit = fit_known_distance_scale(&observations, tolerance)?;
            for (measurement, residual) in guide
                .calibration
                .measurements
                .iter_mut()
                .zip(&fit.residuals_mm)
            {
                let first = source_positions[&measurement.first_landmark_id];
                let second = source_positions[&measurement.second_landmark_id];
                measurement.fitted_distance_mm =
                    norm(sub(second, first)) * fit.millimetres_per_source_unit;
                measurement.residual_mm = *residual;
            }
            guide.calibration.residual_mm = fit.max_residual_mm;
            fit.millimetres_per_source_unit
        }
        CaptureCalibrationMethod::TrustedMetricMetadata {
            accepted_by_user,
            provenance,
        } => {
            if !accepted_by_user || provenance.trim().is_empty() {
                return Err(AppError::validation(
                    "Trusted metric metadata must include provenance and explicit user acceptance.",
                ));
            }
            let scale = guide.calibration.millimetres_per_source_unit;
            if !scale.is_finite() || scale <= 0.0 {
                return Err(AppError::validation(
                    "Trusted metric metadata scale must be finite and positive.",
                ));
            }
            guide.calibration.residual_mm = 0.0;
            scale
        }
    };
    guide.calibration.millimetres_per_source_unit = scale_mm;

    if guide.reconstruction_frame.source_landmark_ids.len() != 3 {
        return Err(AppError::validation(
            "Reconstruction frame needs origin, X-direction, and XY-plane landmark IDs.",
        ));
    }
    let frame_points = guide
        .reconstruction_frame
        .source_landmark_ids
        .iter()
        .map(|id| {
            source_positions
                .get(id)
                .copied()
                .map(|position| scale(position, scale_mm))
                .ok_or_else(|| {
                    AppError::validation(format!(
                        "Reconstruction frame references missing landmark '{id}'."
                    ))
                })
        })
        .collect::<AppResult<Vec<_>>>()?;
    let frame = construct_reconstruction_frame(frame_points[0], frame_points[1], frame_points[2])?;
    guide.reconstruction_frame.origin_mm = frame.origin_mm;
    guide.reconstruction_frame.x_axis = frame.x_axis;
    guide.reconstruction_frame.y_axis = frame.y_axis;
    guide.reconstruction_frame.z_axis = frame.z_axis;

    let mut local_positions = HashMap::<String, [f64; 3]>::new();
    for landmark in &mut guide.landmarks {
        let world_mm = scale(source_positions[&landmark.landmark_id], scale_mm);
        landmark.local_position_mm = frame.to_local_mm(world_mm);
        let normal = source_normals[&landmark.landmark_id];
        landmark.local_normal = canonical_direction([
            dot(normal, frame.x_axis),
            dot(normal, frame.y_axis),
            dot(normal, frame.z_axis),
        ]);
        local_positions.insert(landmark.landmark_id.clone(), landmark.local_position_mm);
    }
    let mesh = IndexedMeshAsset::from_stl(MeshAssetSource::Imported, path)?;
    let radius_source_units = guide.evidence_computation_policy.neighborhood_radius_mm / scale_mm;
    let max_neighborhood_triangles = usize::try_from(
        guide.evidence_computation_policy.max_neighborhood_triangles,
    )
    .map_err(|_| {
        AppError::validation("Capture surface neighborhood triangle budget is out of range.")
    })?;
    let mut surface_neighborhoods = Vec::with_capacity(guide.landmarks.len());
    for landmark in &mut guide.landmarks {
        let neighborhood = extract_surface_neighborhood_from_mesh(
            &mesh,
            &landmark.landmark_id,
            &landmark.anchor,
            landmark.anchor.source_position,
            radius_source_units,
            max_neighborhood_triangles,
        )?;
        landmark.uncertainty_mm = Some(neighborhood.position_uncertainty_source_units * scale_mm);
        surface_neighborhoods.push(neighborhood);
    }
    guide.surface_neighborhoods = surface_neighborhoods;
    let mut robust_exclusions = HashMap::<String, Vec<String>>::new();
    for axis in &mut guide.axes {
        let points = axis
            .landmark_ids
            .iter()
            .map(|id| {
                local_positions.get(id).copied().ok_or_else(|| {
                    AppError::validation(format!(
                        "Axis '{}' references missing landmark '{id}'.",
                        axis.axis_id
                    ))
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        let robust = robust_fit_named_axis(&points, axis.fit.tolerance_mm)?;
        let excluded = robust
            .excluded_sample_indices
            .iter()
            .map(|index| axis.landmark_ids[*index].clone())
            .collect::<Vec<_>>();
        if !excluded.is_empty() {
            robust_exclusions.insert(format!("primitive:{}", axis.axis_id), excluded);
        }
        let fit = robust.fit;
        axis.origin_mm = fit.origin_mm;
        axis.direction = fit.direction;
        axis.fit.rms_mm = fit.rms_mm;
        axis.fit.max_mm = fit.max_mm;
    }
    for plane in &mut guide.planes {
        let points = plane
            .landmark_ids
            .iter()
            .map(|id| {
                local_positions.get(id).copied().ok_or_else(|| {
                    AppError::validation(format!(
                        "Plane '{}' references missing landmark '{id}'.",
                        plane.plane_id
                    ))
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        let robust = robust_fit_named_plane(&points, plane.fit.tolerance_mm)?;
        let excluded = robust
            .excluded_sample_indices
            .iter()
            .map(|index| plane.landmark_ids[*index].clone())
            .collect::<Vec<_>>();
        if !excluded.is_empty() {
            robust_exclusions.insert(format!("primitive:{}", plane.plane_id), excluded);
        }
        let fit = robust.fit;
        plane.origin_mm = fit.origin_mm;
        plane.normal = fit.normal;
        plane.fit.rms_mm = fit.rms_mm;
        plane.fit.max_mm = fit.max_mm;
    }
    let neighborhoods_by_landmark = guide
        .surface_neighborhoods
        .iter()
        .map(|item| (item.landmark_id.as_str(), item))
        .collect::<HashMap<_, _>>();
    let candidate_evidence = |landmark_ids: &[String]| {
        let neighborhoods = landmark_ids
            .iter()
            .filter_map(|id| neighborhoods_by_landmark.get(id.as_str()).copied())
            .collect::<Vec<_>>();
        let neighborhood_ids = neighborhoods
            .iter()
            .map(|item| item.neighborhood_id.clone())
            .collect::<Vec<_>>();
        let support_sample_count = neighborhoods
            .iter()
            .map(|item| item.sample_count)
            .sum::<u64>();
        (neighborhood_ids, support_sample_count)
    };
    let mut primitive_candidates =
        Vec::with_capacity(guide.axes.len() + guide.planes.len() + guide.profiles.len());
    let mut rejected_primitive_hypotheses = Vec::new();
    for axis in &guide.axes {
        let (neighborhood_ids, support_sample_count) = candidate_evidence(&axis.landmark_ids);
        primitive_candidates.push(crate::contracts::CapturePrimitiveCandidate {
            candidate_id: format!("primitive:{}", axis.axis_id),
            label: axis.label.clone(),
            guide_item_ids: vec![axis.axis_id.clone()],
            neighborhood_ids,
            geometry: crate::contracts::CaptureAnalyticPrimitive::Line {
                origin_mm: axis.origin_mm,
                direction: axis.direction,
            },
            fit: axis.fit.clone(),
            support_sample_count,
        });
    }
    for plane in &guide.planes {
        let (neighborhood_ids, support_sample_count) = candidate_evidence(&plane.landmark_ids);
        primitive_candidates.push(crate::contracts::CapturePrimitiveCandidate {
            candidate_id: format!("primitive:{}", plane.plane_id),
            label: plane.label.clone(),
            guide_item_ids: vec![plane.plane_id.clone()],
            neighborhood_ids,
            geometry: crate::contracts::CaptureAnalyticPrimitive::Plane {
                origin_mm: plane.origin_mm,
                normal: plane.normal,
            },
            fit: plane.fit.clone(),
            support_sample_count,
        });
    }
    for profile in &guide.profiles {
        let tolerance_mm = guide
            .planes
            .iter()
            .find(|plane| plane.plane_id == profile.support_plane_id)
            .map(|plane| plane.fit.tolerance_mm)
            .unwrap_or(0.1);
        let points = profile
            .landmark_ids
            .iter()
            .filter_map(|id| local_positions.get(id).copied())
            .collect::<Vec<_>>();
        if points.len() >= 4 {
            let robust = match robust_fit_circle_primitive(&points, tolerance_mm) {
                Ok(circle) => circle,
                Err(error) => {
                    rejected_primitive_hypotheses.push(
                        crate::contracts::CapturePrimitiveHypothesis {
                            hypothesis_id: format!(
                                "hypothesis:{}:circle:rejected",
                                profile.profile_id
                            ),
                            guide_item_ids: vec![profile.profile_id.clone()],
                            kind: crate::contracts::CapturePrimitiveKind::Circle,
                            status: crate::contracts::CapturePrimitiveHypothesisStatus::Rejected,
                            candidate_id: None,
                            domain: crate::contracts::CapturePrimitiveFitDomain {
                                parameter_name: Some("angleDeg".into()),
                                minimum: Some(0.0),
                                maximum: Some(360.0),
                                observed_only: true,
                            },
                            fit: None,
                            robust_evidence: None,
                            reason: error.message,
                        },
                    );
                    continue;
                }
            };
            let circle = robust.fit;
            let candidate_id = format!("primitive:{}:circle", profile.profile_id);
            let excluded = robust
                .excluded_sample_indices
                .iter()
                .map(|index| profile.landmark_ids[*index].clone())
                .collect::<Vec<_>>();
            if !excluded.is_empty() {
                robust_exclusions.insert(candidate_id.clone(), excluded);
            }
            let (neighborhood_ids, support_sample_count) =
                candidate_evidence(&profile.landmark_ids);
            primitive_candidates.push(crate::contracts::CapturePrimitiveCandidate {
                candidate_id,
                label: format!("{} circle", profile.label),
                guide_item_ids: vec![profile.profile_id.clone()],
                neighborhood_ids,
                geometry: crate::contracts::CaptureAnalyticPrimitive::Circle {
                    center_mm: circle.center_mm,
                    normal: circle.normal,
                    radius_mm: circle.radius_mm,
                },
                fit: crate::contracts::CaptureFitResidual {
                    rms_mm: circle.rms_mm,
                    max_mm: circle.max_mm,
                    tolerance_mm,
                },
                support_sample_count,
            });
        }
    }
    let bore_landmark_ids = guide
        .landmarks
        .iter()
        .filter(|landmark| landmark.role == crate::contracts::CaptureLandmarkRole::BoreSample)
        .map(|landmark| landmark.landmark_id.clone())
        .collect::<Vec<_>>();
    if bore_landmark_ids.len() >= 6 {
        let bore_points = bore_landmark_ids
            .iter()
            .filter_map(|id| local_positions.get(id).copied())
            .collect::<Vec<_>>();
        for axis in &guide.axes {
            match robust_fit_cylinder_primitive(&bore_points, axis.direction, axis.fit.tolerance_mm)
            {
                Ok(robust) => {
                    let cylinder = robust.fit;
                    let candidate_id = format!("primitive:bore:{}:cylinder", axis.axis_id);
                    let excluded = robust
                        .excluded_sample_indices
                        .iter()
                        .map(|index| bore_landmark_ids[*index].clone())
                        .collect::<Vec<_>>();
                    if !excluded.is_empty() {
                        robust_exclusions.insert(candidate_id.clone(), excluded);
                    }
                    let (neighborhood_ids, support_sample_count) =
                        candidate_evidence(&bore_landmark_ids);
                    primitive_candidates.push(crate::contracts::CapturePrimitiveCandidate {
                        candidate_id,
                        label: format!("Bore around {}", axis.label),
                        guide_item_ids: std::iter::once(axis.axis_id.clone())
                            .chain(bore_landmark_ids.iter().cloned())
                            .collect(),
                        neighborhood_ids,
                        geometry: crate::contracts::CaptureAnalyticPrimitive::Cylinder {
                            origin_mm: cylinder.origin_mm,
                            axis_direction: cylinder.axis_direction,
                            radius_mm: cylinder.radius_mm,
                            min_axis_mm: cylinder.min_axis_mm,
                            max_axis_mm: cylinder.max_axis_mm,
                        },
                        fit: crate::contracts::CaptureFitResidual {
                            rms_mm: cylinder.rms_mm,
                            max_mm: cylinder.max_mm,
                            tolerance_mm: axis.fit.tolerance_mm,
                        },
                        support_sample_count,
                    });
                }
                Err(error) => rejected_primitive_hypotheses.push(
                    crate::contracts::CapturePrimitiveHypothesis {
                        hypothesis_id: format!(
                            "hypothesis:bore:{}:cylinder:rejected",
                            axis.axis_id
                        ),
                        guide_item_ids: std::iter::once(axis.axis_id.clone())
                            .chain(bore_landmark_ids.iter().cloned())
                            .collect(),
                        kind: crate::contracts::CapturePrimitiveKind::Cylinder,
                        status: crate::contracts::CapturePrimitiveHypothesisStatus::Rejected,
                        candidate_id: None,
                        domain: crate::contracts::CapturePrimitiveFitDomain {
                            parameter_name: Some("axisMm".into()),
                            minimum: None,
                            maximum: None,
                            observed_only: true,
                        },
                        fit: None,
                        robust_evidence: None,
                        reason: error.message,
                    },
                ),
            }
        }
    }
    let surface_landmark_ids = guide
        .landmarks
        .iter()
        .filter(|landmark| {
            landmark.role == crate::contracts::CaptureLandmarkRole::MatingSurfaceSample
        })
        .map(|landmark| landmark.landmark_id.clone())
        .collect::<Vec<_>>();
    if surface_landmark_ids.len() >= 4 {
        let surface_points = surface_landmark_ids
            .iter()
            .filter_map(|id| local_positions.get(id).copied())
            .collect::<Vec<_>>();
        let tolerance_mm = guide
            .planes
            .iter()
            .map(|plane| plane.fit.tolerance_mm)
            .chain(guide.axes.iter().map(|axis| axis.fit.tolerance_mm))
            .fold(0.1_f64, f64::min);
        match robust_fit_sphere_primitive(&surface_points, tolerance_mm) {
            Ok(robust) => {
                let sphere = robust.fit;
                let candidate_id = "primitive:mating-surface:sphere".to_string();
                let excluded = robust
                    .excluded_sample_indices
                    .iter()
                    .map(|index| surface_landmark_ids[*index].clone())
                    .collect::<Vec<_>>();
                if !excluded.is_empty() {
                    robust_exclusions.insert(candidate_id.clone(), excluded);
                }
                let (neighborhood_ids, support_sample_count) =
                    candidate_evidence(&surface_landmark_ids);
                primitive_candidates.push(crate::contracts::CapturePrimitiveCandidate {
                    candidate_id,
                    label: "Mating surface sphere".into(),
                    guide_item_ids: surface_landmark_ids.clone(),
                    neighborhood_ids,
                    geometry: crate::contracts::CaptureAnalyticPrimitive::Sphere {
                        center_mm: sphere.center_mm,
                        radius_mm: sphere.radius_mm,
                    },
                    fit: crate::contracts::CaptureFitResidual {
                        rms_mm: sphere.rms_mm,
                        max_mm: sphere.max_mm,
                        tolerance_mm,
                    },
                    support_sample_count,
                });
            }
            Err(error) => {
                rejected_primitive_hypotheses.push(crate::contracts::CapturePrimitiveHypothesis {
                    hypothesis_id: "hypothesis:mating-surface:sphere:rejected".into(),
                    guide_item_ids: surface_landmark_ids.clone(),
                    kind: crate::contracts::CapturePrimitiveKind::Sphere,
                    status: crate::contracts::CapturePrimitiveHypothesisStatus::Rejected,
                    candidate_id: None,
                    domain: crate::contracts::CapturePrimitiveFitDomain {
                        parameter_name: None,
                        minimum: None,
                        maximum: None,
                        observed_only: true,
                    },
                    fit: None,
                    robust_evidence: None,
                    reason: error.message,
                })
            }
        }
        if surface_points.len() >= 6 {
            for axis in &guide.axes {
                match robust_fit_cone_primitive(&surface_points, axis.direction, tolerance_mm) {
                    Ok(robust) => {
                        let cone = robust.fit;
                        let candidate_id =
                            format!("primitive:mating-surface:{}:cone", axis.axis_id);
                        let excluded = robust
                            .excluded_sample_indices
                            .iter()
                            .map(|index| surface_landmark_ids[*index].clone())
                            .collect::<Vec<_>>();
                        if !excluded.is_empty() {
                            robust_exclusions.insert(candidate_id.clone(), excluded);
                        }
                        let (neighborhood_ids, support_sample_count) =
                            candidate_evidence(&surface_landmark_ids);
                        primitive_candidates.push(crate::contracts::CapturePrimitiveCandidate {
                            candidate_id,
                            label: format!("Mating cone around {}", axis.label),
                            guide_item_ids: std::iter::once(axis.axis_id.clone())
                                .chain(surface_landmark_ids.iter().cloned())
                                .collect(),
                            neighborhood_ids,
                            geometry: crate::contracts::CaptureAnalyticPrimitive::Cone {
                                apex_mm: cone.apex_mm,
                                axis_direction: cone.axis_direction,
                                half_angle_deg: cone.half_angle_deg,
                                min_axis_mm: cone.min_axis_mm,
                                max_axis_mm: cone.max_axis_mm,
                            },
                            fit: crate::contracts::CaptureFitResidual {
                                rms_mm: cone.rms_mm,
                                max_mm: cone.max_mm,
                                tolerance_mm,
                            },
                            support_sample_count,
                        });
                    }
                    Err(error) => rejected_primitive_hypotheses.push(
                        crate::contracts::CapturePrimitiveHypothesis {
                            hypothesis_id: format!(
                                "hypothesis:mating-surface:{}:cone:rejected",
                                axis.axis_id
                            ),
                            guide_item_ids: std::iter::once(axis.axis_id.clone())
                                .chain(surface_landmark_ids.iter().cloned())
                                .collect(),
                            kind: crate::contracts::CapturePrimitiveKind::Cone,
                            status: crate::contracts::CapturePrimitiveHypothesisStatus::Rejected,
                            candidate_id: None,
                            domain: crate::contracts::CapturePrimitiveFitDomain {
                                parameter_name: Some("axisMm".into()),
                                minimum: None,
                                maximum: None,
                                observed_only: true,
                            },
                            fit: None,
                            robust_evidence: None,
                            reason: error.message,
                        },
                    ),
                }
            }
        }
    }
    let mut primitive_hypotheses = primitive_candidates
        .iter()
        .map(|candidate| {
            let (kind, domain) = match &candidate.geometry {
                crate::contracts::CaptureAnalyticPrimitive::Line {
                    origin_mm,
                    direction,
                } => {
                    let extents = candidate
                        .guide_item_ids
                        .first()
                        .and_then(|axis_id| guide.axes.iter().find(|axis| axis.axis_id == *axis_id))
                        .map(|axis| {
                            axis.landmark_ids
                                .iter()
                                .filter_map(|id| local_positions.get(id))
                                .map(|point| dot(sub(*point, *origin_mm), *direction))
                                .fold(
                                    (f64::INFINITY, f64::NEG_INFINITY),
                                    |(minimum, maximum), value| {
                                        (minimum.min(value), maximum.max(value))
                                    },
                                )
                        });
                    (
                        crate::contracts::CapturePrimitiveKind::Line,
                        crate::contracts::CapturePrimitiveFitDomain {
                            parameter_name: Some("distanceMm".into()),
                            minimum: extents.map(|value| value.0),
                            maximum: extents.map(|value| value.1),
                            observed_only: true,
                        },
                    )
                }
                crate::contracts::CaptureAnalyticPrimitive::Plane { .. } => (
                    crate::contracts::CapturePrimitiveKind::Plane,
                    crate::contracts::CapturePrimitiveFitDomain {
                        parameter_name: None,
                        minimum: None,
                        maximum: None,
                        observed_only: true,
                    },
                ),
                crate::contracts::CaptureAnalyticPrimitive::Circle { .. } => (
                    crate::contracts::CapturePrimitiveKind::Circle,
                    crate::contracts::CapturePrimitiveFitDomain {
                        parameter_name: Some("angleDeg".into()),
                        minimum: Some(0.0),
                        maximum: Some(360.0),
                        observed_only: true,
                    },
                ),
                crate::contracts::CaptureAnalyticPrimitive::Cylinder {
                    min_axis_mm,
                    max_axis_mm,
                    ..
                }
                | crate::contracts::CaptureAnalyticPrimitive::Cone {
                    min_axis_mm,
                    max_axis_mm,
                    ..
                } => (
                    if matches!(
                        &candidate.geometry,
                        crate::contracts::CaptureAnalyticPrimitive::Cylinder { .. }
                    ) {
                        crate::contracts::CapturePrimitiveKind::Cylinder
                    } else {
                        crate::contracts::CapturePrimitiveKind::Cone
                    },
                    crate::contracts::CapturePrimitiveFitDomain {
                        parameter_name: Some("axisMm".into()),
                        minimum: Some(*min_axis_mm),
                        maximum: Some(*max_axis_mm),
                        observed_only: true,
                    },
                ),
                crate::contracts::CaptureAnalyticPrimitive::Sphere { .. } => (
                    crate::contracts::CapturePrimitiveKind::Sphere,
                    crate::contracts::CapturePrimitiveFitDomain {
                        parameter_name: None,
                        minimum: None,
                        maximum: None,
                        observed_only: true,
                    },
                ),
            };
            crate::contracts::CapturePrimitiveHypothesis {
                hypothesis_id: format!("hypothesis:{}:supported", candidate.candidate_id),
                guide_item_ids: candidate.guide_item_ids.clone(),
                kind,
                status: crate::contracts::CapturePrimitiveHypothesisStatus::Supported,
                candidate_id: Some(candidate.candidate_id.clone()),
                domain,
                fit: Some(candidate.fit.clone()),
                robust_evidence: robust_exclusions.get(&candidate.candidate_id).map(|excluded| {
                    crate::contracts::CapturePrimitiveRobustEvidence {
                        method: "deterministicLeaveOneOut".into(),
                        excluded_guide_item_ids: excluded.clone(),
                    }
                }),
                reason: if robust_exclusions.contains_key(&candidate.candidate_id) {
                    "Residual-bounded deterministic fit is supported after explicit robust outlier exclusion."
                        .into()
                } else {
                    "Residual-bounded deterministic fit is supported by current evidence.".into()
                },
            }
        })
        .collect::<Vec<_>>();
    primitive_hypotheses.extend(rejected_primitive_hypotheses);
    primitive_hypotheses.sort_by(|left, right| left.hypothesis_id.cmp(&right.hypothesis_id));
    guide.primitive_candidates = primitive_candidates;
    guide.primitive_hypotheses = primitive_hypotheses;
    compute_deterministic_reconstruction_stack(&mesh, guide, &local_positions)?;
    guide.validate().map_err(AppError::validation)
}

fn compute_deterministic_reconstruction_stack(
    mesh: &IndexedMeshAsset,
    guide: &mut CaptureReconstructionGuide,
    local_positions: &HashMap<String, [f64; 3]>,
) -> AppResult<()> {
    let (surface_regions, region_adjacency) = segment_capture_surface(mesh, guide)?;
    guide.surface_regions = surface_regions;
    guide.region_adjacency = region_adjacency;
    guide.reconstructed_profiles = reconstruct_profiles(guide, local_positions)?;
    guide.constraint_graph = build_capture_constraint_graph(guide)?;
    guide.feature_plan_candidates = build_feature_plan_candidates(guide);
    let supported = guide
        .feature_plan_candidates
        .iter()
        .filter(|plan| plan.status == crate::contracts::CaptureFeaturePlanStatus::Supported)
        .map(|plan| plan.plan_id.clone())
        .collect::<Vec<_>>();
    if guide
        .selected_feature_plan_id
        .as_ref()
        .is_some_and(|selected| !supported.contains(selected))
    {
        guide.selected_feature_plan_id = None;
    }
    if guide.selected_feature_plan_id.is_none() && supported.len() == 1 {
        guide.selected_feature_plan_id = supported.first().cloned();
    }
    guide.reconstruction_readiness = evaluate_reconstruction_readiness(guide);
    Ok(())
}

fn segment_capture_surface(
    mesh: &IndexedMeshAsset,
    guide: &CaptureReconstructionGuide,
) -> AppResult<(
    Vec<crate::contracts::CaptureSurfaceRegion>,
    Vec<crate::contracts::CaptureRegionAdjacency>,
)> {
    let vertices = mesh.vertices();
    let triangles = mesh.triangles();
    if triangles.is_empty() {
        return Err(AppError::validation(
            "Capture segmentation needs at least one source triangle.",
        ));
    }
    let mut edge_owners = BTreeMap::<(u32, u32), Vec<usize>>::new();
    let mut normals = Vec::with_capacity(triangles.len());
    let mut areas = Vec::with_capacity(triangles.len());
    for (index, triangle) in triangles.iter().enumerate() {
        for edge in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            let key = if edge.0 < edge.1 {
                edge
            } else {
                (edge.1, edge.0)
            };
            edge_owners.entry(key).or_default().push(index);
        }
        let a = vertices[triangle[0] as usize];
        let b = vertices[triangle[1] as usize];
        let c = vertices[triangle[2] as usize];
        let cross_value = cross(sub(b, a), sub(c, a));
        let magnitude = norm(cross_value);
        if !magnitude.is_finite() || magnitude <= VECTOR_EPSILON {
            return Err(AppError::validation(format!(
                "Capture segmentation found degenerate triangle {index}."
            )));
        }
        normals.push(scale(cross_value, 1.0 / magnitude));
        areas.push(0.5 * magnitude);
    }
    let mut neighbours = vec![Vec::<usize>::new(); triangles.len()];
    for owners in edge_owners.values() {
        if owners.len() == 2 {
            neighbours[owners[0]].push(owners[1]);
            neighbours[owners[1]].push(owners[0]);
        }
    }
    for row in &mut neighbours {
        row.sort_unstable();
        row.dedup();
    }
    let neighborhoods = guide
        .surface_neighborhoods
        .iter()
        .map(|item| (item.neighborhood_id.as_str(), item))
        .collect::<HashMap<_, _>>();
    let ignored_landmarks = guide
        .ignored_regions
        .iter()
        .flat_map(|region| region.landmark_ids.iter().map(String::as_str))
        .collect::<HashSet<_>>();
    let ignored_triangles = guide
        .surface_neighborhoods
        .iter()
        .filter(|item| ignored_landmarks.contains(item.landmark_id.as_str()))
        .flat_map(|item| item.triangle_indices.iter().copied())
        .collect::<HashSet<_>>();
    let is_ignored = |triangle_index: usize| ignored_triangles.contains(&(triangle_index as u64));
    let mut region_for_triangle = vec![usize::MAX; triangles.len()];
    let mut components = Vec::<Vec<usize>>::new();
    const SMOOTH_ANGLE_DEG: f64 = 20.0;
    for seed in 0..triangles.len() {
        if region_for_triangle[seed] != usize::MAX {
            continue;
        }
        let region_index = components.len();
        let mut queue = VecDeque::from([seed]);
        region_for_triangle[seed] = region_index;
        let mut component = Vec::new();
        while let Some(current) = queue.pop_front() {
            component.push(current);
            for &next in &neighbours[current] {
                if region_for_triangle[next] != usize::MAX || is_ignored(next) != is_ignored(seed) {
                    continue;
                }
                let cosine = dot(normals[current], normals[next]).clamp(-1.0, 1.0);
                if cosine.acos().to_degrees() <= SMOOTH_ANGLE_DEG {
                    region_for_triangle[next] = region_index;
                    queue.push_back(next);
                }
            }
        }
        component.sort_unstable();
        components.push(component);
    }
    let mut regions = Vec::with_capacity(components.len());
    for (region_index, component) in components.iter().enumerate() {
        let triangle_set = component
            .iter()
            .map(|index| *index as u64)
            .collect::<HashSet<_>>();
        let mut primitive_candidate_ids = guide
            .primitive_candidates
            .iter()
            .filter(|candidate| {
                candidate.neighborhood_ids.iter().any(|id| {
                    neighborhoods.get(id.as_str()).is_some_and(|neighborhood| {
                        neighborhood
                            .triangle_indices
                            .iter()
                            .any(|index| triangle_set.contains(index))
                    })
                })
            })
            .map(|candidate| candidate.candidate_id.clone())
            .collect::<Vec<_>>();
        primitive_candidate_ids.sort();
        primitive_candidate_ids.dedup();
        let mut landmark_ids = guide
            .surface_neighborhoods
            .iter()
            .filter(|neighborhood| {
                neighborhood
                    .triangle_indices
                    .iter()
                    .any(|index| triangle_set.contains(index))
            })
            .map(|neighborhood| neighborhood.landmark_id.clone())
            .collect::<Vec<_>>();
        landmark_ids.sort();
        landmark_ids.dedup();
        let ignored = component.iter().all(|index| is_ignored(*index));
        let kind = if ignored {
            crate::contracts::CaptureSurfaceRegionKind::IgnoredDamage
        } else {
            primitive_candidate_ids
                .iter()
                .filter_map(|id| {
                    guide
                        .primitive_candidates
                        .iter()
                        .find(|candidate| candidate.candidate_id == *id)
                })
                .map(|candidate| match candidate.geometry {
                    crate::contracts::CaptureAnalyticPrimitive::Plane { .. } => {
                        crate::contracts::CaptureSurfaceRegionKind::Plane
                    }
                    crate::contracts::CaptureAnalyticPrimitive::Cylinder { .. } => {
                        crate::contracts::CaptureSurfaceRegionKind::Cylinder
                    }
                    crate::contracts::CaptureAnalyticPrimitive::Cone { .. } => {
                        crate::contracts::CaptureSurfaceRegionKind::Cone
                    }
                    crate::contracts::CaptureAnalyticPrimitive::Sphere { .. } => {
                        crate::contracts::CaptureSurfaceRegionKind::Sphere
                    }
                    _ => crate::contracts::CaptureSurfaceRegionKind::Freeform,
                })
                .find(|kind| *kind != crate::contracts::CaptureSurfaceRegionKind::Freeform)
                .unwrap_or(crate::contracts::CaptureSurfaceRegionKind::Freeform)
        };
        let boundary_edge_count = edge_owners
            .values()
            .filter(|owners| {
                owners.iter().any(|owner| component.contains(owner))
                    && (owners.len() == 1 || owners.iter().any(|owner| !component.contains(owner)))
            })
            .count() as u64;
        regions.push(crate::contracts::CaptureSurfaceRegion {
            region_id: format!("region:{region_index}"),
            source_mesh_content_digest: guide.source_mesh.content_digest.clone(),
            triangle_indices: component.iter().map(|index| *index as u64).collect(),
            landmark_ids,
            primitive_candidate_ids,
            kind,
            area_source_units_squared: component.iter().map(|index| areas[*index]).sum(),
            boundary_edge_count,
            ignored,
        });
    }
    let mut adjacency_accumulator = BTreeMap::<(usize, usize), (u64, f64)>::new();
    for owners in edge_owners.values().filter(|owners| owners.len() == 2) {
        let first = region_for_triangle[owners[0]];
        let second = region_for_triangle[owners[1]];
        if first == second {
            continue;
        }
        let key = if first < second {
            (first, second)
        } else {
            (second, first)
        };
        let angle = dot(normals[owners[0]], normals[owners[1]])
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees();
        let entry = adjacency_accumulator.entry(key).or_insert((0, 0.0));
        entry.0 += 1;
        entry.1 = entry.1.max(angle);
    }
    let adjacency = adjacency_accumulator
        .into_iter()
        .map(
            |((first, second), (shared_edge_count, maximum_normal_angle_deg))| {
                crate::contracts::CaptureRegionAdjacency {
                    first_region_id: format!("region:{first}"),
                    second_region_id: format!("region:{second}"),
                    shared_edge_count,
                    relation: if maximum_normal_angle_deg <= SMOOTH_ANGLE_DEG {
                        crate::contracts::CaptureRegionRelation::Smooth
                    } else {
                        crate::contracts::CaptureRegionRelation::Sharp
                    },
                    maximum_normal_angle_deg,
                }
            },
        )
        .collect();
    Ok((regions, adjacency))
}

fn reconstruct_profiles(
    guide: &CaptureReconstructionGuide,
    local_positions: &HashMap<String, [f64; 3]>,
) -> AppResult<Vec<crate::contracts::CaptureReconstructedProfile>> {
    let neighborhoods = guide
        .surface_neighborhoods
        .iter()
        .map(|item| (item.landmark_id.as_str(), item.neighborhood_id.as_str()))
        .collect::<HashMap<_, _>>();
    let planes = guide
        .planes
        .iter()
        .map(|plane| (plane.plane_id.as_str(), plane))
        .collect::<HashMap<_, _>>();
    let mut output = Vec::with_capacity(guide.profiles.len());
    for profile in &guide.profiles {
        let points = profile
            .landmark_ids
            .iter()
            .map(|id| {
                local_positions.get(id).copied().ok_or_else(|| {
                    AppError::validation(format!(
                        "Profile '{}' references missing local landmark '{id}'.",
                        profile.profile_id
                    ))
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        let plane = planes
            .get(profile.support_plane_id.as_str())
            .ok_or_else(|| {
                AppError::validation(format!(
                    "Profile '{}' references missing support plane '{}'.",
                    profile.profile_id, profile.support_plane_id
                ))
            })?;
        let support_plane_max_mm = points
            .iter()
            .map(|point| dot(sub(*point, plane.origin_mm), plane.normal).abs())
            .fold(0.0_f64, f64::max);
        if support_plane_max_mm > plane.fit.tolerance_mm {
            return Err(AppError::validation(format!(
                "Profile '{}' leaves support plane by {} mm; tolerance is {} mm.",
                profile.profile_id, support_plane_max_mm, plane.fit.tolerance_mm
            )));
        }
        let closed = profile.kind == crate::contracts::CaptureProfileKind::Closed;
        let fit_role = profile
            .fit_role
            .as_deref()
            .unwrap_or("line")
            .trim()
            .to_ascii_lowercase();
        let all_neighborhood_ids = profile
            .landmark_ids
            .iter()
            .filter_map(|id| neighborhoods.get(id.as_str()).copied())
            .map(str::to_string)
            .collect::<Vec<_>>();
        let (candidate_suffix, segments, rejected_hypotheses) = match fit_role.as_str() {
            "circle" => {
                if !closed {
                    return Err(AppError::validation(format!(
                        "Profile '{}' requests circle fit but is open.",
                        profile.profile_id
                    )));
                }
                let circle = fit_circle_primitive(&points, plane.fit.tolerance_mm)?;
                (
                    "circle",
                    vec![crate::contracts::CaptureProfileSegment {
                        segment_id: format!("profile:{}:circle", profile.profile_id),
                        source_landmark_ids: profile.landmark_ids.clone(),
                        neighborhood_ids: all_neighborhood_ids.clone(),
                        parameter_range: [0.0, 360.0],
                        geometry: crate::contracts::CaptureProfileSegmentGeometry::Circle {
                            center_mm: circle.center_mm,
                            normal: circle.normal,
                            radius_mm: circle.radius_mm,
                        },
                        fit: crate::contracts::CaptureFitResidual {
                            rms_mm: circle.rms_mm,
                            max_mm: circle.max_mm,
                            tolerance_mm: plane.fit.tolerance_mm,
                        },
                    }],
                    vec!["Line and spline hypotheses were not requested by the named profile fit role.".into()],
                )
            }
            "arc" => {
                if closed {
                    return Err(AppError::validation(format!(
                        "Profile '{}' requests arc fit but is closed.",
                        profile.profile_id
                    )));
                }
                let arc = fit_profile_arc(&points, plane.normal, plane.fit.tolerance_mm)?;
                (
                    "arc",
                    vec![crate::contracts::CaptureProfileSegment {
                        segment_id: format!("profile:{}:arc", profile.profile_id),
                        source_landmark_ids: profile.landmark_ids.clone(),
                        neighborhood_ids: all_neighborhood_ids.clone(),
                        parameter_range: [arc.start_angle_deg, arc.end_angle_deg],
                        geometry: crate::contracts::CaptureProfileSegmentGeometry::Arc {
                            center_mm: arc.center_mm,
                            normal: arc.normal,
                            radius_mm: arc.radius_mm,
                            start_angle_deg: arc.start_angle_deg,
                            end_angle_deg: arc.end_angle_deg,
                        },
                        fit: crate::contracts::CaptureFitResidual {
                            rms_mm: arc.rms_mm,
                            max_mm: arc.max_mm,
                            tolerance_mm: plane.fit.tolerance_mm,
                        },
                    }],
                    vec![
                        "Circle hypothesis rejected because the profile is explicitly open.".into(),
                    ],
                )
            }
            "spline" => {
                if closed {
                    return Err(AppError::validation(format!(
                        "Profile '{}' requests open interpolating spline but is closed.",
                        profile.profile_id
                    )));
                }
                let spline = fit_interpolating_profile_spline(&points, plane.fit.tolerance_mm)?;
                (
                    "spline",
                    vec![crate::contracts::CaptureProfileSegment {
                        segment_id: format!("profile:{}:spline", profile.profile_id),
                        source_landmark_ids: profile.landmark_ids.clone(),
                        neighborhood_ids: all_neighborhood_ids.clone(),
                        parameter_range: [0.0, 1.0],
                        geometry: crate::contracts::CaptureProfileSegmentGeometry::Spline {
                            degree: spline.degree,
                            control_points_mm: spline.control_points_mm,
                            knots: spline.knots,
                        },
                        fit: crate::contracts::CaptureFitResidual {
                            rms_mm: spline.rms_mm,
                            max_mm: spline.max_mm,
                            tolerance_mm: plane.fit.tolerance_mm,
                        },
                    }],
                    vec!["Line, arc, and circle hypotheses were not selected by the explicit spline fit role.".into()],
                )
            }
            "line" | "polyline" | "" => {
                let segment_count = if closed {
                    points.len()
                } else {
                    points.len().saturating_sub(1)
                };
                let mut segments = Vec::with_capacity(segment_count);
                for index in 0..segment_count {
                    let next = (index + 1) % points.len();
                    segments.push(crate::contracts::CaptureProfileSegment {
                        segment_id: format!("profile:{}:segment:{index}", profile.profile_id),
                        source_landmark_ids: vec![
                            profile.landmark_ids[index].clone(),
                            profile.landmark_ids[next].clone(),
                        ],
                        neighborhood_ids: [
                            neighborhoods.get(profile.landmark_ids[index].as_str()),
                            neighborhoods.get(profile.landmark_ids[next].as_str()),
                        ]
                        .into_iter()
                        .flatten()
                        .map(|id| (*id).to_string())
                        .collect(),
                        parameter_range: [index as f64, (index + 1) as f64],
                        geometry: crate::contracts::CaptureProfileSegmentGeometry::Line {
                            start_mm: points[index],
                            end_mm: points[next],
                        },
                        fit: crate::contracts::CaptureFitResidual {
                            rms_mm: 0.0,
                            max_mm: 0.0,
                            tolerance_mm: plane.fit.tolerance_mm,
                        },
                    });
                }
                (
                    "polyline",
                    segments,
                    vec![
                        "No curved segment promoted without an explicit fit role and residual-bounded fit."
                            .into(),
                    ],
                )
            }
            unsupported => {
                return Err(AppError::validation(format!(
                    "Profile '{}' requests unsupported fit role '{}'; expected line, arc, circle, or spline.",
                    profile.profile_id, unsupported
                )));
            }
        };
        let closure_error_mm = if closed {
            0.0
        } else {
            norm(sub(points[0], points[points.len() - 1]))
        };
        output.push(crate::contracts::CaptureReconstructedProfile {
            candidate_id: format!(
                "profile-candidate:{}:{candidate_suffix}",
                profile.profile_id
            ),
            source_profile_id: profile.profile_id.clone(),
            support_plane_id: profile.support_plane_id.clone(),
            segments,
            closed,
            continuous: true,
            closure_error_mm,
            maximum_continuity_gap_mm: 0.0,
            support_plane_max_mm,
            supporting_evidence_ids: profile.landmark_ids.clone(),
            rejected_hypotheses,
        });
    }
    Ok(output)
}

pub(crate) fn build_capture_constraint_graph(
    guide: &CaptureReconstructionGuide,
) -> AppResult<crate::contracts::CaptureConstraintGraph> {
    let dimensions = guide
        .measurements
        .iter()
        .map(|measurement| crate::contracts::CaptureDimensionEvidence {
            dimension_id: measurement.measurement_id.clone(),
            label: measurement.label.clone(),
            landmark_ids: measurement.landmark_ids.clone(),
            value: measurement.value,
            unit: measurement.unit.clone(),
            fit_critical: measurement.fit_critical,
            parameter_name: measurement.authored_parameter_name.clone(),
            constraint_kind: measurement.constraint_kind,
        })
        .collect::<Vec<_>>();
    let mut relations = guide.authored_constraints.clone();
    for measurement in &guide.measurements {
        if let Some(kind) = measurement.constraint_kind {
            relations.push(crate::contracts::CaptureNamedConstraint {
                constraint_id: format!("constraint:dimension:{}", measurement.measurement_id),
                label: measurement.label.clone(),
                kind,
                entity_ids: if measurement.landmark_ids.is_empty() {
                    vec![measurement.measurement_id.clone()]
                } else {
                    measurement.landmark_ids.clone()
                },
                parameter_name: measurement.authored_parameter_name.clone(),
                value: Some(measurement.value),
                unit: Some(measurement.unit.clone()),
                tolerance: 0.0,
                residual: None,
                user_confirmed: true,
            });
        }
    }
    for plane in guide
        .planes
        .iter()
        .filter(|plane| plane.role == crate::contracts::CapturePlaneRole::Symmetry)
    {
        relations.push(crate::contracts::CaptureNamedConstraint {
            constraint_id: format!("constraint:symmetry:{}", plane.plane_id),
            label: format!("{} symmetry", plane.label),
            kind: crate::contracts::CaptureConstraintKind::Symmetry,
            entity_ids: vec![plane.plane_id.clone()],
            parameter_name: None,
            value: None,
            unit: None,
            tolerance: plane.fit.tolerance_mm,
            residual: Some(plane.fit.max_mm),
            user_confirmed: true,
        });
    }
    relations.sort_by(|left, right| left.constraint_id.cmp(&right.constraint_id));
    let canonical = serde_json::to_vec(&serde_json::json!({
        "dimensions": dimensions,
        "relations": relations,
    }))
    .map_err(|error| AppError::internal(error.to_string()))?;
    Ok(crate::contracts::CaptureConstraintGraph {
        dimensions,
        relations,
        content_digest: format!("sha256:{:x}", Sha256::digest(canonical)),
    })
}

fn build_feature_plan_candidates(
    guide: &CaptureReconstructionGuide,
) -> Vec<crate::contracts::CaptureFeaturePlanCandidate> {
    let mut output = Vec::new();
    let distance_dimension = guide.constraint_graph.dimensions.iter().find(|dimension| {
        dimension.fit_critical
            && dimension.unit == "mm"
            && dimension
                .parameter_name
                .as_ref()
                .is_some_and(|name| !name.trim().is_empty())
    });
    for profile in &guide.profiles {
        let Some(reconstructed) = guide
            .reconstructed_profiles
            .iter()
            .find(|candidate| candidate.source_profile_id == profile.profile_id)
        else {
            continue;
        };
        let mut operation_kinds = match profile.operation_hint {
            crate::contracts::CaptureProfileOperationHint::Extrude => vec!["extrude"],
            crate::contracts::CaptureProfileOperationHint::Revolve => vec!["revolve"],
            crate::contracts::CaptureProfileOperationHint::Sweep => vec!["sweep"],
            crate::contracts::CaptureProfileOperationHint::ReferenceOnly => vec![],
            crate::contracts::CaptureProfileOperationHint::AgentDecide => {
                vec!["extrude", "revolve", "sweep"]
            }
        };
        operation_kinds.sort_unstable();
        for kind in operation_kinds {
            let mut rejecting_evidence = Vec::new();
            let primary = match kind {
                "extrude" => {
                    if !reconstructed.closed || !reconstructed.continuous {
                        rejecting_evidence
                            .push("Extrude needs one closed continuous profile.".into());
                    }
                    distance_dimension.map(|dimension| {
                        crate::contracts::CaptureFeatureOperation::Extrude {
                            profile_candidate_id: reconstructed.candidate_id.clone(),
                            distance_dimension_id: dimension.dimension_id.clone(),
                        }
                    })
                }
                "revolve" => guide.axes.first().map(|axis| {
                    crate::contracts::CaptureFeatureOperation::Revolve {
                        profile_candidate_id: reconstructed.candidate_id.clone(),
                        axis_id: axis.axis_id.clone(),
                        angle_deg: 360.0,
                    }
                }),
                "sweep" => guide.axes.first().map(|axis| {
                    crate::contracts::CaptureFeatureOperation::Sweep {
                        profile_candidate_id: reconstructed.candidate_id.clone(),
                        path_id: axis.axis_id.clone(),
                    }
                }),
                _ => None,
            };
            if primary.is_none() {
                rejecting_evidence.push(
                    match kind {
                        "extrude" => "Extrude needs a fit-critical named length parameter.",
                        "revolve" => "Revolve needs a named axis.",
                        _ => "Sweep needs a named path or axis.",
                    }
                    .into(),
                );
            }
            let mut operations = primary.into_iter().collect::<Vec<_>>();
            match &guide.symmetry_completion {
                crate::contracts::CaptureSymmetryCompletion::None => {}
                crate::contracts::CaptureSymmetryCompletion::Half { plane_id } => {
                    operations.push(crate::contracts::CaptureFeatureOperation::Mirror {
                        plane_id: plane_id.clone(),
                    });
                }
                crate::contracts::CaptureSymmetryCompletion::Quarter {
                    first_plane_id,
                    second_plane_id,
                } => {
                    operations.push(crate::contracts::CaptureFeatureOperation::Mirror {
                        plane_id: first_plane_id.clone(),
                    });
                    operations.push(crate::contracts::CaptureFeatureOperation::Mirror {
                        plane_id: second_plane_id.clone(),
                    });
                }
            }
            let ambiguous = profile.operation_hint
                == crate::contracts::CaptureProfileOperationHint::AgentDecide
                && rejecting_evidence.is_empty();
            let plan_id = format!("plan:{}:{kind}", profile.profile_id);
            let explicitly_selected =
                guide.selected_feature_plan_id.as_deref() == Some(plan_id.as_str());
            output.push(crate::contracts::CaptureFeaturePlanCandidate {
                plan_id,
                label: format!("{} {kind}", profile.label),
                operations,
                supporting_evidence_ids: std::iter::once(profile.profile_id.clone())
                    .chain(reconstructed.supporting_evidence_ids.iter().cloned())
                    .collect(),
                rejecting_evidence: rejecting_evidence.clone(),
                score: if rejecting_evidence.is_empty() {
                    1.0
                } else {
                    0.0
                },
                status: if !rejecting_evidence.is_empty() {
                    crate::contracts::CaptureFeaturePlanStatus::Rejected
                } else if ambiguous && !explicitly_selected {
                    crate::contracts::CaptureFeaturePlanStatus::NeedsConfirmation
                } else {
                    crate::contracts::CaptureFeaturePlanStatus::Supported
                },
            });
        }
    }
    let operand_plans = output
        .iter()
        .filter(|plan| plan.status != crate::contracts::CaptureFeaturePlanStatus::Rejected)
        .map(|plan| plan.plan_id.clone())
        .collect::<Vec<_>>();
    if operand_plans.len() >= 2 {
        let union_id = format!("plan:boolean-union:{}", operand_plans.join("+"));
        let union_selected = guide.selected_feature_plan_id.as_deref() == Some(union_id.as_str());
        output.push(crate::contracts::CaptureFeaturePlanCandidate {
            plan_id: union_id,
            label: "Evidence-backed boolean union".into(),
            operations: vec![crate::contracts::CaptureFeatureOperation::BooleanUnion {
                operand_plan_ids: operand_plans.clone(),
            }],
            supporting_evidence_ids: operand_plans.clone(),
            rejecting_evidence: Vec::new(),
            score: 0.85,
            status: if union_selected {
                crate::contracts::CaptureFeaturePlanStatus::Supported
            } else {
                crate::contracts::CaptureFeaturePlanStatus::NeedsConfirmation
            },
        });
        for base_plan_id in &operand_plans {
            let cutter_plan_ids = operand_plans
                .iter()
                .filter(|plan_id| *plan_id != base_plan_id)
                .cloned()
                .collect::<Vec<_>>();
            let plan_id = format!(
                "plan:boolean-difference:{base_plan_id}-{}",
                cutter_plan_ids.join("-")
            );
            let selected = guide.selected_feature_plan_id.as_deref() == Some(plan_id.as_str());
            output.push(crate::contracts::CaptureFeaturePlanCandidate {
                plan_id,
                label: format!("Evidence-backed difference from {base_plan_id}"),
                operations: vec![
                    crate::contracts::CaptureFeatureOperation::BooleanDifference {
                        base_plan_id: base_plan_id.clone(),
                        cutter_plan_ids,
                    },
                ],
                supporting_evidence_ids: operand_plans.clone(),
                rejecting_evidence: Vec::new(),
                score: 0.8,
                status: if selected {
                    crate::contracts::CaptureFeaturePlanStatus::Supported
                } else {
                    crate::contracts::CaptureFeaturePlanStatus::NeedsConfirmation
                },
            });
        }
    }
    output.sort_by(|left, right| left.plan_id.cmp(&right.plan_id));
    output
}

fn evaluate_reconstruction_readiness(
    guide: &CaptureReconstructionGuide,
) -> crate::contracts::CaptureReconstructionReadiness {
    use crate::contracts::{
        CaptureReadinessStageEvidence as Evidence, CaptureReadinessStageStatus as Status,
        CaptureReconstructionStage as Stage,
    };
    let checks = [
        (
            Stage::Neighborhood,
            guide.surface_neighborhoods.len() == guide.landmarks.len(),
            false,
            guide
                .landmarks
                .iter()
                .map(|item| item.landmark_id.clone())
                .collect(),
            "Every landmark needs bounded neighborhood and uncertainty evidence.",
        ),
        (
            Stage::PrimitiveFit,
            guide.axes.len() + guide.planes.len()
                <= guide
                    .primitive_candidates
                    .iter()
                    .filter(|candidate| {
                        matches!(
                            candidate.geometry,
                            crate::contracts::CaptureAnalyticPrimitive::Line { .. }
                                | crate::contracts::CaptureAnalyticPrimitive::Plane { .. }
                        )
                    })
                    .count(),
            false,
            guide
                .axes
                .iter()
                .map(|item| item.axis_id.clone())
                .chain(guide.planes.iter().map(|item| item.plane_id.clone()))
                .collect(),
            "Every named axis and plane needs a residual-bounded primitive candidate.",
        ),
        (
            Stage::Segmentation,
            !guide.surface_regions.is_empty()
                && guide
                    .surface_regions
                    .iter()
                    .map(|region| region.triangle_indices.len())
                    .sum::<usize>()
                    == guide.source_mesh.triangle_count as usize,
            false,
            guide
                .surface_regions
                .iter()
                .map(|item| item.region_id.clone())
                .collect(),
            "Selected source mesh needs complete deterministic region coverage.",
        ),
        (
            Stage::ProfileReconstruction,
            guide.reconstructed_profiles.len() == guide.profiles.len(),
            false,
            guide
                .profiles
                .iter()
                .map(|item| item.profile_id.clone())
                .collect(),
            "Every requested profile needs continuity and support-plane proof.",
        ),
        (
            Stage::ConstraintGraph,
            guide.constraint_graph.dimensions.iter().all(|dimension| {
                !dimension.fit_critical
                    || dimension
                        .parameter_name
                        .as_ref()
                        .is_some_and(|name| !name.trim().is_empty())
            }),
            false,
            guide
                .constraint_graph
                .dimensions
                .iter()
                .filter(|item| item.fit_critical)
                .map(|item| item.dimension_id.clone())
                .collect(),
            "Fit-critical dimensions need named authored parameters.",
        ),
        (
            Stage::FeaturePlan,
            guide
                .selected_feature_plan_id
                .as_ref()
                .is_some_and(|selected| {
                    guide.feature_plan_candidates.iter().any(|candidate| {
                        candidate.plan_id == *selected
                            && candidate.status
                                == crate::contracts::CaptureFeaturePlanStatus::Supported
                    })
                }),
            guide.feature_plan_candidates.iter().any(|candidate| {
                candidate.status == crate::contracts::CaptureFeaturePlanStatus::NeedsConfirmation
            }),
            guide
                .feature_plan_candidates
                .iter()
                .map(|item| item.plan_id.clone())
                .collect(),
            "One supported feature plan must be selected; competing plans need confirmation.",
        ),
    ];
    let mut stages = Vec::with_capacity(checks.len());
    let mut missing_stages = Vec::new();
    let mut ambiguous_stages = Vec::new();
    for (stage, satisfied, ambiguous, affected_evidence_ids, detail) in checks {
        let bypass = guide.stage_bypasses.iter().find(|bypass| {
            bypass.stage == stage
                && bypass.accepted_by_user
                && !bypass.rationale.trim().is_empty()
                && !bypass.explicit_constraint_ids.is_empty()
                && bypass.explicit_constraint_ids.iter().all(|constraint_id| {
                    guide.constraint_graph.relations.iter().any(|constraint| {
                        constraint.constraint_id == *constraint_id && constraint.user_confirmed
                    })
                })
        });
        let status = if satisfied {
            Status::Satisfied
        } else if bypass.is_some() {
            Status::Bypassed
        } else if ambiguous {
            ambiguous_stages.push(stage);
            Status::Ambiguous
        } else {
            missing_stages.push(stage);
            Status::Missing
        };
        stages.push(Evidence {
            stage,
            status,
            affected_evidence_ids,
            detail: bypass
                .map(|bypass| format!("Bypassed by explicit constraints: {}", bypass.rationale))
                .unwrap_or_else(|| detail.into()),
        });
    }
    let ready = missing_stages.is_empty() && ambiguous_stages.is_empty();
    crate::contracts::CaptureReconstructionReadiness {
        ready,
        stages,
        missing_stages,
        ambiguous_stages,
        selected_feature_plan_id: guide.selected_feature_plan_id.clone(),
        detail: if ready {
            "Deterministic reconstruction stack is ready for bounded semantic authoring.".into()
        } else {
            "Deterministic reconstruction stack is incomplete or ambiguous; agent handoff is blocked."
                .into()
        },
    }
}

pub fn validate_guide_draft_from_stl(
    path: &Path,
    guide: &mut CaptureReconstructionGuide,
) -> AppResult<()> {
    if guide.schema_version != crate::contracts::CAPTURE_RECONSTRUCTION_GUIDE_SCHEMA_VERSION {
        return Err(AppError::validation(format!(
            "Unsupported capture reconstruction guide schema version '{}'.",
            guide.schema_version
        )));
    }
    if guide.landmarks.len() > 256 || guide.instruction.len() > 4_096 {
        return Err(AppError::validation(
            "Capture reconstruction guide draft exceeds bounded limits.",
        ));
    }
    let mut ids = std::collections::HashSet::new();
    for landmark in &mut guide.landmarks {
        if landmark.landmark_id.trim().is_empty() || !ids.insert(landmark.landmark_id.clone()) {
            return Err(AppError::validation(format!(
                "Duplicate or empty capture landmark ID '{}'.",
                landmark.landmark_id
            )));
        }
        let validated = validate_surface_anchor_from_stl(path, &landmark.anchor, 1.0e-6)?;
        landmark.anchor.source_position = validated.source_position;
        landmark.anchor.source_normal = validated.source_normal;
    }
    Ok(())
}

pub fn deterministic_evidence_views(
    guide: &CaptureReconstructionGuide,
) -> AppResult<Vec<crate::contracts::CaptureEvidenceView>> {
    if guide.landmarks.is_empty() {
        return Err(AppError::validation(
            "Guided reconstruction needs at least one calibrated landmark.",
        ));
    }
    let mut minimum = [f64::INFINITY; 3];
    let mut maximum = [f64::NEG_INFINITY; 3];
    for landmark in &guide.landmarks {
        if !finite3(landmark.local_position_mm) {
            return Err(AppError::validation(format!(
                "Landmark '{}' has non-finite local coordinates.",
                landmark.landmark_id
            )));
        }
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(landmark.local_position_mm[axis]);
            maximum[axis] = maximum[axis].max(landmark.local_position_mm[axis]);
        }
    }
    let center = scale(add(minimum, maximum), 0.5);
    let span = sub(maximum, minimum);
    let distance = norm(span).max(1.0) * 1.5;
    let landmark_ids = guide
        .landmarks
        .iter()
        .map(|landmark| landmark.landmark_id.clone())
        .collect::<Vec<_>>();
    let profile_ids = guide
        .profiles
        .iter()
        .map(|profile| profile.profile_id.clone())
        .collect::<Vec<_>>();
    let view = |view_id: &str, label: &str, camera_position_mm: [f64; 3], camera_up: [f64; 3]| {
        crate::contracts::CaptureEvidenceView {
            view_id: view_id.into(),
            label: label.into(),
            camera_position_mm,
            camera_target_mm: center,
            camera_up,
            landmark_ids: landmark_ids.clone(),
            profile_ids: profile_ids.clone(),
            artifact_digest: None,
        }
    };
    Ok(vec![
        view(
            "front",
            "Front",
            [center[0], center[1] - distance, center[2]],
            [0.0, 0.0, 1.0],
        ),
        view(
            "right",
            "Right",
            [center[0] + distance, center[1], center[2]],
            [0.0, 0.0, 1.0],
        ),
        view(
            "top",
            "Top",
            [center[0], center[1], center[2] + distance],
            [0.0, 1.0, 0.0],
        ),
        view(
            "isometric",
            "Isometric",
            [
                center[0] + distance,
                center[1] - distance,
                center[2] + distance,
            ],
            [0.0, 0.0, 1.0],
        ),
    ])
}

pub fn build_guided_reconstruction_request(
    guide: &CaptureReconstructionGuide,
    target_source_digest: &str,
    target_version_id: Option<String>,
) -> AppResult<crate::contracts::CaptureGuidedReconstructionRequest> {
    guide.validate().map_err(AppError::validation)?;
    validate_computed_reconstruction_evidence(guide)?;
    if guide.landmarks.len() > 256
        || guide.profiles.len() > 64
        || guide.feature_expectations.len() > 128
        || guide.instruction.len() > 4_096
    {
        return Err(AppError::validation(
            "Capture reconstruction guide exceeds bounded handoff limits.",
        ));
    }
    match &guide.calibration.method {
        CaptureCalibrationMethod::KnownDistance if guide.calibration.measurements.is_empty() => {
            return Err(AppError::validation(
                "Guided reconstruction needs known-distance calibration evidence.",
            ));
        }
        CaptureCalibrationMethod::TrustedMetricMetadata {
            accepted_by_user: true,
            provenance,
        } if !provenance.trim().is_empty() => {}
        CaptureCalibrationMethod::TrustedMetricMetadata { .. } => {
            return Err(AppError::validation(
                "Guided reconstruction metric provenance is untrusted.",
            ));
        }
        _ => {}
    }
    if !target_source_digest.starts_with("sha256:") {
        return Err(AppError::validation(
            "Guided reconstruction target source digest is invalid.",
        ));
    }
    if guide.target_source_digest != target_source_digest
        || guide.target_version_id != target_version_id
    {
        return Err(AppError::conflict(
            "Guided request target source/version differs from guide identity.",
        ));
    }
    let expected_digest = guide
        .compute_canonical_digest()
        .map_err(AppError::validation)?;
    if guide.canonical_digest != expected_digest {
        return Err(AppError::conflict(
            "Capture guide canonical digest differs from guide payload.",
        ));
    }
    let evidence_views = deterministic_evidence_views(guide)?;
    let required_feature_expectation_ids = guide
        .feature_expectations
        .iter()
        .filter(|expectation| expectation.required_for_acceptance)
        .map(|expectation| expectation.expectation_id.clone())
        .collect::<Vec<_>>();
    let identity = serde_json::to_vec(&serde_json::json!({
        "captureRunId": guide.capture_run_id,
        "guideId": guide.guide_id,
        "guideRevision": guide.revision,
        "guideCanonicalDigest": guide.canonical_digest,
        "targetSourceDigest": target_source_digest,
        "targetVersionId": target_version_id,
    }))
    .map_err(|error| AppError::internal(error.to_string()))?;
    Ok(crate::contracts::CaptureGuidedReconstructionRequest {
        schema_version: 1,
        request_id: format!("capture-guide:sha256:{:x}", Sha256::digest(identity)),
        capture_run_id: guide.capture_run_id.clone(),
        guide_id: guide.guide_id.clone(),
        guide_revision: guide.revision,
        guide_canonical_digest: guide.canonical_digest.clone(),
        target_thread_id: guide.target_thread_id.clone(),
        target_message_id: guide.target_message_id.clone(),
        target_source_digest: target_source_digest.into(),
        target_version_id,
        source_mesh_artifact_digest: guide.source_mesh.artifact_digest.clone(),
        source_mesh_content_digest: guide.source_mesh.content_digest.clone(),
        instruction: guide.instruction.clone(),
        guide: guide.clone(),
        evidence_views,
        requirements: crate::contracts::CaptureGuidedOutputRequirements {
            source_language: "ecky".into(),
            geometry_representation: "analyticBrep".into(),
            require_parametric_source: true,
            require_named_fit_constraints: true,
            require_explicit_symmetry_operations: true,
            forbid_mesh_solidification: true,
            forbid_unbound_feature_operations: true,
            selected_feature_plan_id: guide.selected_feature_plan_id.clone().ok_or_else(|| {
                AppError::validation("Capture guide has no selected feature plan.")
            })?,
            required_feature_expectation_ids,
        },
    })
}

pub fn validate_computed_reconstruction_evidence(
    guide: &CaptureReconstructionGuide,
) -> AppResult<()> {
    let neighborhoods_by_landmark = guide
        .surface_neighborhoods
        .iter()
        .map(|neighborhood| (neighborhood.landmark_id.as_str(), neighborhood))
        .collect::<BTreeMap<_, _>>();
    for landmark in &guide.landmarks {
        let neighborhood = neighborhoods_by_landmark
            .get(landmark.landmark_id.as_str())
            .ok_or_else(|| {
                AppError::validation(format!(
                    "Capture guide is missing computed surface neighborhood for landmark '{}'.",
                    landmark.landmark_id
                ))
            })?;
        if landmark.uncertainty_mm.is_none() {
            return Err(AppError::validation(format!(
                "Capture guide is missing computed uncertainty for landmark '{}'.",
                landmark.landmark_id
            )));
        }
        if neighborhood.truncated_by_budget {
            return Err(AppError::validation(format!(
                "Capture surface neighborhood '{}' reached its triangle budget; increase budget or reduce radius before handoff.",
                neighborhood.neighborhood_id
            )));
        }
    }
    let candidate_guide_items = guide
        .primitive_candidates
        .iter()
        .flat_map(|candidate| candidate.guide_item_ids.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    for (kind, item_id) in guide
        .axes
        .iter()
        .map(|axis| ("axis", axis.axis_id.as_str()))
        .chain(
            guide
                .planes
                .iter()
                .map(|plane| ("plane", plane.plane_id.as_str())),
        )
    {
        if !candidate_guide_items.contains(item_id) {
            return Err(AppError::validation(format!(
                "Capture guide is missing computed primitive candidate for {kind} '{item_id}'."
            )));
        }
    }
    if !guide.reconstruction_readiness.ready {
        return Err(AppError::validation(format!(
            "Capture deterministic reconstruction stack is not ready: missing={:?}, ambiguous={:?}. {}",
            guide.reconstruction_readiness.missing_stages,
            guide.reconstruction_readiness.ambiguous_stages,
            guide.reconstruction_readiness.detail
        )));
    }
    let selected_plan_id = guide
        .selected_feature_plan_id
        .as_deref()
        .ok_or_else(|| AppError::validation("Capture guide has no selected feature plan."))?;
    let selected_plan = guide
        .feature_plan_candidates
        .iter()
        .find(|candidate| candidate.plan_id == selected_plan_id)
        .ok_or_else(|| AppError::validation("Capture selected feature plan is missing."))?;
    if selected_plan.status != crate::contracts::CaptureFeaturePlanStatus::Supported
        || !selected_plan.rejecting_evidence.is_empty()
    {
        return Err(AppError::validation(format!(
            "Capture selected feature plan '{}' is not supported by deterministic evidence.",
            selected_plan.plan_id
        )));
    }
    Ok(())
}

pub fn guided_reconstruction_prompt(
    request: &crate::contracts::CaptureGuidedReconstructionRequest,
) -> AppResult<String> {
    let request_json = serde_json::to_string(request)
        .map_err(|error| AppError::internal(format!("Guided request encoding failed: {error}")))?;
    let prompt = format!(
        "Build parametric .ecky analytic BRep from exact capture guide below.\n\
         Follow MCP order: inspect -> validate -> preview -> commit. Keep source/history unchanged until explicit commit.\n\
         Do not solidify, mirror, patch, or export the scan mesh as manufacturing geometry.\n\
         Author unique half/quarter features once; use explicit named mirror/repeat/instance operations.\n\
         Represent every fit-critical offset as named parameter/binding/constraint.\n\
         Author only operations from requirements.selectedFeaturePlanId and guide.featurePlanCandidates; do not invent primitives, dimensions, or feature operations.\n\
         Bind every selected-plan operation to authored stable nodes and exact BRep targets.\n\
         Emit named authored binding or selector tag for every required feature expectation.\n\
         Before commit, pass captureGuidedResult with exact requestId/guideCanonicalDigest, unresolvedAssumptions, and inferredRegions.\n\
         If unresolvedAssumptions is non-empty, call user_confirm_request and do not commit.\n\
         Keep inferred regions explicit; never hide an assumption.\n\
         CANONICAL_CAPTURE_GUIDE_REQUEST={request_json}"
    );
    if prompt.len() > 65_536 {
        return Err(AppError::validation(
            "Canonical guided reconstruction request exceeds 65536-byte prompt bound.",
        ));
    }
    Ok(prompt)
}

fn validate_points_and_tolerance(
    points: &[[f64; 3]],
    tolerance_mm: f64,
    kind: &str,
) -> AppResult<()> {
    if points.iter().any(|point| !finite3(*point)) {
        return Err(AppError::validation(format!(
            "{kind} evidence contains non-finite coordinates."
        )));
    }
    if !tolerance_mm.is_finite() || tolerance_mm < 0.0 {
        return Err(AppError::validation(format!(
            "{kind} fit tolerance must be finite and non-negative."
        )));
    }
    Ok(())
}

fn centroid(points: &[[f64; 3]]) -> [f64; 3] {
    scale(
        points.iter().copied().fold([0.0; 3], add),
        1.0 / points.len() as f64,
    )
}

fn covariance(points: &[[f64; 3]], center: [f64; 3]) -> [[f64; 3]; 3] {
    let mut covariance = [[0.0; 3]; 3];
    for point in points {
        let delta = sub(*point, center);
        for row in 0..3 {
            for col in 0..3 {
                covariance[row][col] += delta[row] * delta[col];
            }
        }
    }
    covariance
}

fn symmetric_eigen(mut matrix: [[f64; 3]; 3]) -> ([f64; 3], [[f64; 3]; 3]) {
    let mut vectors = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..32 {
        let mut p = 0;
        let mut q = 1;
        let mut maximum = matrix[0][1].abs();
        for (row, col) in [(0, 2), (1, 2)] {
            if matrix[row][col].abs() > maximum {
                p = row;
                q = col;
                maximum = matrix[row][col].abs();
            }
        }
        if maximum <= 1.0e-15 {
            break;
        }
        let angle = 0.5 * (2.0 * matrix[p][q]).atan2(matrix[q][q] - matrix[p][p]);
        let cosine = angle.cos();
        let sine = angle.sin();
        let app = matrix[p][p];
        let aqq = matrix[q][q];
        let apq = matrix[p][q];
        matrix[p][p] = cosine * cosine * app - 2.0 * sine * cosine * apq + sine * sine * aqq;
        matrix[q][q] = sine * sine * app + 2.0 * sine * cosine * apq + cosine * cosine * aqq;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;
        for index in 0..3 {
            if index != p && index != q {
                let aip = matrix[index][p];
                let aiq = matrix[index][q];
                matrix[index][p] = cosine * aip - sine * aiq;
                matrix[p][index] = matrix[index][p];
                matrix[index][q] = sine * aip + cosine * aiq;
                matrix[q][index] = matrix[index][q];
            }
            let vip = vectors[index][p];
            let viq = vectors[index][q];
            vectors[index][p] = cosine * vip - sine * viq;
            vectors[index][q] = sine * vip + cosine * viq;
        }
    }
    ([matrix[0][0], matrix[1][1], matrix[2][2]], vectors)
}

fn largest_index(values: [f64; 3]) -> usize {
    if values[0] >= values[1] && values[0] >= values[2] {
        0
    } else if values[1] >= values[2] {
        1
    } else {
        2
    }
}

fn column(matrix: [[f64; 3]; 3], index: usize) -> [f64; 3] {
    [matrix[0][index], matrix[1][index], matrix[2][index]]
}

fn canonical_direction(vector: [f64; 3]) -> [f64; 3] {
    let mut direction = normalize(vector).unwrap_or([1.0, 0.0, 0.0]);
    let first_nonzero = direction
        .iter()
        .copied()
        .find(|value| value.abs() > 1.0e-12)
        .unwrap_or(1.0);
    if first_nonzero < 0.0 {
        direction = scale(direction, -1.0);
    }
    direction
}

fn residual_summary(residuals: &[f64]) -> (f64, f64) {
    let max = residuals.iter().copied().fold(0.0, f64::max);
    let rms =
        (residuals.iter().map(|value| value * value).sum::<f64>() / residuals.len() as f64).sqrt();
    (rms, max)
}

fn finite3(value: [f64; 3]) -> bool {
    value.iter().all(|component| component.is_finite())
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(value: [f64; 3], factor: f64) -> [f64; 3] {
    [value[0] * factor, value[1] * factor, value[2] * factor]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm(value: [f64; 3]) -> f64 {
    dot(value, value).sqrt()
}

fn normalize(value: [f64; 3]) -> Option<[f64; 3]> {
    let length = norm(value);
    if !length.is_finite() || length <= VECTOR_EPSILON {
        None
    } else {
        Some(scale(value, 1.0 / length))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::CaptureSurfaceAnchor;

    fn write_triangle_stl() -> std::path::PathBuf {
        write_triangle_stl_at_z(0.0)
    }

    fn write_triangle_stl_at_z(z: f64) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("ecky-capture-guide-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("source.stl");
        std::fs::write(
            &path,
            format!("solid source\nfacet normal 0 0 1\nouter loop\nvertex 0 0 {z}\nvertex 2 0 {z}\nvertex 0 2 {z}\nendloop\nendfacet\nendsolid source\n"),
        )
        .unwrap();
        path
    }

    #[test]
    fn reloads_stl_and_validates_digest_bound_barycentric_anchor() {
        let path = write_triangle_stl();
        let digest = source_mesh_content_digest(&path).unwrap();
        let anchor = CaptureSurfaceAnchor {
            source_mesh_content_digest: digest,
            triangle_index: 0,
            barycentric: [0.25, 0.25, 0.5],
            source_position: [0.5, 1.0, 0.0],
            source_normal: [0.0, 0.0, 1.0],
        };

        let validated = validate_surface_anchor_from_stl(&path, &anchor, 1.0e-9).unwrap();
        assert_eq!(validated.source_position, [0.5, 1.0, 0.0]);
        assert_eq!(validated.source_normal, [0.0, 0.0, 1.0]);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn selected_source_identity_separates_file_artifact_digest_from_indexed_mesh_digest() {
        let path = write_triangle_stl();
        let identity =
            inspect_capture_source_mesh(&path, crate::contracts::CaptureMeshSelection::Raw)
                .unwrap();
        assert!(identity.artifact_digest.starts_with("sha256:"));
        assert!(identity.content_digest.starts_with("sha256:"));
        assert_ne!(identity.artifact_digest, identity.content_digest);
        assert_eq!(identity.triangle_count, 1);
        assert_eq!(identity.source_bounds.min, [0.0, 0.0, 0.0]);
        assert_eq!(identity.source_bounds.max, [2.0, 2.0, 0.0]);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn anchor_validation_rejects_digest_range_position_and_normal_failures() {
        let path = write_triangle_stl();
        let valid_digest = source_mesh_content_digest(&path).unwrap();
        let base = CaptureSurfaceAnchor {
            source_mesh_content_digest: valid_digest,
            triangle_index: 0,
            barycentric: [0.25, 0.25, 0.5],
            source_position: [0.5, 1.0, 0.0],
            source_normal: [0.0, 0.0, 1.0],
        };
        let mut invalid = base.clone();
        invalid.source_mesh_content_digest = "sha256:stale".into();
        assert_eq!(
            validate_surface_anchor_from_stl(&path, &invalid, 1.0e-9)
                .unwrap_err()
                .message,
            "Capture anchor mesh digest differs from selected source mesh."
        );
        let mut invalid = base.clone();
        invalid.barycentric = [0.5, 0.5, 0.5];
        assert_eq!(
            validate_surface_anchor_from_stl(&path, &invalid, 1.0e-9)
                .unwrap_err()
                .message,
            "Capture anchor barycentric weights must be finite, bounded, and sum to one."
        );
        let mut invalid = base.clone();
        invalid.source_position = [0.6, 1.0, 0.0];
        assert_eq!(validate_surface_anchor_from_stl(&path, &invalid, 1.0e-9).unwrap_err().message, "Capture anchor source position differs from triangle interpolation by 0.100000 source units.");
        let mut invalid = base;
        invalid.source_normal = [0.0, 0.0, -1.0];
        assert_eq!(
            validate_surface_anchor_from_stl(&path, &invalid, 1.0e-9)
                .unwrap_err()
                .message,
            "Capture anchor normal opposes selected triangle orientation."
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn remap_proposals_are_non_authoritative_until_every_stale_anchor_is_confirmed() {
        let old_path = write_triangle_stl();
        let new_path = write_triangle_stl_at_z(0.1);
        let old_digest = source_mesh_content_digest(&old_path).unwrap();
        let new_digest = source_mesh_content_digest(&new_path).unwrap();
        let mut guide = crate::contracts::CaptureReconstructionGuide::test_fixture();
        guide.source_mesh =
            inspect_capture_source_mesh(&old_path, crate::contracts::CaptureMeshSelection::Raw)
                .unwrap();
        guide.calibration.measurements = vec![crate::contracts::CaptureKnownDistanceMeasurement {
            measurement_id: "calibration-1".into(),
            label: "known span".into(),
            first_landmark_id: "landmark-1".into(),
            second_landmark_id: "landmark-2".into(),
            known_distance_mm: 1.0,
            fitted_distance_mm: 1.0,
            residual_mm: 0.0,
            accepted_tolerance_mm: 0.01,
        }];
        let positions = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        let barycentrics = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        for ((landmark, position), barycentric) in
            guide.landmarks.iter_mut().zip(positions).zip(barycentrics)
        {
            landmark.anchor = CaptureSurfaceAnchor {
                source_mesh_content_digest: old_digest.clone(),
                triangle_index: 0,
                barycentric,
                source_position: position,
                source_normal: [0.0, 0.0, 1.0],
            };
        }
        guide.canonical_digest = guide.compute_canonical_digest().unwrap();
        for index in 0..guide.landmarks.len() {
            let landmark_id = guide.landmarks[index].landmark_id.clone();
            let mut position = positions[index];
            position[2] = 0.1;
            let proposal = propose_capture_anchor_remap(
                &new_path,
                &guide,
                &landmark_id,
                CaptureSurfaceAnchor {
                    source_mesh_content_digest: new_digest.clone(),
                    triangle_index: 0,
                    barycentric: barycentrics[index],
                    source_position: position,
                    source_normal: [0.0, 0.0, 1.0],
                },
            )
            .unwrap();
            assert!(!proposal.confirmed);
            guide.remap_proposals.push(proposal);
        }
        let before_rejected_apply = guide.clone();
        let error = apply_confirmed_capture_anchor_remaps(&new_path, &mut guide)
            .expect_err("unconfirmed remap must not mutate guide")
            .message;
        assert!(error.contains("no confirmed remap"), "{error}");
        assert_eq!(guide, before_rejected_apply);

        for proposal in &mut guide.remap_proposals {
            proposal.confirmed = true;
        }
        let previous_revision = guide.revision;
        apply_confirmed_capture_anchor_remaps(&new_path, &mut guide).unwrap();
        assert_eq!(guide.revision, previous_revision + 1);
        assert!(guide.remap_proposals.is_empty());
        assert!(guide
            .landmarks
            .iter()
            .all(|landmark| { landmark.anchor.source_mesh_content_digest == new_digest }));
        assert_eq!(guide.source_mesh.content_digest, new_digest);

        std::fs::remove_dir_all(old_path.parent().unwrap()).unwrap();
        std::fs::remove_dir_all(new_path.parent().unwrap()).unwrap();
    }

    #[test]
    fn known_distance_fit_solves_uniform_scale_and_reports_each_residual() {
        let fit = fit_known_distance_scale(
            &[
                KnownDistanceObservation {
                    source_distance: 10.0,
                    known_distance_mm: 20.0,
                },
                KnownDistanceObservation {
                    source_distance: 20.0,
                    known_distance_mm: 40.0,
                },
            ],
            0.01,
        )
        .unwrap();
        assert!((fit.millimetres_per_source_unit - 2.0).abs() < 1.0e-12);
        assert_eq!(fit.residuals_mm, vec![0.0, 0.0]);

        let error = fit_known_distance_scale(
            &[
                KnownDistanceObservation {
                    source_distance: 10.0,
                    known_distance_mm: 20.0,
                },
                KnownDistanceObservation {
                    source_distance: 20.0,
                    known_distance_mm: 50.0,
                },
            ],
            1.0,
        )
        .unwrap_err();
        assert!(
            error
                .message
                .starts_with("Known-distance evidence conflicts:"),
            "{}",
            error.message
        );
    }

    #[test]
    fn constructs_right_handed_frame_and_transforms_world_mm_to_local_mm() {
        let frame =
            construct_reconstruction_frame([1.0, 2.0, 3.0], [3.0, 2.0, 3.0], [1.0, 5.0, 3.0])
                .unwrap();
        assert_eq!(frame.x_axis, [1.0, 0.0, 0.0]);
        assert_eq!(frame.y_axis, [0.0, 1.0, 0.0]);
        assert_eq!(frame.z_axis, [0.0, 0.0, 1.0]);
        assert_eq!(frame.to_local_mm([3.0, 5.0, 7.0]), [2.0, 3.0, 4.0]);

        let error =
            construct_reconstruction_frame([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0])
                .unwrap_err();
        assert_eq!(
            error.message,
            "Frame evidence is degenerate: origin, X, and Y landmarks are collinear."
        );
    }

    #[test]
    fn least_squares_axis_and_plane_fits_are_deterministic_and_tolerance_gated() {
        let axis = fit_named_axis(
            &[
                [0.0, 0.01, 0.0],
                [1.0, -0.01, 0.0],
                [2.0, 0.01, 0.0],
                [3.0, -0.01, 0.0],
            ],
            0.02,
        )
        .unwrap();
        assert!(axis.direction[0] > 0.999, "{:?}", axis.direction);
        assert!(axis.rms_mm <= axis.max_mm && axis.max_mm <= 0.02);

        let plane = fit_named_plane(
            &[
                [0.0, 0.0, 0.01],
                [2.0, 0.0, -0.01],
                [0.0, 2.0, 0.01],
                [2.0, 2.0, -0.01],
            ],
            0.02,
        )
        .unwrap();
        assert!(plane.normal[2] > 0.999, "{:?}", plane.normal);
        assert!(plane.rms_mm <= plane.max_mm && plane.max_mm <= 0.02);

        let error =
            fit_named_plane(&[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]], 0.1).unwrap_err();
        assert_eq!(
            error.message,
            "Plane evidence is degenerate: landmarks are collinear."
        );
    }

    #[test]
    fn analytic_circle_cylinder_cone_and_sphere_fits_are_deterministic() {
        let circle_points = (0..8)
            .map(|index| {
                let angle = index as f64 * std::f64::consts::TAU / 8.0;
                [2.0 + 3.0 * angle.cos(), -1.0 + 3.0 * angle.sin(), 0.0]
            })
            .collect::<Vec<_>>();
        let circle = fit_circle_primitive(&circle_points, 1.0e-9).unwrap();
        assert!(norm(sub(circle.center_mm, [2.0, -1.0, 0.0])) <= 1.0e-9);
        assert!((circle.radius_mm - 3.0).abs() <= 1.0e-9);
        assert!(circle.max_mm <= 1.0e-9);

        let cylinder_points = [-2.0, 2.0]
            .into_iter()
            .flat_map(|z| {
                (0..8).map(move |index| {
                    let angle = index as f64 * std::f64::consts::TAU / 8.0;
                    [2.0 * angle.cos(), 2.0 * angle.sin(), z]
                })
            })
            .collect::<Vec<_>>();
        let cylinder = fit_cylinder_primitive(&cylinder_points, [0.0, 0.0, 1.0], 1.0e-9).unwrap();
        assert!((cylinder.radius_mm - 2.0).abs() <= 1.0e-9);
        assert!((cylinder.min_axis_mm + 2.0).abs() <= 1.0e-9);
        assert!((cylinder.max_axis_mm - 2.0).abs() <= 1.0e-9);

        let cone_slope = 30.0_f64.to_radians().tan();
        let cone_points = [2.0, 4.0]
            .into_iter()
            .flat_map(|z| {
                (0..8).map(move |index| {
                    let angle = index as f64 * std::f64::consts::TAU / 8.0;
                    [
                        z * cone_slope * angle.cos(),
                        z * cone_slope * angle.sin(),
                        z,
                    ]
                })
            })
            .collect::<Vec<_>>();
        let cone = fit_cone_primitive(&cone_points, [0.0, 0.0, 1.0], 1.0e-9).unwrap();
        assert!(norm(cone.apex_mm) <= 1.0e-9);
        assert!((cone.half_angle_deg - 30.0).abs() <= 1.0e-9);

        let sphere_points = vec![
            [5.0, 2.0, 3.0],
            [-3.0, 2.0, 3.0],
            [1.0, 6.0, 3.0],
            [1.0, -2.0, 3.0],
            [1.0, 2.0, 7.0],
            [1.0, 2.0, -1.0],
        ];
        let sphere = fit_sphere_primitive(&sphere_points, 1.0e-9).unwrap();
        assert!(norm(sub(sphere.center_mm, [1.0, 2.0, 3.0])) <= 1.0e-9);
        assert!((sphere.radius_mm - 4.0).abs() <= 1.0e-9);
    }

    #[test]
    fn robust_primitive_fits_report_one_deterministic_outlier_without_hiding_it() {
        let mut axis_points = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
        axis_points.push([1.0, 4.0, 0.0]);
        let axis = robust_fit_named_axis(&axis_points, 1.0e-9).unwrap();
        assert_eq!(axis.excluded_sample_indices, vec![3]);
        assert!(axis.fit.direction[0] > 0.999999);

        let mut plane_points = vec![
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [2.0, 2.0, 0.0],
        ];
        plane_points.push([1.0, 1.0, 3.0]);
        let plane = robust_fit_named_plane(&plane_points, 1.0e-9).unwrap();
        assert_eq!(plane.excluded_sample_indices, vec![4]);
        assert!(plane.fit.normal[2] > 0.999999);

        let mut circle_points = (0..8)
            .map(|index| {
                let angle = index as f64 * std::f64::consts::TAU / 8.0;
                [3.0 * angle.cos(), 3.0 * angle.sin(), 0.0]
            })
            .collect::<Vec<_>>();
        circle_points.push([9.0, 9.0, 2.0]);
        let circle = robust_fit_circle_primitive(&circle_points, 1.0e-8).unwrap();
        assert_eq!(circle.excluded_sample_indices, vec![8]);
        assert!((circle.fit.radius_mm - 3.0).abs() <= 1.0e-8);

        let mut sphere_points = vec![
            [4.0, 0.0, 0.0],
            [-4.0, 0.0, 0.0],
            [0.0, 4.0, 0.0],
            [0.0, -4.0, 0.0],
            [0.0, 0.0, 4.0],
            [0.0, 0.0, -4.0],
        ];
        sphere_points.push([12.0, 8.0, 3.0]);
        let sphere = robust_fit_sphere_primitive(&sphere_points, 1.0e-8).unwrap();
        assert_eq!(sphere.excluded_sample_indices, vec![6]);
        assert!((sphere.fit.radius_mm - 4.0).abs() <= 1.0e-8);

        let mut cylinder_points = [-2.0, 2.0]
            .into_iter()
            .flat_map(|z| {
                (0..8).map(move |index| {
                    let angle = index as f64 * std::f64::consts::TAU / 8.0;
                    [2.0 * angle.cos(), 2.0 * angle.sin(), z]
                })
            })
            .collect::<Vec<_>>();
        cylinder_points.push([8.0, 8.0, 0.0]);
        let cylinder =
            robust_fit_cylinder_primitive(&cylinder_points, [0.0, 0.0, 1.0], 1.0e-8).unwrap();
        assert_eq!(cylinder.excluded_sample_indices, vec![16]);
        assert!((cylinder.fit.radius_mm - 2.0).abs() <= 1.0e-8);

        let slope = 30.0_f64.to_radians().tan();
        let mut cone_points = [2.0, 4.0]
            .into_iter()
            .flat_map(|z| {
                (0..8).map(move |index| {
                    let angle = index as f64 * std::f64::consts::TAU / 8.0;
                    [z * slope * angle.cos(), z * slope * angle.sin(), z]
                })
            })
            .collect::<Vec<_>>();
        cone_points.push([10.0, 10.0, 3.0]);
        let cone = robust_fit_cone_primitive(&cone_points, [0.0, 0.0, 1.0], 1.0e-8).unwrap();
        assert_eq!(cone.excluded_sample_indices, vec![16]);
        assert!((cone.fit.half_angle_deg - 30.0).abs() <= 1.0e-8);
    }

    #[test]
    fn constraint_graph_keeps_every_typed_mechanical_relation_and_named_dimension() {
        let mut guide = crate::contracts::CaptureReconstructionGuide::test_fixture();
        guide
            .measurements
            .push(crate::contracts::CaptureNamedMeasurement {
                measurement_id: "clearance".into(),
                label: "running clearance".into(),
                landmark_ids: vec!["landmark-1".into(), "landmark-2".into()],
                value: 0.2,
                unit: "mm".into(),
                fit_critical: true,
                authored_parameter_name: Some("running-clearance".into()),
                constraint_kind: Some(crate::contracts::CaptureConstraintKind::Clearance),
            });
        let kinds = [
            crate::contracts::CaptureConstraintKind::Coaxial,
            crate::contracts::CaptureConstraintKind::Coplanar,
            crate::contracts::CaptureConstraintKind::Parallel,
            crate::contracts::CaptureConstraintKind::Perpendicular,
            crate::contracts::CaptureConstraintKind::Tangent,
            crate::contracts::CaptureConstraintKind::EqualRadius,
            crate::contracts::CaptureConstraintKind::Thickness,
            crate::contracts::CaptureConstraintKind::Extent,
            crate::contracts::CaptureConstraintKind::Tolerance,
        ];
        guide.authored_constraints = kinds
            .iter()
            .enumerate()
            .map(|(index, kind)| crate::contracts::CaptureNamedConstraint {
                constraint_id: format!("constraint:{index}"),
                label: format!("relation {index}"),
                kind: *kind,
                entity_ids: vec!["landmark-1".into(), "landmark-2".into()],
                parameter_name: None,
                value: None,
                unit: None,
                tolerance: 0.01,
                residual: Some(0.0),
                user_confirmed: true,
            })
            .collect();

        let graph = build_capture_constraint_graph(&guide).unwrap();
        assert_eq!(
            graph.dimensions[0].constraint_kind,
            Some(crate::contracts::CaptureConstraintKind::Clearance)
        );
        for kind in kinds {
            assert!(graph.relations.iter().any(|relation| relation.kind == kind));
        }
        assert!(graph.relations.iter().any(|relation| {
            relation.kind == crate::contracts::CaptureConstraintKind::Clearance
                && relation.parameter_name.as_deref() == Some("running-clearance")
        }));
    }

    #[test]
    fn multiple_supported_profiles_offer_bounded_union_and_difference_plans_for_confirmation() {
        let mut guide = crate::contracts::CaptureReconstructionGuide::test_fixture();
        guide
            .measurements
            .push(crate::contracts::CaptureNamedMeasurement {
                measurement_id: "depth".into(),
                label: "depth".into(),
                landmark_ids: Vec::new(),
                value: 5.0,
                unit: "mm".into(),
                fit_critical: true,
                authored_parameter_name: Some("depth".into()),
                constraint_kind: Some(crate::contracts::CaptureConstraintKind::Extent),
            });
        guide.constraint_graph = build_capture_constraint_graph(&guide).unwrap();
        let first_profile = crate::contracts::CaptureReconstructedProfile {
            candidate_id: "profile-candidate:profile-1:polyline".into(),
            source_profile_id: "profile-1".into(),
            support_plane_id: "plane-1".into(),
            segments: Vec::new(),
            closed: true,
            continuous: true,
            closure_error_mm: 0.0,
            maximum_continuity_gap_mm: 0.0,
            support_plane_max_mm: 0.0,
            supporting_evidence_ids: vec!["landmark-1".into()],
            rejected_hypotheses: Vec::new(),
        };
        let mut second_profile = first_profile.clone();
        second_profile.candidate_id = "profile-candidate:profile-2:polyline".into();
        second_profile.source_profile_id = "profile-2".into();
        let mut second_source = guide.profiles[0].clone();
        second_source.profile_id = "profile-2".into();
        second_source.label = "cutter".into();
        guide.profiles.push(second_source);
        guide.reconstructed_profiles = vec![first_profile, second_profile];

        let candidates = build_feature_plan_candidates(&guide);
        let union = candidates
            .iter()
            .find(|plan| plan.plan_id.starts_with("plan:boolean-union:"))
            .unwrap();
        assert_eq!(
            union.status,
            crate::contracts::CaptureFeaturePlanStatus::NeedsConfirmation
        );
        assert!(matches!(
            union.operations[0],
            crate::contracts::CaptureFeatureOperation::BooleanUnion { .. }
        ));
        assert_eq!(
            candidates
                .iter()
                .filter(|plan| plan.plan_id.starts_with("plan:boolean-difference:"))
                .count(),
            2
        );

        guide.selected_feature_plan_id = Some(union.plan_id.clone());
        let selected = build_feature_plan_candidates(&guide);
        assert_eq!(
            selected
                .iter()
                .find(|plan| plan.plan_id == union.plan_id)
                .unwrap()
                .status,
            crate::contracts::CaptureFeaturePlanStatus::Supported
        );
    }

    #[test]
    fn profile_arc_and_interpolating_spline_have_bounded_domains_and_residuals() {
        let arc_points = vec![
            [10.0, 0.0, 0.0],
            [10.0 / 2.0_f64.sqrt(), 10.0 / 2.0_f64.sqrt(), 0.0],
            [0.0, 10.0, 0.0],
        ];
        let arc = fit_profile_arc(&arc_points, [0.0, 0.0, 1.0], 1.0e-8).unwrap();
        assert!((arc.radius_mm - 10.0).abs() <= 1.0e-8);
        assert!(arc.end_angle_deg > arc.start_angle_deg);
        assert!(arc.max_mm <= 1.0e-8);

        let spline_points = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.5, 0.0],
            [2.0, -0.25, 0.0],
            [3.0, 1.0, 0.0],
            [4.0, 0.0, 0.0],
        ];
        let spline = fit_interpolating_profile_spline(&spline_points, 1.0e-8).unwrap();
        assert_eq!(spline.degree, 3);
        assert_eq!(spline.control_points_mm.len(), spline_points.len());
        assert_eq!(
            spline.knots.len(),
            spline.control_points_mm.len() + spline.degree as usize + 1
        );
        assert!(spline.max_mm <= 1.0e-8);
    }

    #[test]
    fn authoritative_guide_recompute_validates_stl_then_derives_scale_frame_and_local_items() {
        let path = write_triangle_stl();
        let digest = source_mesh_content_digest(&path).unwrap();
        let mut guide = crate::contracts::CaptureReconstructionGuide::test_fixture();
        guide.source_mesh.content_digest = digest.clone();
        guide.source_mesh.triangle_count = 1;
        let source_positions = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        for (landmark, source_position) in guide.landmarks.iter_mut().zip(source_positions) {
            landmark.anchor.source_mesh_content_digest = digest.clone();
            landmark.anchor.triangle_index = 0;
            landmark.anchor.source_position = source_position;
            landmark.anchor.barycentric = match source_position {
                [0.0, 0.0, 0.0] => [1.0, 0.0, 0.0],
                [2.0, 0.0, 0.0] => [0.0, 1.0, 0.0],
                _ => [0.0, 0.0, 1.0],
            };
        }
        guide.calibration.measurements = vec![crate::contracts::CaptureKnownDistanceMeasurement {
            measurement_id: "calibration-1".into(),
            label: "known span".into(),
            first_landmark_id: "landmark-1".into(),
            second_landmark_id: "landmark-2".into(),
            known_distance_mm: 40.0,
            fitted_distance_mm: 0.0,
            residual_mm: 0.0,
            accepted_tolerance_mm: 0.1,
        }];

        recompute_guide_geometry_from_stl(&path, &mut guide).unwrap();

        assert_eq!(guide.calibration.millimetres_per_source_unit, 20.0);
        assert_eq!(guide.reconstruction_frame.origin_mm, [0.0, 0.0, 0.0]);
        assert_eq!(guide.landmarks[1].local_position_mm, [40.0, 0.0, 0.0]);
        assert_eq!(guide.landmarks[2].local_position_mm, [0.0, 40.0, 0.0]);
        assert!(guide.planes[0].fit.max_mm <= 1.0e-9);
        assert_eq!(guide.surface_neighborhoods.len(), 3);
        assert!(guide
            .surface_neighborhoods
            .iter()
            .all(|neighborhood| neighborhood.source_mesh_content_digest == digest));
        assert!(guide
            .landmarks
            .iter()
            .all(|landmark| landmark.uncertainty_mm == Some(0.0)));
        assert_eq!(guide.primitive_candidates.len(), 1);
        assert_eq!(
            guide.primitive_candidates[0].candidate_id,
            "primitive:plane-1"
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn deterministic_stack_produces_regions_curved_profile_constraints_plan_and_readiness() {
        let path = write_triangle_stl();
        let digest = source_mesh_content_digest(&path).unwrap();
        let mut guide = crate::contracts::CaptureReconstructionGuide::test_fixture();
        guide.source_mesh.content_digest = digest.clone();
        guide.source_mesh.triangle_count = 1;
        guide.calibration.method =
            crate::contracts::CaptureCalibrationMethod::TrustedMetricMetadata {
                provenance: "fixture metric frame".into(),
                accepted_by_user: true,
            };
        let source_positions = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        for (landmark, source_position) in guide.landmarks.iter_mut().zip(source_positions) {
            landmark.anchor.source_mesh_content_digest = digest.clone();
            landmark.anchor.triangle_index = 0;
            landmark.anchor.source_position = source_position;
            landmark.anchor.barycentric = match source_position {
                [0.0, 0.0, 0.0] => [1.0, 0.0, 0.0],
                [2.0, 0.0, 0.0] => [0.0, 1.0, 0.0],
                _ => [0.0, 0.0, 1.0],
            };
        }
        guide
            .measurements
            .push(crate::contracts::CaptureNamedMeasurement {
                measurement_id: "depth".into(),
                label: "extrusion depth".into(),
                landmark_ids: vec!["landmark-1".into(), "landmark-2".into()],
                value: 12.0,
                unit: "mm".into(),
                fit_critical: true,
                authored_parameter_name: Some("insert-depth".into()),
                constraint_kind: Some(crate::contracts::CaptureConstraintKind::Extent),
            });

        recompute_guide_geometry_from_stl(&path, &mut guide).unwrap();

        assert_eq!(guide.surface_regions.len(), 1);
        assert_eq!(guide.surface_regions[0].triangle_indices, vec![0]);
        assert!(guide.region_adjacency.is_empty());
        assert_eq!(guide.reconstructed_profiles.len(), 1);
        assert!(guide.reconstructed_profiles[0].closed);
        assert!(guide.reconstructed_profiles[0].continuous);
        assert_eq!(guide.reconstructed_profiles[0].segments.len(), 3);
        assert!(guide.reconstructed_profiles[0]
            .segments
            .iter()
            .all(|segment| matches!(
                segment.geometry,
                crate::contracts::CaptureProfileSegmentGeometry::Line { .. }
            )));
        assert!(guide.constraint_graph.dimensions.iter().any(|dimension| {
            dimension.parameter_name.as_deref() == Some("insert-depth") && dimension.fit_critical
        }));
        assert_eq!(guide.feature_plan_candidates.len(), 1);
        assert_eq!(
            guide.selected_feature_plan_id.as_deref(),
            Some("plan:profile-1:extrude")
        );
        assert!(guide.reconstruction_readiness.ready);
        assert!(guide.reconstruction_readiness.missing_stages.is_empty());

        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn competing_feature_plans_require_explicit_selection_before_handoff_readiness() {
        let path = write_triangle_stl();
        let digest = source_mesh_content_digest(&path).unwrap();
        let mut guide = crate::contracts::CaptureReconstructionGuide::test_fixture();
        guide.source_mesh.content_digest = digest.clone();
        guide.source_mesh.triangle_count = 1;
        guide.calibration.method =
            crate::contracts::CaptureCalibrationMethod::TrustedMetricMetadata {
                provenance: "fixture metric frame".into(),
                accepted_by_user: true,
            };
        let source_positions = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        for (landmark, source_position) in guide.landmarks.iter_mut().zip(source_positions) {
            landmark.anchor.source_mesh_content_digest = digest.clone();
            landmark.anchor.triangle_index = 0;
            landmark.anchor.source_position = source_position;
            landmark.anchor.barycentric = match source_position {
                [0.0, 0.0, 0.0] => [1.0, 0.0, 0.0],
                [2.0, 0.0, 0.0] => [0.0, 1.0, 0.0],
                _ => [0.0, 0.0, 1.0],
            };
        }
        guide
            .measurements
            .push(crate::contracts::CaptureNamedMeasurement {
                measurement_id: "depth".into(),
                label: "depth".into(),
                landmark_ids: vec!["landmark-1".into(), "landmark-2".into()],
                value: 12.0,
                unit: "mm".into(),
                fit_critical: true,
                authored_parameter_name: Some("insert-depth".into()),
                constraint_kind: Some(crate::contracts::CaptureConstraintKind::Extent),
            });
        guide.axes.push(crate::contracts::CaptureNamedAxis {
            axis_id: "axis-1".into(),
            label: "rotation axis".into(),
            landmark_ids: vec!["landmark-1".into(), "landmark-2".into()],
            origin_mm: [0.0, 0.0, 0.0],
            direction: [1.0, 0.0, 0.0],
            fit: crate::contracts::CaptureFitResidual {
                rms_mm: 0.0,
                max_mm: 0.0,
                tolerance_mm: 0.1,
            },
        });
        guide.profiles[0].operation_hint =
            crate::contracts::CaptureProfileOperationHint::AgentDecide;

        recompute_guide_geometry_from_stl(&path, &mut guide).unwrap();
        assert!(!guide.reconstruction_readiness.ready);
        assert_eq!(
            guide.reconstruction_readiness.ambiguous_stages,
            vec![crate::contracts::CaptureReconstructionStage::FeaturePlan]
        );
        assert!(guide.selected_feature_plan_id.is_none());

        guide.selected_feature_plan_id = Some("plan:profile-1:extrude".into());
        recompute_guide_geometry_from_stl(&path, &mut guide).unwrap();
        assert!(guide.reconstruction_readiness.ready);
        assert_eq!(
            guide.selected_feature_plan_id.as_deref(),
            Some("plan:profile-1:extrude")
        );

        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn canonical_handoff_has_deterministic_local_views_and_parametric_brep_requirements() {
        let mut guide = crate::contracts::CaptureReconstructionGuide::test_fixture();
        guide.calibration.method =
            crate::contracts::CaptureCalibrationMethod::TrustedMetricMetadata {
                provenance: "fixture-metric".into(),
                accepted_by_user: true,
            };
        guide.target_source_digest = "sha256:target-source".into();
        guide.target_version_id = Some("version-7".into());
        guide.canonical_digest = guide.compute_canonical_digest().unwrap();

        let error = build_guided_reconstruction_request(
            &guide,
            "sha256:target-source",
            Some("version-7".into()),
        )
        .unwrap_err();
        assert!(error
            .message
            .contains("missing computed surface neighborhood"));

        let path = write_triangle_stl();
        let digest = source_mesh_content_digest(&path).unwrap();
        guide.source_mesh.content_digest = digest.clone();
        guide.source_mesh.triangle_count = 1;
        let source_positions = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        for (landmark, source_position) in guide.landmarks.iter_mut().zip(source_positions) {
            landmark.anchor.source_mesh_content_digest = digest.clone();
            landmark.anchor.triangle_index = 0;
            landmark.anchor.source_position = source_position;
            landmark.anchor.barycentric = match source_position {
                [0.0, 0.0, 0.0] => [1.0, 0.0, 0.0],
                [2.0, 0.0, 0.0] => [0.0, 1.0, 0.0],
                _ => [0.0, 0.0, 1.0],
            };
        }
        guide
            .measurements
            .push(crate::contracts::CaptureNamedMeasurement {
                measurement_id: "depth".into(),
                label: "extrusion depth".into(),
                landmark_ids: vec!["landmark-1".into(), "landmark-2".into()],
                value: 12.0,
                unit: "mm".into(),
                fit_critical: true,
                authored_parameter_name: Some("insert-depth".into()),
                constraint_kind: Some(crate::contracts::CaptureConstraintKind::Extent),
            });
        recompute_guide_geometry_from_stl(&path, &mut guide).unwrap();
        guide.canonical_digest = guide.compute_canonical_digest().unwrap();

        let first = build_guided_reconstruction_request(
            &guide,
            "sha256:target-source",
            Some("version-7".into()),
        )
        .unwrap();
        let second = build_guided_reconstruction_request(
            &guide,
            "sha256:target-source",
            Some("version-7".into()),
        )
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(
            first
                .evidence_views
                .iter()
                .map(|view| view.view_id.as_str())
                .collect::<Vec<_>>(),
            vec!["front", "right", "top", "isometric"]
        );
        assert!(first.requirements.require_parametric_source);
        assert!(first.requirements.forbid_mesh_solidification);
        let prompt = guided_reconstruction_prompt(&first).unwrap();
        assert!(prompt.contains("inspect -> validate -> preview -> commit"));
        assert!(prompt.contains("Do not solidify, mirror, patch, or export the scan mesh"));
        assert!(prompt.contains("captureGuidedResult"));
        assert!(prompt.contains("call user_confirm_request and do not commit"));
        assert!(prompt.len() < 65_536);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
