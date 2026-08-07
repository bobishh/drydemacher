use std::path::Path;

use csgrs::float_types::parry3d::{na::Point3, query::PointQuery, shape::TriMesh};
use sha2::{Digest, Sha256};

use crate::contracts::{
    AppError, AppResult, CaptureDeviationDisplaySample, CaptureDeviationPartIdentity,
    CaptureObservedDeviationReport, CaptureReconstructionGuide,
};
use crate::ecky_cad_host::analysis_boundary::AnalysisBoundarySurface;
use crate::ecky_ir::mesh_asset::{IndexedMeshAsset, MeshAssetSource};

const DEVIATION_REPORT_SCHEMA_VERSION: u32 = 1;
const MAX_DEVIATION_SAMPLES: usize = 100_000;
const MAX_DEVIATION_DISPLAY_SAMPLES: usize = 2_048;

pub fn compute_observed_mesh_to_brep_deviation(
    source_stl_path: &Path,
    guide: &CaptureReconstructionGuide,
    boundary: &AnalysisBoundarySurface,
    generated_geometry_digest: &str,
    maximum_samples: usize,
    outlier_threshold_mm: f64,
) -> AppResult<CaptureObservedDeviationReport> {
    compute_observed_mesh_to_brep_deviation_across_boundaries(
        source_stl_path,
        guide,
        std::slice::from_ref(boundary),
        generated_geometry_digest,
        maximum_samples,
        outlier_threshold_mm,
    )
}

