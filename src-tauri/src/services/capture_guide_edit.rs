use crate::contracts::{
    AppError, AppResult, ApplyCaptureGuideEditInput, ApplyCaptureGuideEditResult,
    CaptureAuthoredSelector, CaptureCalibration, CaptureCalibrationMethod, CaptureConstraintGraph,
    CaptureEvidenceComputationPolicy, CaptureExpectedGeometryKind, CaptureFeaturePlanStatus,
    CaptureGuideEditIntent, CaptureReconstructionFrame, CaptureReconstructionGuide,
    CaptureReconstructionGuideState, CaptureReconstructionReadiness,
    CaptureRequiredBrepTopologyKind, CaptureSymmetryCompletion,
    EnsureCaptureReconstructionGuideResult, ValidateCaptureGuideIntentInput,
    ValidateCaptureGuideIntentResult, CAPTURE_RECONSTRUCTION_GUIDE_SCHEMA_VERSION,
};
use rusqlite::Connection;
use std::collections::HashSet;

pub fn ensure_capture_reconstruction_guide(
    conn: &Connection,
    run_id: &str,
) -> AppResult<EnsureCaptureReconstructionGuideResult> {
    let run = crate::capture_runs::get(conn, run_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| AppError::not_found("Capture run not found."))?;
    if let Some(guide) = run.reconstruction_guide.as_ref() {
        validate_guide_ownership(&run, &guide)?;
        return Ok(EnsureCaptureReconstructionGuideResult {
            guide: guide.clone(),
            state: run
                .reconstruction_guide_state
                .unwrap_or(CaptureReconstructionGuideState::Draft),
            created: false,
        });
    }

    let source_mesh = crate::capture_runs::selected_source_identity(conn, run_id)?;
    let source_mesh_digest = source_mesh.content_digest.clone();
    let mut guide = CaptureReconstructionGuide {
        schema_version: CAPTURE_RECONSTRUCTION_GUIDE_SCHEMA_VERSION,
        guide_id: uuid::Uuid::new_v4().to_string(),
        revision: 1,
        capture_run_id: run.id.clone(),
        target_thread_id: run.target_thread_id.clone(),
        target_message_id: run.target_message_id.clone(),
        target_source_digest: crate::services::render_snapshot::canonical_source_digest(
            &run.target_source,
        ),
        target_version_id: run.target_message_id.clone(),
        source_mesh,
        calibration: CaptureCalibration {
            source_units: "sourceUnit".into(),
            millimetres_per_source_unit: 1.0,
            method: CaptureCalibrationMethod::KnownDistance,
            measurements: Vec::new(),
            residual_mm: 0.0,
        },
        reconstruction_frame: CaptureReconstructionFrame {
            origin_mm: [0.0, 0.0, 0.0],
            x_axis: [1.0, 0.0, 0.0],
            y_axis: [0.0, 1.0, 0.0],
            z_axis: [0.0, 0.0, 1.0],
            source_landmark_ids: Vec::new(),
        },
        landmarks: Vec::new(),
        evidence_computation_policy: CaptureEvidenceComputationPolicy::default(),
        surface_neighborhoods: Vec::new(),
        primitive_candidates: Vec::new(),
        primitive_hypotheses: Vec::new(),
        surface_regions: Vec::new(),
        region_adjacency: Vec::new(),
        reconstructed_profiles: Vec::new(),
        feature_expectations: Vec::new(),
        measurements: Vec::new(),
        axes: Vec::new(),
        planes: Vec::new(),
        profiles: Vec::new(),
        ignored_regions: Vec::new(),
        authored_constraints: Vec::new(),
        constraint_graph: CaptureConstraintGraph::default(),
        feature_plan_candidates: Vec::new(),
        selected_feature_plan_id: None,
        stage_bypasses: Vec::new(),
        reconstruction_readiness: CaptureReconstructionReadiness::default(),
        remap_proposals: Vec::new(),
        symmetry_completion: CaptureSymmetryCompletion::None,
        instruction: String::new(),
        evidence_views: Vec::new(),
        canonical_digest: String::new(),
    };
    guide.canonical_digest = guide
        .compute_canonical_digest()
        .map_err(AppError::validation)?;
    let state = CaptureReconstructionGuideState::Draft;
    crate::capture_runs::append_capture_guide_version(
        conn,
        run_id,
        0,
        0,
        &source_mesh_digest,
        &source_mesh_digest,
        &guide,
        &state,
        &[],
    )?;

    Ok(EnsureCaptureReconstructionGuideResult {
        guide,
        state,
        created: true,
    })
}

pub fn apply_capture_guide_edit(
    conn: &Connection,
    input: ApplyCaptureGuideEditInput,
) -> AppResult<ApplyCaptureGuideEditResult> {
    let run = crate::capture_runs::get(conn, &input.run_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| AppError::not_found("Capture run not found."))?;
    let mut guide = run
        .reconstruction_guide
        .clone()
        .ok_or_else(|| AppError::validation("Capture run has no reconstruction guide."))?;
    validate_guide_ownership(&run, &guide)?;

    let base_revision = guide.revision;
    let expected_revision_matched = input.expected_revision == base_revision;
    let (selected_path, _) = crate::capture_runs::selected_source_path(conn, &input.run_id)?;
    let actual_mesh_digest = crate::capture_guidance::source_mesh_content_digest(&selected_path);
    let mut raw_evidence = Vec::new();
    if !expected_revision_matched {
        raw_evidence.push(format!(
            "Capture guide edit expected revision {}, current head was {}.",
            input.expected_revision, base_revision
        ));
    }
    if input.expected_mesh_digest != guide.source_mesh.content_digest {
        raw_evidence.push(format!(
            "Capture guide edit expected mesh digest '{}', current guide references '{}'.",
            input.expected_mesh_digest, guide.source_mesh.content_digest
        ));
    }

    apply_edit(&mut guide, input.edit)?;
    guide.revision = base_revision
        .checked_add(1)
        .ok_or_else(|| AppError::validation("Capture guide revision overflow."))?;

    let (current_mesh_digest, source_digest_matched, state) = match actual_mesh_digest {
        Ok(actual_mesh_digest) if actual_mesh_digest == guide.source_mesh.content_digest => {
            crate::capture_guidance::validate_guide_draft_from_stl(&selected_path, &mut guide)?;
            (
                actual_mesh_digest,
                input.expected_mesh_digest == guide.source_mesh.content_digest,
                CaptureReconstructionGuideState::Draft,
            )
        }
        Ok(actual_mesh_digest) => {
            let reason = format!(
                "Guide is stale: selected crop/source mesh digest changed from '{}' to '{}'.",
                guide.source_mesh.content_digest, actual_mesh_digest
            );
            raw_evidence.push(reason.clone());
            (
                actual_mesh_digest,
                false,
                CaptureReconstructionGuideState::Stale { reason },
            )
        }
        Err(error) => {
            let reason = format!(
                "Guide is stale: selected capture source could not be read: {}",
                error.message
            );
            raw_evidence.push(reason.clone());
            (
                "unavailable".into(),
                false,
                CaptureReconstructionGuideState::Stale { reason },
            )
        }
    };
    guide.canonical_digest = guide
        .compute_canonical_digest()
        .map_err(AppError::validation)?;
    crate::capture_runs::append_capture_guide_version(
        conn,
        &input.run_id,
        base_revision,
        input.expected_revision,
        &input.expected_mesh_digest,
        &current_mesh_digest,
        &guide,
        &state,
        &raw_evidence,
    )?;

    Ok(ApplyCaptureGuideEditResult {
        guide,
        state,
        base_revision,
        expected_revision_matched,
        source_digest_matched,
        raw_evidence,
    })
}

pub fn validate_capture_guide_intent(
    conn: &Connection,
    input: ValidateCaptureGuideIntentInput,
) -> AppResult<ValidateCaptureGuideIntentResult> {
    let run = crate::capture_runs::get(conn, &input.run_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| AppError::not_found("Capture run not found."))?;
    let current = run
        .reconstruction_guide
        .as_ref()
        .ok_or_else(|| AppError::validation("Capture run has no reconstruction guide."))?;
    validate_guide_ownership(&run, current)?;

    let base_revision = current.revision;
    let expected_revision_matched = input.expected_revision == base_revision;
    let mut raw_evidence = Vec::new();
    if !expected_revision_matched {
        raw_evidence.push(format!(
            "Capture guide validation expected revision {}, current head was {}.",
            input.expected_revision, base_revision
        ));
    }
    if input.expected_mesh_digest != current.source_mesh.content_digest {
        raw_evidence.push(format!(
            "Capture guide validation expected mesh digest '{}', current guide references '{}'.",
            input.expected_mesh_digest, current.source_mesh.content_digest
        ));
    }

    let mut guide = current.clone();
    let finalization = crate::capture_guidance::finalize_mechanical_guide_draft(
        guide.clone(),
        input.known_distance_mm,
        &input.instruction,
        input.feature_depth_mm,
    );
    let finalization_succeeded = match finalization {
        Ok(finalized) => {
            guide = finalized;
            true
        }
        Err(error) => {
            crate::capture_guidance::invalidate_computed_reconstruction_evidence(&mut guide);
            guide.instruction = input.instruction.trim().into();
            raw_evidence.push(format!(
                "Capture guide finalization failed: {}",
                error.message
            ));
            false
        }
    };
    guide.revision = base_revision
        .checked_add(1)
        .ok_or_else(|| AppError::validation("Capture guide revision overflow."))?;

    let selected_path = crate::capture_runs::selected_source_path(conn, &input.run_id);
    let (current_mesh_digest, source_digest_matched, state) = match selected_path {
        Ok((selected_path, _)) => {
            match crate::capture_guidance::source_mesh_content_digest(&selected_path) {
                Ok(actual_mesh_digest)
                    if actual_mesh_digest == guide.source_mesh.content_digest =>
                {
                    let source_digest_matched =
                        input.expected_mesh_digest == guide.source_mesh.content_digest;
                    let state = if finalization_succeeded {
                        match crate::capture_guidance::recompute_guide_geometry_from_stl(
                            &selected_path,
                            &mut guide,
                        ) {
                            Ok(()) if guide.reconstruction_readiness.ready => {
                                CaptureReconstructionGuideState::Ready
                            }
                            Ok(()) => CaptureReconstructionGuideState::Draft,
                            Err(error) => {
                                raw_evidence.push(format!(
                                    "Capture guide deterministic evaluation failed: {}",
                                    error.message
                                ));
                                CaptureReconstructionGuideState::Draft
                            }
                        }
                    } else {
                        CaptureReconstructionGuideState::Draft
                    };
                    (actual_mesh_digest, source_digest_matched, state)
                }
                Ok(actual_mesh_digest) => {
                    let reason = format!(
                        "Guide is stale: selected crop/source mesh digest changed from '{}' to '{}'.",
                        guide.source_mesh.content_digest, actual_mesh_digest
                    );
                    raw_evidence.push(reason.clone());
                    (
                        actual_mesh_digest,
                        false,
                        CaptureReconstructionGuideState::Stale { reason },
                    )
                }
                Err(error) => {
                    let reason = format!(
                        "Guide is stale: selected capture source could not be read: {}",
                        error.message
                    );
                    raw_evidence.push(reason.clone());
                    (
                        "unavailable".into(),
                        false,
                        CaptureReconstructionGuideState::Stale { reason },
                    )
                }
            }
        }
        Err(error) => {
            let reason = format!(
                "Guide is stale: selected capture source is unavailable: {}",
                error.message
            );
            raw_evidence.push(reason.clone());
            (
                "unavailable".into(),
                false,
                CaptureReconstructionGuideState::Stale { reason },
            )
        }
    };
    guide.canonical_digest = guide
        .compute_canonical_digest()
        .map_err(AppError::validation)?;
    crate::capture_runs::append_capture_guide_version(
        conn,
        &input.run_id,
        base_revision,
        input.expected_revision,
        &input.expected_mesh_digest,
        &current_mesh_digest,
        &guide,
        &state,
        &raw_evidence,
    )?;

    Ok(ValidateCaptureGuideIntentResult {
        guide,
        state,
        base_revision,
        expected_revision_matched,
        source_digest_matched,
        raw_evidence,
    })
}

fn validate_guide_ownership(
    run: &crate::contracts::CaptureRun,
    guide: &CaptureReconstructionGuide,
) -> AppResult<()> {
    if guide.capture_run_id != run.id
        || guide.target_thread_id != run.target_thread_id
        || guide.target_message_id != run.target_message_id
    {
        return Err(AppError::conflict(
            "Capture guide ownership differs from capture run.",
        ));
    }
    let target_source_digest =
        crate::services::render_snapshot::canonical_source_digest(&run.target_source);
    if guide.target_source_digest != target_source_digest
        || guide.target_version_id != run.target_message_id
    {
        return Err(AppError::conflict(
            "Capture guide target source/version differs from capture run.",
        ));
    }
    Ok(())
}