pub fn compute_observed_mesh_to_brep_deviation_across_boundaries(
    source_stl_path: &Path,
    guide: &CaptureReconstructionGuide,
    boundaries: &[AnalysisBoundarySurface],
    generated_geometry_digest: &str,
    maximum_samples: usize,
    outlier_threshold_mm: f64,
) -> AppResult<CaptureObservedDeviationReport> {
    if maximum_samples == 0 || maximum_samples > MAX_DEVIATION_SAMPLES {
        return Err(AppError::validation(format!(
            "Observed deviation sample bound must be between 1 and {MAX_DEVIATION_SAMPLES}."
        )));
    }
    if !outlier_threshold_mm.is_finite() || outlier_threshold_mm < 0.0 {
        return Err(AppError::validation(
            "Observed deviation outlier threshold must be finite and non-negative.",
        ));
    }
    require_digest("generated geometry", generated_geometry_digest)?;
    if boundaries.is_empty() {
        return Err(AppError::validation(
            "Observed deviation requires at least one exact BRep analysis boundary.",
        ));
    }
    if guide
        .compute_canonical_digest()
        .map_err(AppError::validation)?
        != guide.canonical_digest
    {
        return Err(AppError::conflict(
            "Observed deviation guide canonical digest is stale.",
        ));
    }

    let mesh = IndexedMeshAsset::from_stl(MeshAssetSource::Imported, source_stl_path)?;
    if mesh.content_digest() != guide.source_mesh.content_digest {
        return Err(AppError::conflict(
            "Observed deviation source mesh digest differs from guide source mesh.",
        ));
    }
    let scale_mm = guide.calibration.millimetres_per_source_unit;
    if !scale_mm.is_finite() || scale_mm <= 0.0 {
        return Err(AppError::validation(
            "Observed deviation requires finite positive guide calibration scale.",
        ));
    }
    let source_points = deterministic_sample_indices(mesh.vertices().len(), maximum_samples)
        .into_iter()
        .map(|index| {
            let world = scale(mesh.vertices()[index], scale_mm);
            to_guide_local(guide, world).map(|local_position_mm| (index, local_position_mm))
        })
        .collect::<AppResult<Vec<_>>>()?;
    let mut ordered_boundaries = boundaries.iter().collect::<Vec<_>>();
    ordered_boundaries.sort_by(|left, right| left.part_id.cmp(&right.part_id));
    let mut parts = Vec::with_capacity(ordered_boundaries.len());
    let mut boundary_vertices = Vec::new();
    let mut boundary_triangles = Vec::new();
    for boundary in ordered_boundaries {
        if parts
            .last()
            .is_some_and(|previous: &CaptureDeviationPartIdentity| {
                previous.part_id == boundary.part_id
            })
        {
            return Err(AppError::validation(format!(
                "Observed deviation repeats analysis boundary part '{}'.",
                boundary.part_id
            )));
        }
        require_digest(
            "analysis boundary source geometry",
            &boundary.source_geometry_digest,
        )?;
        require_digest("analysis boundary", &boundary.content_digest)?;
        let offset = u32::try_from(boundary_vertices.len()).map_err(|_| {
            AppError::validation("Observed deviation analysis boundary vertex count overflowed.")
        })?;
        boundary_vertices.extend(
            boundary
                .vertices
                .iter()
                .map(|vertex| Point3::new(vertex[0], vertex[1], vertex[2])),
        );
        for triangle in &boundary.triangles {
            boundary_triangles.push([
                triangle[0].checked_add(offset).ok_or_else(|| {
                    AppError::validation("Observed deviation boundary index overflowed.")
                })?,
                triangle[1].checked_add(offset).ok_or_else(|| {
                    AppError::validation("Observed deviation boundary index overflowed.")
                })?,
                triangle[2].checked_add(offset).ok_or_else(|| {
                    AppError::validation("Observed deviation boundary index overflowed.")
                })?,
            ]);
        }
        parts.push(CaptureDeviationPartIdentity {
            part_id: boundary.part_id.clone(),
            source_geometry_digest: boundary.source_geometry_digest.clone(),
            analysis_boundary_digest: boundary.content_digest.clone(),
        });
    }
    let boundary_mesh = TriMesh::new(boundary_vertices, boundary_triangles).map_err(|error| {
        AppError::validation(format!(
            "Observed deviation BRep analysis boundary is invalid: {error}"
        ))
    })?;

    let residual_samples = source_points
        .iter()
        .map(|(source_vertex_index, local_position_mm)| {
            let point = Point3::new(
                local_position_mm[0],
                local_position_mm[1],
                local_position_mm[2],
            );
            let projected = boundary_mesh.project_local_point(&point, false);
            (
                *source_vertex_index,
                *local_position_mm,
                (point - projected.point).norm(),
            )
        })
        .collect::<Vec<_>>();
    if residual_samples.is_empty()
        || residual_samples
            .iter()
            .any(|(_, _, distance_mm)| !distance_mm.is_finite())
    {
        return Err(AppError::validation(
            "Observed deviation produced no finite samples.",
        ));
    }
    let display_samples =
        deterministic_sample_indices(residual_samples.len(), MAX_DEVIATION_DISPLAY_SAMPLES)
            .into_iter()
            .map(|sample_index| {
                let (source_vertex_index, local_position_mm, distance_mm) =
                    residual_samples[sample_index];
                Ok(CaptureDeviationDisplaySample {
                    source_vertex_index: u64::try_from(source_vertex_index).map_err(|_| {
                        AppError::validation("Observed deviation source vertex index overflowed.")
                    })?,
                    local_position_mm,
                    distance_mm,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
    let mut residuals = residual_samples
        .iter()
        .map(|(_, _, distance_mm)| *distance_mm)
        .collect::<Vec<_>>();
    let maximum_mm = residuals.iter().copied().fold(0.0, f64::max);
    let rms_mm =
        (residuals.iter().map(|value| value * value).sum::<f64>() / residuals.len() as f64).sqrt();
    let outlier_count = residuals
        .iter()
        .filter(|value| **value > outlier_threshold_mm)
        .count() as u64;
    residuals.sort_by(f64::total_cmp);
    let percentile_index = ((residuals.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(residuals.len() - 1);
    let percentile_95_mm = residuals[percentile_index];

    let mut report = CaptureObservedDeviationReport {
        schema_version: DEVIATION_REPORT_SCHEMA_VERSION,
        guide_id: guide.guide_id.clone(),
        guide_revision: guide.revision,
        guide_canonical_digest: guide.canonical_digest.clone(),
        source_mesh_content_digest: guide.source_mesh.content_digest.clone(),
        generated_geometry_digest: generated_geometry_digest.to_string(),
        parts,
        source_vertex_count: mesh.vertices().len() as u64,
        sample_count: residuals.len() as u64,
        maximum_mm,
        rms_mm,
        percentile_95_mm,
        outlier_threshold_mm,
        outlier_count,
        evidence_scope: "observedRegionOnly".into(),
        display_samples,
        content_digest: String::new(),
    };
    report.content_digest = report_digest(&report)?;
    Ok(report)
}

fn deterministic_sample_indices(vertex_count: usize, maximum_samples: usize) -> Vec<usize> {
    let sample_count = vertex_count.min(maximum_samples);
    if sample_count == 0 {
        return Vec::new();
    }
    (0..sample_count)
        .map(|index| index * vertex_count / sample_count)
        .collect()
}

fn to_guide_local(guide: &CaptureReconstructionGuide, world_mm: [f64; 3]) -> AppResult<[f64; 3]> {
    let frame = &guide.reconstruction_frame;
    let offset = sub(world_mm, frame.origin_mm);
    let local = [
        dot(offset, frame.x_axis),
        dot(offset, frame.y_axis),
        dot(offset, frame.z_axis),
    ];
    if local.iter().all(|value| value.is_finite()) {
        Ok(local)
    } else {
        Err(AppError::validation(
            "Observed deviation guide frame produced non-finite coordinates.",
        ))
    }
}

fn report_digest(report: &CaptureObservedDeviationReport) -> AppResult<String> {
    let mut canonical = report.clone();
    canonical.content_digest.clear();
    let bytes = serde_json::to_vec(&canonical).map_err(|error| {
        AppError::validation(format!(
            "Observed deviation report canonical serialization failed: {error}"
        ))
    })?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn require_digest(label: &str, digest: &str) -> AppResult<()> {
    if digest.starts_with("sha256:") && digest.len() > "sha256:".len() {
        Ok(())
    } else {
        Err(AppError::validation(format!(
            "Observed deviation {label} digest is invalid."
        )))
    }
}

fn sub(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[0] - second[0],
        first[1] - second[1],
        first[2] - second[2],
    ]
}

fn scale(value: [f64; 3], factor: f64) -> [f64; 3] {
    [value[0] * factor, value[1] * factor, value[2] * factor]
}

fn dot(first: [f64; 3], second: [f64; 3]) -> f64 {
    first[0] * second[0] + first[1] * second[1] + first[2] * second[2]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecky_cad_host::analysis_boundary::{
        AnalysisBoundaryEvidence, AnalysisBoundarySurface,
    };

    #[test]
    fn observed_deviation_is_bounded_deterministic_and_digest_bound() {
        let root = std::env::temp_dir().join(format!(
            "ecky-capture-deviation-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        let source_path = root.join("scan.stl");
        std::fs::write(
            &source_path,
            b"solid scan\nfacet normal 0 0 1\nouter loop\nvertex 0 0 1\nvertex 1 0 1\nvertex 0 1 1\nendloop\nendfacet\nendsolid scan\n",
        )
        .expect("scan STL");
        let mut guide = CaptureReconstructionGuide::test_fixture();
        guide.source_mesh.content_digest =
            crate::capture_guidance::source_mesh_content_digest(&source_path).expect("mesh digest");
        guide.canonical_digest = guide.compute_canonical_digest().expect("guide digest");
        let boundary = AnalysisBoundarySurface {
            part_id: "part-1".into(),
            label: "Part".into(),
            source_geometry_digest: "sha256:part-geometry".into(),
            vertices: vec![[-10.0, -10.0, 0.0], [20.0, -10.0, 0.0], [-10.0, 20.0, 0.0]],
            triangles: vec![[0, 1, 2]],
            triangle_face_group_indices: vec![0],
            face_groups: vec![],
            evidence: AnalysisBoundaryEvidence {
                closed: true,
                manifold: true,
                component_count: 1,
                positive_volume: true,
                boundary_edge_count: 0,
                non_manifold_edge_count: 0,
                winding_mismatch_count: 0,
                signed_volume: 1.0,
            },
            content_digest: "sha256:boundary".into(),
        };

        let report = compute_observed_mesh_to_brep_deviation(
            &source_path,
            &guide,
            &boundary,
            "sha256:generated-geometry",
            2,
            0.5,
        )
        .expect("deviation report");

        assert_eq!(report.source_vertex_count, 3);
        assert_eq!(report.sample_count, 2);
        assert_eq!(report.maximum_mm, 1.0);
        assert_eq!(report.rms_mm, 1.0);
        assert_eq!(report.percentile_95_mm, 1.0);
        assert_eq!(report.outlier_count, 2);
        assert_eq!(report.evidence_scope, "observedRegionOnly");
        assert_eq!(report.display_samples.len(), 2);
        assert_eq!(report.display_samples[0].source_vertex_index, 0);
        assert_eq!(report.display_samples[1].source_vertex_index, 1);
        assert_eq!(report.display_samples[0].distance_mm, 1.0);
        assert_eq!(report.parts.len(), 1);
        assert_eq!(report.parts[0].part_id, "part-1");
        assert!(report.content_digest.starts_with("sha256:"));

        let repeated = compute_observed_mesh_to_brep_deviation(
            &source_path,
            &guide,
            &boundary,
            "sha256:generated-geometry",
            2,
            0.5,
        )
        .expect("repeat report");
        assert_eq!(repeated, report);

        let mut matching_boundary = boundary.clone();
        matching_boundary.part_id = "part-2".into();
        matching_boundary.source_geometry_digest = "sha256:part-2-geometry".into();
        matching_boundary.content_digest = "sha256:part-2-boundary".into();
        for vertex in &mut matching_boundary.vertices {
            vertex[2] = 1.0;
        }
        let assembly = compute_observed_mesh_to_brep_deviation_across_boundaries(
            &source_path,
            &guide,
            &[matching_boundary, boundary],
            "sha256:generated-geometry",
            3,
            0.5,
        )
        .expect("assembly deviation report");
        assert_eq!(assembly.maximum_mm, 0.0);
        assert_eq!(
            assembly
                .parts
                .iter()
                .map(|part| part.part_id.as_str())
                .collect::<Vec<_>>(),
            vec!["part-1", "part-2"]
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