fn apply_edit(
    guide: &mut CaptureReconstructionGuide,
    edit: CaptureGuideEditIntent,
) -> AppResult<()> {
    let invalidate_computed_evidence = match edit {
        CaptureGuideEditIntent::AddLandmark { role, anchor } => {
            if anchor.source_mesh_content_digest != guide.source_mesh.content_digest {
                return Err(AppError::conflict(
                    "Capture anchor mesh digest differs from guide source mesh.",
                ));
            }
            let next_ordinal = guide
                .landmarks
                .iter()
                .filter_map(|landmark| {
                    landmark
                        .landmark_id
                        .strip_prefix("landmark-")
                        .and_then(|value| value.parse::<u64>().ok())
                })
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or_else(|| AppError::validation("Capture landmark ordinal overflow."))?;
            let role_label = serde_json::to_value(&role)
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .ok_or_else(|| {
                    AppError::validation("Capture landmark role could not be encoded.")
                })?;
            guide.landmarks.push(crate::contracts::CaptureLandmark {
                landmark_id: format!("landmark-{next_ordinal}"),
                label: format!("{role_label} {next_ordinal}"),
                role,
                local_position_mm: anchor.source_position,
                local_normal: anchor.source_normal,
                anchor,
                uncertainty_mm: None,
            });
            true
        }
        CaptureGuideEditIntent::UpdateLandmark {
            landmark_id,
            label,
            role,
        } => {
            let landmark = guide
                .landmarks
                .iter_mut()
                .find(|landmark| landmark.landmark_id == landmark_id)
                .ok_or_else(|| {
                    AppError::not_found(format!("Capture landmark '{landmark_id}' does not exist."))
                })?;
            let label = label.trim();
            if label.is_empty() {
                return Err(AppError::validation("Capture landmark label is required."));
            }
            landmark.label = label.into();
            landmark.role = role;
            true
        }
        CaptureGuideEditIntent::DeleteLandmark { landmark_id } => {
            if !guide
                .landmarks
                .iter()
                .any(|landmark| landmark.landmark_id == landmark_id)
            {
                return Err(AppError::not_found(format!(
                    "Capture landmark '{landmark_id}' does not exist."
                )));
            }
            guide
                .landmarks
                .retain(|landmark| landmark.landmark_id != landmark_id);
            remove_landmark_references(guide, &landmark_id);
            true
        }
        CaptureGuideEditIntent::ReplaceDraft { guide: replacement } => {
            validate_replacement_identity(guide, &replacement)?;
            let current_revision = guide.revision;
            *guide = *replacement;
            guide.revision = current_revision;
            guide.canonical_digest.clear();
            true
        }
        CaptureGuideEditIntent::ConfigureProfile {
            profile_id,
            label,
            profile_kind,
            operation_hint,
            support_plane_id,
            feature_label,
            fit_role,
        } => {
            if !guide
                .planes
                .iter()
                .any(|plane| plane.plane_id == support_plane_id)
            {
                return Err(AppError::validation(format!(
                    "Capture profile '{profile_id}' references missing support plane '{support_plane_id}'."
                )));
            }
            let profile = guide
                .profiles
                .iter_mut()
                .find(|profile| profile.profile_id == profile_id)
                .ok_or_else(|| {
                    AppError::not_found(format!("Capture profile '{profile_id}' does not exist."))
                })?;
            let label = required_trimmed("Capture profile label", label)?;
            profile.label = label;
            profile.kind = profile_kind;
            profile.operation_hint = operation_hint;
            profile.support_plane_id = support_plane_id;
            profile.feature_label = optional_trimmed(feature_label);
            profile.fit_role = optional_trimmed(fit_role);
            true
        }
        CaptureGuideEditIntent::ReorderProfileLandmark {
            profile_id,
            landmark_id,
            target_index,
        } => {
            let profile = guide
                .profiles
                .iter_mut()
                .find(|profile| profile.profile_id == profile_id)
                .ok_or_else(|| {
                    AppError::not_found(format!("Capture profile '{profile_id}' does not exist."))
                })?;
            let source_index = profile
                .landmark_ids
                .iter()
                .position(|id| id == &landmark_id)
                .ok_or_else(|| {
                    AppError::validation(format!(
                        "Capture profile '{profile_id}' does not contain landmark '{landmark_id}'."
                    ))
                })?;
            let moved = profile.landmark_ids.remove(source_index);
            let target_index = usize::try_from(target_index)
                .unwrap_or(usize::MAX)
                .min(profile.landmark_ids.len());
            profile.landmark_ids.insert(target_index, moved);
            true
        }
        CaptureGuideEditIntent::UpdateFeatureExpectation {
            expectation_id,
            label,
            expected_geometry_kind,
            required_brep_topology_kind,
            cardinality,
            part_id,
            instance_path,
            expected_authored_selector,
            required_for_acceptance,
            position_tolerance_mm,
            normal_tolerance_deg,
            radial_tolerance_mm,
        } => {
            validate_expected_topology(
                &expectation_id,
                &expected_geometry_kind,
                &required_brep_topology_kind,
            )?;
            validate_optional_tolerance("position tolerance", position_tolerance_mm)?;
            validate_optional_tolerance("normal tolerance", normal_tolerance_deg)?;
            validate_optional_tolerance("radial tolerance", radial_tolerance_mm)?;
            let expectation = guide
                .feature_expectations
                .iter_mut()
                .find(|expectation| expectation.expectation_id == expectation_id)
                .ok_or_else(|| {
                    AppError::not_found(format!(
                        "Capture expectation '{expectation_id}' does not exist."
                    ))
                })?;
            expectation.label = required_trimmed("Capture expectation label", label)?;
            expectation.expected_geometry_kind = expected_geometry_kind;
            expectation.required_brep_topology_kind = required_brep_topology_kind;
            expectation.cardinality = cardinality;
            expectation.part_id = required_trimmed("Capture expectation part", part_id)?;
            expectation.instance_path = optional_trimmed(instance_path);
            expectation.expected_authored_selector =
                trim_authored_selector(expected_authored_selector)?;
            expectation.required_for_acceptance = required_for_acceptance;
            expectation.position_tolerance_mm = position_tolerance_mm;
            expectation.normal_tolerance_deg = normal_tolerance_deg;
            expectation.radial_tolerance_mm = radial_tolerance_mm;
            true
        }
        CaptureGuideEditIntent::SelectFeaturePlan { plan_id } => {
            if let Some(plan_id) = plan_id.as_deref() {
                let plan = guide
                    .feature_plan_candidates
                    .iter()
                    .find(|plan| plan.plan_id == plan_id)
                    .ok_or_else(|| AppError::not_found("Selected feature plan does not exist."))?;
                if plan.status == CaptureFeaturePlanStatus::Rejected {
                    return Err(AppError::validation(
                        "Rejected capture feature plan cannot be selected.",
                    ));
                }
            }
            guide.selected_feature_plan_id = plan_id.clone();
            guide.reconstruction_readiness.ready = false;
            guide.reconstruction_readiness.selected_feature_plan_id = plan_id;
            guide.reconstruction_readiness.detail =
                "Feature plan selection changed; deterministic readiness must be reevaluated."
                    .into();
            false
        }
    };
    if invalidate_computed_evidence {
        crate::capture_guidance::invalidate_computed_reconstruction_evidence(guide);
    }
    Ok(())
}

fn validate_replacement_identity(
    current: &CaptureReconstructionGuide,
    replacement: &CaptureReconstructionGuide,
) -> AppResult<()> {
    if replacement.schema_version != current.schema_version
        || replacement.guide_id != current.guide_id
        || replacement.capture_run_id != current.capture_run_id
        || replacement.target_thread_id != current.target_thread_id
        || replacement.target_message_id != current.target_message_id
        || replacement.target_source_digest != current.target_source_digest
        || replacement.target_version_id != current.target_version_id
        || replacement.source_mesh != current.source_mesh
    {
        return Err(AppError::conflict(
            "Replacement capture guide identity differs from current head.",
        ));
    }
    Ok(())
}

fn required_trimmed(label: &str, value: String) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::validation(format!("{label} is required.")));
    }
    Ok(value.into())
}

fn optional_trimmed(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_owned())
    })
}

fn trim_authored_selector(selector: CaptureAuthoredSelector) -> AppResult<CaptureAuthoredSelector> {
    match selector {
        CaptureAuthoredSelector::Binding { name } => Ok(CaptureAuthoredSelector::Binding {
            name: required_trimmed("Capture authored binding", name)?,
        }),
        CaptureAuthoredSelector::Tag { name } => Ok(CaptureAuthoredSelector::Tag {
            name: required_trimmed("Capture authored tag", name)?,
        }),
    }
}

fn validate_optional_tolerance(label: &str, value: Option<f64>) -> AppResult<()> {
    if value.is_some_and(|value| !value.is_finite() || value < 0.0) {
        return Err(AppError::validation(format!(
            "Capture expectation {label} must be finite and non-negative."
        )));
    }
    Ok(())
}

fn validate_expected_topology(
    expectation_id: &str,
    geometry_kind: &CaptureExpectedGeometryKind,
    topology_kind: &CaptureRequiredBrepTopologyKind,
) -> AppResult<()> {
    let valid = match geometry_kind {
        CaptureExpectedGeometryKind::Point => {
            topology_kind == &CaptureRequiredBrepTopologyKind::Vertex
        }
        CaptureExpectedGeometryKind::Curve => {
            topology_kind == &CaptureRequiredBrepTopologyKind::Edge
        }
        CaptureExpectedGeometryKind::Plane => {
            topology_kind == &CaptureRequiredBrepTopologyKind::Face
        }
        CaptureExpectedGeometryKind::Cylinder => matches!(
            topology_kind,
            CaptureRequiredBrepTopologyKind::Face | CaptureRequiredBrepTopologyKind::Edge
        ),
        CaptureExpectedGeometryKind::Profile => {
            topology_kind == &CaptureRequiredBrepTopologyKind::OrderedEdges
        }
    };
    if !valid {
        return Err(AppError::validation(format!(
            "Feature expectation '{expectation_id}' has incompatible analytic geometry and BRep topology kinds."
        )));
    }
    Ok(())
}

fn remove_landmark_references(guide: &mut CaptureReconstructionGuide, landmark_id: &str) {
    let landmark_ids = guide
        .landmarks
        .iter()
        .map(|landmark| landmark.landmark_id.as_str())
        .collect::<HashSet<_>>();
    guide.calibration.measurements.retain(|measurement| {
        landmark_ids.contains(measurement.first_landmark_id.as_str())
            && landmark_ids.contains(measurement.second_landmark_id.as_str())
    });
    guide
        .reconstruction_frame
        .source_landmark_ids
        .retain(|id| landmark_ids.contains(id.as_str()));
    guide.measurements.retain(|item| {
        item.landmark_ids
            .iter()
            .all(|id| landmark_ids.contains(id.as_str()))
    });
    guide.axes.retain(|item| {
        item.landmark_ids
            .iter()
            .all(|id| landmark_ids.contains(id.as_str()))
    });
    guide.planes.retain(|item| {
        item.landmark_ids
            .iter()
            .all(|id| landmark_ids.contains(id.as_str()))
    });
    guide.profiles.retain(|item| {
        item.landmark_ids
            .iter()
            .all(|id| landmark_ids.contains(id.as_str()))
    });
    guide.ignored_regions.retain(|item| {
        item.landmark_ids
            .iter()
            .all(|id| landmark_ids.contains(id.as_str()))
    });
    guide
        .authored_constraints
        .retain(|constraint| !constraint.entity_ids.iter().any(|id| id == landmark_id));
    guide
        .remap_proposals
        .retain(|proposal| proposal.landmark_id != landmark_id);

    let guide_item_ids = guide
        .landmarks
        .iter()
        .map(|item| item.landmark_id.as_str())
        .chain(
            guide
                .measurements
                .iter()
                .map(|item| item.measurement_id.as_str()),
        )
        .chain(guide.axes.iter().map(|item| item.axis_id.as_str()))
        .chain(guide.planes.iter().map(|item| item.plane_id.as_str()))
        .chain(guide.profiles.iter().map(|item| item.profile_id.as_str()))
        .chain(
            guide
                .ignored_regions
                .iter()
                .map(|item| item.region_id.as_str()),
        )
        .collect::<HashSet<_>>();
    guide.feature_expectations.retain(|expectation| {
        expectation
            .guide_item_ids
            .iter()
            .all(|id| guide_item_ids.contains(id.as_str()))
    });
    match &guide.symmetry_completion {
        CaptureSymmetryCompletion::Half { plane_id }
            if !guide.planes.iter().any(|plane| plane.plane_id == *plane_id) =>
        {
            guide.symmetry_completion = CaptureSymmetryCompletion::None;
        }
        CaptureSymmetryCompletion::Quarter {
            first_plane_id,
            second_plane_id,
        } if !guide
            .planes
            .iter()
            .any(|plane| plane.plane_id == *first_plane_id)
            || !guide
                .planes
                .iter()
                .any(|plane| plane.plane_id == *second_plane_id) =>
        {
            guide.symmetry_completion = CaptureSymmetryCompletion::None;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use crate::contracts::{
        ApplyCaptureGuideEditInput, CaptureAuthoredSelector, CaptureExpectedGeometryKind,
        CaptureFeatureOperation, CaptureFeaturePlanCandidate, CaptureFeaturePlanStatus,
        CaptureGuideEditIntent, CaptureLandmarkRole, CaptureMeshPreview, CaptureProfileKind,
        CaptureProfileOperationHint, CaptureReconstructionGuide, CaptureReconstructionGuideState,
        CaptureRequiredBrepTopologyKind, CaptureRun, CaptureSelectorCardinality,
        CaptureSessionState, ValidateCaptureGuideIntentInput,
    };

    struct Fixture {
        root: std::path::PathBuf,
        conn: rusqlite::Connection,
        run: CaptureRun,
        mesh_digest: String,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn fixture_with_guide(has_guide: bool) -> Fixture {
        let root =
            std::env::temp_dir().join(format!("ecky-capture-guide-edit-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let stl_path = root.join("source.stl");
        std::fs::write(
            &stl_path,
            "solid source\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid source\n",
        )
        .unwrap();
        let mesh_digest = crate::capture_guidance::source_mesh_content_digest(&stl_path).unwrap();
        let conn = crate::db::init_db(&root.join("history.sqlite")).unwrap();
        let mut guide = CaptureReconstructionGuide::test_fixture();
        guide.capture_run_id = "run-guide-edit".into();
        guide.target_thread_id = "thread-guide-edit".into();
        guide.target_message_id = None;
        guide.target_version_id = None;
        guide.target_source_digest =
            crate::services::render_snapshot::canonical_source_digest("(solid blank)");
        guide.source_mesh.content_digest = mesh_digest.clone();
        guide.source_mesh.triangle_count = 1;
        guide.revision = 4;
        for (index, landmark) in guide.landmarks.iter_mut().enumerate() {
            landmark.anchor.source_mesh_content_digest = mesh_digest.clone();
            landmark.anchor.triangle_index = 0;
            landmark.anchor.barycentric = match index {
                0 => [1.0, 0.0, 0.0],
                1 => [0.0, 1.0, 0.0],
                _ => [0.0, 0.0, 1.0],
            };
        }
        guide.canonical_digest = guide.compute_canonical_digest().unwrap();
        let run = CaptureRun {
            id: guide.capture_run_id.clone(),
            target_thread_id: guide.target_thread_id.clone(),
            target_message_id: None,
            title: "Guide edit".into(),
            state: CaptureSessionState::Preview,
            created_at: 1,
            updated_at: 1,
            accepted_frame_count: 1,
            mesh_preview: Some(CaptureMeshPreview {
                stl_path: stl_path.to_string_lossy().into_owned(),
                triangle_count: 1,
                bounds_mm: [1.0, 1.0, 0.0],
                scale_label: "source".into(),
                warnings: vec![],
            }),
            derived_stl_path: None,
            crop_bounds: None,
            preview_scale: 1.0,
            target_source: "(solid blank)".into(),
            target_source_language: "ecky".into(),
            started_from_empty: true,
            raw_error: None,
            reconstruction_guide: has_guide.then_some(guide),
            reconstruction_guide_state: has_guide.then_some(CaptureReconstructionGuideState::Ready),
            guided_reconstruction_message_id: None,
            guided_reconstruction_model_id: None,
            guided_reconstruction_result: None,
            guided_reconstruction_deviation: None,
        };
        crate::capture_runs::insert(&conn, &run).unwrap();
        Fixture {
            root,
            conn,
            run,
            mesh_digest,
        }
    }

    fn fixture() -> Fixture {
        fixture_with_guide(true)
    }

    fn fixture_without_guide() -> Fixture {
        fixture_with_guide(false)
    }

    fn install_mechanical_guide(fixture: &Fixture, ambiguous_feature_plan: bool) {
        let mut run = fixture.run.clone();
        let mut guide = run.reconstruction_guide.clone().unwrap();
        let template = guide.landmarks[0].clone();
        let mut evidence = vec![
            (
                CaptureLandmarkRole::CalibrationEndpoint,
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
            ),
            (
                CaptureLandmarkRole::CalibrationEndpoint,
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ),
            (
                CaptureLandmarkRole::FrameOrigin,
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
            ),
            (
                CaptureLandmarkRole::FrameDirection,
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ),
            (
                CaptureLandmarkRole::FrameDirection,
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ),
            (
                CaptureLandmarkRole::SymmetrySample,
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
            ),
            (
                CaptureLandmarkRole::SymmetrySample,
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ),
            (
                CaptureLandmarkRole::SymmetrySample,
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ),
            (
                CaptureLandmarkRole::ProfileVertex,
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
            ),
            (
                CaptureLandmarkRole::ProfileVertex,
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ),
            (
                CaptureLandmarkRole::ProfileVertex,
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ),
        ];
        if ambiguous_feature_plan {
            evidence.extend([
                (
                    CaptureLandmarkRole::RotationAxisEndpoint,
                    [0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                ),
                (
                    CaptureLandmarkRole::RotationAxisEndpoint,
                    [1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                ),
            ]);
        }
        guide.landmarks = evidence
            .into_iter()
            .enumerate()
            .map(|(index, (role, position, barycentric))| {
                let mut landmark = template.clone();
                landmark.landmark_id = format!("landmark-{}", index + 1);
                landmark.label = landmark.landmark_id.clone();
                landmark.role = role;
                landmark.anchor.source_mesh_content_digest = fixture.mesh_digest.clone();
                landmark.anchor.triangle_index = 0;
                landmark.anchor.barycentric = barycentric;
                landmark.anchor.source_position = position;
                landmark.local_position_mm = position;
                landmark
            })
            .collect();
        if ambiguous_feature_plan {
            let mut profile = guide.profiles[0].clone();
            profile.support_plane_id = "symmetry-plane-1".into();
            profile.landmark_ids = vec![
                "landmark-9".into(),
                "landmark-10".into(),
                "landmark-11".into(),
            ];
            profile.operation_hint = CaptureProfileOperationHint::AgentDecide;
            guide.profiles = vec![profile];
        } else {
            guide.profiles.clear();
        }
        guide.canonical_digest = guide.compute_canonical_digest().unwrap();
        run.reconstruction_guide = Some(guide);
        run.reconstruction_guide_state = Some(CaptureReconstructionGuideState::Draft);
        crate::capture_runs::insert(&fixture.conn, &run).unwrap();
    }

    #[test]
    fn given_ready_guide_when_landmark_is_edited_then_rust_invalidates_and_appends_draft() {
        let fixture = fixture();

        let result = super::apply_capture_guide_edit(
            &fixture.conn,
            ApplyCaptureGuideEditInput {
                run_id: fixture.run.id.clone(),
                expected_revision: 4,
                expected_mesh_digest: fixture.mesh_digest.clone(),
                edit: CaptureGuideEditIntent::UpdateLandmark {
                    landmark_id: "landmark-1".into(),
                    label: "  datum origin  ".into(),
                    role: CaptureLandmarkRole::FrameOrigin,
                },
            },
        )
        .unwrap();

        assert_eq!(result.guide.revision, 5);
        assert_eq!(result.guide.landmarks[0].label, "datum origin");
        assert_eq!(
            result.guide.landmarks[0].role,
            CaptureLandmarkRole::FrameOrigin
        );
        assert!(result.guide.surface_neighborhoods.is_empty());
        assert!(result.guide.feature_plan_candidates.is_empty());
        assert_eq!(result.guide.selected_feature_plan_id, None);
        assert_eq!(result.state, CaptureReconstructionGuideState::Draft);
        assert!(result.expected_revision_matched);
        assert!(result.source_digest_matched);
        assert_eq!(
            crate::capture_runs::capture_guide_version_count(&fixture.conn, &fixture.run.id)
                .unwrap(),
            1
        );
    }

    #[test]
    fn given_stale_expected_revision_when_landmark_is_edited_then_current_head_still_appends() {
        let fixture = fixture();

        let result = super::apply_capture_guide_edit(
            &fixture.conn,
            ApplyCaptureGuideEditInput {
                run_id: fixture.run.id.clone(),
                expected_revision: 1,
                expected_mesh_digest: fixture.mesh_digest.clone(),
                edit: CaptureGuideEditIntent::UpdateLandmark {
                    landmark_id: "landmark-2".into(),
                    label: "new head label".into(),
                    role: CaptureLandmarkRole::NamedReference,
                },
            },
        )
        .unwrap();

        assert_eq!(result.guide.revision, 5);
        assert!(!result.expected_revision_matched);
        assert_eq!(result.base_revision, 4);
        assert_eq!(
            crate::capture_runs::get(&fixture.conn, &fixture.run.id)
                .unwrap()
                .unwrap()
                .reconstruction_guide
                .unwrap(),
            result.guide
        );
    }

    #[test]
    fn given_source_drift_when_safe_label_edit_arrives_then_stale_candidate_still_appends() {
        let fixture = fixture();
        let source_path = fixture
            .run
            .mesh_preview
            .as_ref()
            .map(|preview| std::path::PathBuf::from(&preview.stl_path))
            .unwrap();
        std::fs::write(
            source_path,
            "solid changed\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 2 0 0\nvertex 0 2 0\nendloop\nendfacet\nendsolid changed\n",
        )
        .unwrap();

        let result = super::apply_capture_guide_edit(
            &fixture.conn,
            ApplyCaptureGuideEditInput {
                run_id: fixture.run.id.clone(),
                expected_revision: 4,
                expected_mesh_digest: fixture.mesh_digest.clone(),
                edit: CaptureGuideEditIntent::UpdateLandmark {
                    landmark_id: "landmark-3".into(),
                    label: "preserved stale edit".into(),
                    role: CaptureLandmarkRole::NamedReference,
                },
            },
        )
        .unwrap();

        assert_eq!(result.guide.revision, 5);
        assert!(!result.source_digest_matched);
        assert!(matches!(
            result.state,
            CaptureReconstructionGuideState::Stale { .. }
        ));
        assert!(result
            .raw_evidence
            .iter()
            .any(|evidence| evidence.contains("selected crop/source mesh digest changed")));
        assert_eq!(
            crate::capture_runs::capture_guide_version_count(&fixture.conn, &fixture.run.id)
                .unwrap(),
            1
        );
    }

    #[test]
    fn given_surface_pick_when_landmark_is_added_then_rust_assigns_identity_and_appends() {
        let fixture = fixture();
        let anchor = fixture.run.reconstruction_guide.as_ref().unwrap().landmarks[0]
            .anchor
            .clone();

        let result = super::apply_capture_guide_edit(
            &fixture.conn,
            ApplyCaptureGuideEditInput {
                run_id: fixture.run.id.clone(),
                expected_revision: 4,
                expected_mesh_digest: fixture.mesh_digest.clone(),
                edit: CaptureGuideEditIntent::AddLandmark {
                    role: CaptureLandmarkRole::NamedReference,
                    anchor,
                },
            },
        )
        .unwrap();

        let added = result.guide.landmarks.last().unwrap();
        assert_eq!(added.landmark_id, "landmark-4");
        assert_eq!(added.label, "namedReference 4");
        assert_eq!(added.role, CaptureLandmarkRole::NamedReference);
        assert_eq!(result.guide.revision, 5);
        assert!(result.guide.feature_plan_candidates.is_empty());
    }

    #[test]
    fn given_profile_and_expectation_edits_when_applied_then_rust_mutates_current_head() {
        let fixture = fixture();
        let configured = super::apply_capture_guide_edit(
            &fixture.conn,
            ApplyCaptureGuideEditInput {
                run_id: fixture.run.id.clone(),
                expected_revision: 4,
                expected_mesh_digest: fixture.mesh_digest.clone(),
                edit: CaptureGuideEditIntent::ConfigureProfile {
                    profile_id: "profile-1".into(),
                    label: "  mating outline  ".into(),
                    profile_kind: CaptureProfileKind::Open,
                    operation_hint: CaptureProfileOperationHint::Sweep,
                    support_plane_id: "plane-1".into(),
                    feature_label: Some("  flange  ".into()),
                    fit_role: Some("  mating  ".into()),
                },
            },
        )
        .unwrap();
        assert_eq!(configured.guide.profiles[0].label, "mating outline");
        assert_eq!(
            configured.guide.profiles[0].feature_label.as_deref(),
            Some("flange")
        );

        let reordered = super::apply_capture_guide_edit(
            &fixture.conn,
            ApplyCaptureGuideEditInput {
                run_id: fixture.run.id.clone(),
                expected_revision: 5,
                expected_mesh_digest: fixture.mesh_digest.clone(),
                edit: CaptureGuideEditIntent::ReorderProfileLandmark {
                    profile_id: "profile-1".into(),
                    landmark_id: "landmark-3".into(),
                    target_index: 0,
                },
            },
        )
        .unwrap();
        assert_eq!(
            reordered.guide.profiles[0].landmark_ids,
            ["landmark-3", "landmark-1", "landmark-2"]
        );

        let updated = super::apply_capture_guide_edit(
            &fixture.conn,
            ApplyCaptureGuideEditInput {
                run_id: fixture.run.id.clone(),
                expected_revision: 6,
                expected_mesh_digest: fixture.mesh_digest.clone(),
                edit: CaptureGuideEditIntent::UpdateFeatureExpectation {
                    expectation_id: "expectation-1".into(),
                    label: "  exact support  ".into(),
                    expected_geometry_kind: CaptureExpectedGeometryKind::Plane,
                    required_brep_topology_kind: CaptureRequiredBrepTopologyKind::Face,
                    cardinality: CaptureSelectorCardinality::One,
                    part_id: "  part-1  ".into(),
                    instance_path: Some("  root/insert  ".into()),
                    expected_authored_selector: CaptureAuthoredSelector::Tag {
                        name: "  support-face  ".into(),
                    },
                    required_for_acceptance: true,
                    position_tolerance_mm: Some(0.2),
                    normal_tolerance_deg: Some(2.0),
                    radial_tolerance_mm: None,
                },
            },
        )
        .unwrap();
        let expectation = &updated.guide.feature_expectations[0];
        assert_eq!(expectation.label, "exact support");
        assert_eq!(expectation.part_id, "part-1");
        assert_eq!(expectation.instance_path.as_deref(), Some("root/insert"));
        assert_eq!(
            expectation.expected_authored_selector,
            CaptureAuthoredSelector::Tag {
                name: "support-face".into()
            }
        );
        assert_eq!(updated.guide.revision, 7);
    }

    #[test]
    fn given_supported_feature_plan_when_selected_then_candidates_remain_canonical() {
        let fixture = fixture();
        let mut guide = fixture.run.reconstruction_guide.clone().unwrap();
        guide.feature_plan_candidates = vec![CaptureFeaturePlanCandidate {
            plan_id: "plan-1".into(),
            label: "mirror support".into(),
            operations: vec![CaptureFeatureOperation::Mirror {
                plane_id: "plane-1".into(),
            }],
            supporting_evidence_ids: vec!["plane-1".into()],
            rejecting_evidence: vec![],
            score: 1.0,
            status: CaptureFeaturePlanStatus::Supported,
        }];
        crate::capture_runs::insert(
            &fixture.conn,
            &CaptureRun {
                reconstruction_guide: Some(guide),
                ..fixture.run.clone()
            },
        )
        .unwrap();

        let result = super::apply_capture_guide_edit(
            &fixture.conn,
            ApplyCaptureGuideEditInput {
                run_id: fixture.run.id.clone(),
                expected_revision: 4,
                expected_mesh_digest: fixture.mesh_digest.clone(),
                edit: CaptureGuideEditIntent::SelectFeaturePlan {
                    plan_id: Some("plan-1".into()),
                },
            },
        )
        .unwrap();

        assert_eq!(
            result.guide.selected_feature_plan_id.as_deref(),
            Some("plan-1")
        );
        assert_eq!(result.guide.feature_plan_candidates.len(), 1);
    }

    #[test]
    fn given_previous_draft_when_replaced_then_rust_preserves_identity_and_invalidates_evidence() {
        let fixture = fixture();
        let mut previous = fixture.run.reconstruction_guide.clone().unwrap();
        previous.revision = 1;
        previous.landmarks[0].label = "restored datum".into();

        let result = super::apply_capture_guide_edit(
            &fixture.conn,
            ApplyCaptureGuideEditInput {
                run_id: fixture.run.id.clone(),
                expected_revision: 4,
                expected_mesh_digest: fixture.mesh_digest.clone(),
                edit: CaptureGuideEditIntent::ReplaceDraft {
                    guide: Box::new(previous),
                },
            },
        )
        .unwrap();

        assert_eq!(result.guide.revision, 5);
        assert_eq!(result.guide.guide_id, "guide-1");
        assert_eq!(result.guide.landmarks[0].label, "restored datum");
        assert!(result.guide.feature_plan_candidates.is_empty());

        let mut foreign = result.guide.clone();
        foreign.guide_id = "foreign-guide".into();
        let error = super::apply_capture_guide_edit(
            &fixture.conn,
            ApplyCaptureGuideEditInput {
                run_id: fixture.run.id.clone(),
                expected_revision: 5,
                expected_mesh_digest: fixture.mesh_digest.clone(),
                edit: CaptureGuideEditIntent::ReplaceDraft {
                    guide: Box::new(foreign),
                },
            },
        )
        .unwrap_err();
        assert!(error.message.contains("identity differs"));
    }

    #[test]
    fn given_capture_run_without_guide_when_ensured_then_rust_creates_canonical_initial_head() {
        let fixture = fixture_without_guide();

        let result =
            super::ensure_capture_reconstruction_guide(&fixture.conn, &fixture.run.id).unwrap();

        assert!(result.created);
        assert_eq!(result.state, CaptureReconstructionGuideState::Draft);
        assert_eq!(result.guide.revision, 1);
        assert_eq!(result.guide.capture_run_id, fixture.run.id);
        assert_eq!(result.guide.target_thread_id, fixture.run.target_thread_id);
        assert_eq!(result.guide.target_message_id, None);
        assert_eq!(result.guide.target_version_id, None);
        assert_eq!(result.guide.source_mesh.content_digest, fixture.mesh_digest);
        assert_eq!(
            result.guide.target_source_digest,
            crate::services::render_snapshot::canonical_source_digest("(solid blank)")
        );
        assert!(result.guide.landmarks.is_empty());
        assert!(!result.guide.canonical_digest.is_empty());
        assert_eq!(
            crate::capture_runs::capture_guide_version_count(&fixture.conn, &fixture.run.id)
                .unwrap(),
            1
        );
        assert_eq!(
            crate::capture_runs::get(&fixture.conn, &fixture.run.id)
                .unwrap()
                .unwrap()
                .reconstruction_guide,
            Some(result.guide)
        );
    }

    #[test]
    fn given_existing_guide_when_ensured_then_rust_returns_same_head_without_append() {
        let fixture = fixture();

        let result =
            super::ensure_capture_reconstruction_guide(&fixture.conn, &fixture.run.id).unwrap();

        assert!(!result.created);
        assert_eq!(result.state, CaptureReconstructionGuideState::Ready);
        assert_eq!(
            result.guide,
            fixture.run.reconstruction_guide.clone().unwrap()
        );
        assert_eq!(
            crate::capture_runs::capture_guide_version_count(&fixture.conn, &fixture.run.id)
                .unwrap(),
            0
        );
    }

    #[test]
    fn given_capture_run_without_source_when_guide_is_ensured_then_raw_error_is_returned() {
        let fixture = fixture_without_guide();
        let mut run = fixture.run.clone();
        run.mesh_preview = None;
        crate::capture_runs::insert(&fixture.conn, &run).unwrap();

        let error =
            super::ensure_capture_reconstruction_guide(&fixture.conn, &fixture.run.id).unwrap_err();

        assert_eq!(error.message, "Capture run has no source mesh.");
        assert_eq!(
            crate::capture_runs::capture_guide_version_count(&fixture.conn, &fixture.run.id)
                .unwrap(),
            0
        );
    }

    #[test]
    fn given_role_complete_head_when_validation_intent_arrives_then_rust_finalizes_ready_and_appends(
    ) {
        let fixture = fixture();
        install_mechanical_guide(&fixture, false);

        let result = super::validate_capture_guide_intent(
            &fixture.conn,
            ValidateCaptureGuideIntentInput {
                run_id: fixture.run.id.clone(),
                expected_revision: 4,
                expected_mesh_digest: fixture.mesh_digest.clone(),
                known_distance_mm: 40.0,
                instruction: "  Build exact insert.  ".into(),
                feature_depth_mm: 18.0,
            },
        )
        .unwrap();

        assert_eq!(result.guide.revision, 5);
        assert_eq!(result.guide.instruction, "Build exact insert.");
        assert_eq!(
            result.state,
            CaptureReconstructionGuideState::Ready,
            "raw={:?} readiness={:?}",
            result.raw_evidence,
            result.guide.reconstruction_readiness
        );
        assert!(result.guide.reconstruction_readiness.ready);
        assert!(result.raw_evidence.is_empty());
        assert_eq!(
            crate::capture_runs::capture_guide_version_count(&fixture.conn, &fixture.run.id)
                .unwrap(),
            1
        );
    }

    #[test]
    fn given_ambiguous_feature_plans_when_validation_intent_arrives_then_draft_head_is_appended() {
        let fixture = fixture();
        install_mechanical_guide(&fixture, true);

        let result = super::validate_capture_guide_intent(
            &fixture.conn,
            ValidateCaptureGuideIntentInput {
                run_id: fixture.run.id.clone(),
                expected_revision: 4,
                expected_mesh_digest: fixture.mesh_digest.clone(),
                known_distance_mm: 40.0,
                instruction: "Build ambiguous insert.".into(),
                feature_depth_mm: 18.0,
            },
        )
        .unwrap();

        assert_eq!(result.state, CaptureReconstructionGuideState::Draft);
        assert!(!result.guide.reconstruction_readiness.ready);
        assert!(
            !result.guide.feature_plan_candidates.is_empty(),
            "raw={:?} readiness={:?}",
            result.raw_evidence,
            result.guide.reconstruction_readiness
        );
        assert!(result.raw_evidence.is_empty());
        assert_eq!(result.guide.revision, 5);
    }

    #[test]
    fn given_incomplete_mechanical_evidence_when_validation_intent_arrives_then_raw_error_head_is_appended(
    ) {
        let fixture = fixture();

        let result = super::validate_capture_guide_intent(
            &fixture.conn,
            ValidateCaptureGuideIntentInput {
                run_id: fixture.run.id.clone(),
                expected_revision: 4,
                expected_mesh_digest: fixture.mesh_digest.clone(),
                known_distance_mm: 40.0,
                instruction: "Build incomplete insert.".into(),
                feature_depth_mm: 18.0,
            },
        )
        .unwrap();

        assert_eq!(result.state, CaptureReconstructionGuideState::Draft);
        assert_eq!(result.guide.revision, 5);
        assert!(result
            .raw_evidence
            .iter()
            .any(|evidence| evidence.contains("Pick two calibration endpoints")));
        assert_eq!(
            crate::capture_runs::capture_guide_version_count(&fixture.conn, &fixture.run.id)
                .unwrap(),
            1
        );
    }

    #[test]
    fn given_source_drift_when_validation_intent_arrives_then_stale_head_is_appended() {
        let fixture = fixture();
        install_mechanical_guide(&fixture, false);
        let source_path = fixture
            .run
            .mesh_preview
            .as_ref()
            .map(|preview| std::path::PathBuf::from(&preview.stl_path))
            .unwrap();
        std::fs::write(
            source_path,
            "solid changed\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 2 0 0\nvertex 0 2 0\nendloop\nendfacet\nendsolid changed\n",
        )
        .unwrap();

        let result = super::validate_capture_guide_intent(
            &fixture.conn,
            ValidateCaptureGuideIntentInput {
                run_id: fixture.run.id.clone(),
                expected_revision: 4,
                expected_mesh_digest: fixture.mesh_digest.clone(),
                known_distance_mm: 40.0,
                instruction: "Build stale insert.".into(),
                feature_depth_mm: 18.0,
            },
        )
        .unwrap();

        assert!(matches!(
            result.state,
            CaptureReconstructionGuideState::Stale { .. }
        ));
        assert!(!result.source_digest_matched);
        assert_eq!(result.guide.revision, 5);
        assert!(result
            .raw_evidence
            .iter()
            .any(|evidence| evidence.contains("selected crop/source mesh digest changed")));
    }
}
