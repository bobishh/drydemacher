use std::collections::{BTreeMap, HashSet};

use crate::contracts::{
    AppError, AppResult, CaptureAuthoredSelector, CaptureCorrespondenceRelation,
    CaptureCorrespondenceStatus, CaptureEvidenceCorrespondence, CaptureExpectedGeometryKind,
    CaptureGuideResultProvenance, CaptureReconstructionGuide, CaptureRequiredBrepTopologyKind,
    CaptureSelectorCardinality, ModelManifest, RenderSnapshot, SelectionTarget,
    SelectionTargetKind, TaggedAnchorBinding,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ExactBrepTargetGeometry {
    Vertex {
        point: [f64; 3],
    },
    LineEdge {
        start: [f64; 3],
        end: [f64; 3],
    },
    CircleEdge {
        center: [f64; 3],
        normal: [f64; 3],
        x_direction: [f64; 3],
        radius: f64,
        first_parameter: f64,
        last_parameter: f64,
    },
    PlaneFace {
        origin: [f64; 3],
        normal: [f64; 3],
        boundary_edge_target_ids: Vec<Vec<String>>,
    },
    CylinderFace {
        axis_origin: [f64; 3],
        axis_direction: [f64; 3],
        radius: f64,
        boundary_edge_target_ids: Vec<Vec<String>>,
    },
}

pub fn validate_capture_direct_occt_snapshot(
    guide: &CaptureReconstructionGuide,
    snapshot: &RenderSnapshot,
    topology_path: &std::path::Path,
) -> AppResult<CaptureGuideResultProvenance> {
    crate::services::render_snapshot::validate_render_snapshot(snapshot)?;
    let source_path = snapshot
        .model_manifest
        .document
        .source_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| {
            topology_path
                .parent()
                .map(|parent| parent.join("source.ecky"))
        })
        .ok_or_else(|| {
            AppError::validation("Guided reconstruction preview has no source artifact path.")
        })?;
    let source = std::fs::read_to_string(&source_path).map_err(|error| {
        AppError::persistence(format!(
            "Guided reconstruction source '{}' could not be read: {error}",
            source_path.display()
        ))
    })?;
    validate_capture_guided_source_semantics(guide, &source)?;
    crate::contracts::validate_model_runtime_bundle(
        &snapshot.model_manifest,
        &snapshot.artifact_bundle,
    )?;
    let structural = crate::services::structural_verification::verify_structure(
        &snapshot.artifact_bundle,
        &snapshot.model_manifest,
    );
    if !structural.passed {
        return Err(AppError::validation(structural.summary));
    }
    let provenance = snapshot
        .artifact_bundle
        .geometry_provenance
        .as_ref()
        .or(snapshot.model_manifest.geometry_provenance.as_ref())
        .ok_or_else(|| {
            AppError::validation("Guided reconstruction preview has no geometry provenance.")
        })?;
    if provenance.representation != crate::contracts::GeometryRepresentation::AnalyticBrep
        || !provenance.source_mesh_digests.is_empty()
    {
        return Err(AppError::validation(
            "Guided reconstruction preview must be analytic BRep and cannot originate from mesh solidification.",
        ));
    }
    crate::ecky_cad_host::direct_occt_runtime::validate_direct_occt_guided_expected_solids(
        topology_path,
    )?;
    let part_digests =
        crate::ecky_cad_host::direct_occt_runtime::direct_occt_part_source_geometry_digests(
            topology_path,
        )?;
    let exact_target_geometries =
        crate::ecky_cad_host::direct_occt_runtime::direct_occt_exact_target_geometries(
            topology_path,
        )?;
    let authored_binding_target_ids =
        crate::ecky_cad_host::direct_occt_runtime::direct_occt_authored_binding_target_ids(
            topology_path,
        )?;
    let authored_binding_ordered_edge_target_ids = crate::ecky_cad_host::direct_occt_runtime::direct_occt_authored_binding_ordered_edge_target_ids(
        topology_path,
    )?;
    validate_capture_brep_correspondences_with_bindings(
        guide,
        &snapshot.model_manifest,
        &part_digests,
        &exact_target_geometries,
        &authored_binding_target_ids,
        &authored_binding_ordered_edge_target_ids,
        &snapshot.source_digest,
        &snapshot.artifact_digest,
    )
}

pub fn validate_capture_guided_source_semantics(
    guide: &CaptureReconstructionGuide,
    source: &str,
) -> AppResult<()> {
    use crate::ecky_core_ir::{
        CoreKeywordValue, CoreNode, CoreNodeKind, CoreOperation, CoreSelectorTagKind,
        CoreTransformOp,
    };

    #[derive(Default)]
    struct SourceEvidence {
        mirror_count: usize,
        mirror_planes: HashSet<String>,
        binding_names: HashSet<String>,
    }
    fn visit(node: &CoreNode, evidence: &mut SourceEvidence) {
        match &node.kind {
            CoreNodeKind::Literal(_) | CoreNodeKind::Reference(_) => {}
            CoreNodeKind::Build { bindings, result } => {
                for binding in bindings {
                    evidence.binding_names.insert(binding.name.clone());
                    visit(&binding.value, evidence);
                }
                visit(result, evidence);
            }
            CoreNodeKind::Let { bindings, body } => {
                for binding in bindings {
                    evidence.binding_names.insert(binding.name.clone());
                    visit(&binding.value, evidence);
                }
                visit(body, evidence);
            }
            CoreNodeKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                visit(condition, evidence);
                visit(then_branch, evidence);
                visit(else_branch, evidence);
            }
            CoreNodeKind::Call { op, args, keywords } => {
                if *op == CoreOperation::Transform(CoreTransformOp::Mirror) {
                    evidence.mirror_count += 1;
                    let axis = args.first().and_then(|arg| match &arg.kind {
                        CoreNodeKind::Literal(crate::ecky_core_ir::CoreLiteral::Text(value)) => {
                            Some(value.as_str())
                        }
                        _ => None,
                    });
                    let offset = args.get(1).and_then(|arg| match &arg.kind {
                        CoreNodeKind::Literal(crate::ecky_core_ir::CoreLiteral::Number(value)) => {
                            Some(*value)
                        }
                        _ => None,
                    });
                    if let (Some(axis), Some(offset)) = (axis, offset) {
                        evidence.mirror_planes.insert(format!(
                            "{}:{:016x}",
                            axis.to_ascii_lowercase(),
                            offset.to_bits()
                        ));
                    }
                }
                for arg in args {
                    visit(arg, evidence);
                }
                for keyword in keywords {
                    match &keyword.value {
                        CoreKeywordValue::Expr(value) => visit(value, evidence),
                        CoreKeywordValue::Selector { source, .. } => visit(source, evidence),
                    }
                }
            }
            CoreNodeKind::Range { start, end } => {
                visit(start, evidence);
                visit(end, evidence);
            }
            CoreNodeKind::Map { sources, body, .. } => {
                for source in sources {
                    visit(source, evidence);
                }
                visit(body, evidence);
            }
            CoreNodeKind::Apply { args, list, .. } => {
                for arg in args {
                    visit(arg, evidence);
                }
                visit(list, evidence);
            }
            CoreNodeKind::List(items) | CoreNodeKind::Group(items) => {
                for item in items {
                    visit(item, evidence);
                }
            }
        }
    }

    let program = crate::ecky_scheme::compile_to_core_program(source).map_err(|error| {
        AppError::validation(format!(
            "Guided reconstruction source does not compile: {}",
            error.message
        ))
    })?;
    let mut evidence = SourceEvidence::default();
    for part in &program.parts {
        visit(&part.root, &mut evidence);
    }
    let parameter_names = program
        .parameters
        .iter()
        .map(|parameter| parameter.key.as_str())
        .collect::<HashSet<_>>();
    for measurement in guide.measurements.iter().filter(|item| item.fit_critical) {
        let name = measurement
            .authored_parameter_name
            .as_deref()
            .unwrap_or_default();
        if !parameter_names.contains(name) && !evidence.binding_names.contains(name) {
            return Err(AppError::validation(format!(
                "Fit-critical guide measurement '{}' requires missing authored parameter/binding '{}'.",
                measurement.measurement_id, name
            )));
        }
    }
    let required_mirrors = match guide.symmetry_completion {
        crate::contracts::CaptureSymmetryCompletion::None => 0,
        crate::contracts::CaptureSymmetryCompletion::Half { .. } => 1,
        crate::contracts::CaptureSymmetryCompletion::Quarter { .. } => 2,
    };
    if evidence.mirror_count < required_mirrors {
        return Err(AppError::validation(format!(
            "Guided reconstruction requires {required_mirrors} explicit mirror operation(s), source contains {}.",
            evidence.mirror_count
        )));
    }
    if required_mirrors == 2 && evidence.mirror_planes.len() < 2 {
        return Err(AppError::validation(
            "Quarter guided reconstruction requires two distinct explicit mirror planes.",
        ));
    }
    for expectation in guide
        .feature_expectations
        .iter()
        .filter(|expectation| expectation.required_for_acceptance)
    {
        match &expectation.expected_authored_selector {
            CaptureAuthoredSelector::Binding { name } => {
                if !evidence.binding_names.contains(name) {
                    return Err(AppError::validation(format!(
                        "Required capture expectation '{}' references missing authored binding '{}'.",
                        expectation.expectation_id, name
                    )));
                }
            }
            CaptureAuthoredSelector::Tag { name } => {
                let expected_kind = match expectation.required_brep_topology_kind {
                    CaptureRequiredBrepTopologyKind::Vertex => CoreSelectorTagKind::Vertex,
                    CaptureRequiredBrepTopologyKind::Edge
                    | CaptureRequiredBrepTopologyKind::OrderedEdges => CoreSelectorTagKind::Edge,
                    CaptureRequiredBrepTopologyKind::Face => CoreSelectorTagKind::Face,
                };
                if !program
                    .selector_tags
                    .iter()
                    .any(|tag| tag.name == *name && tag.kind == expected_kind)
                {
                    return Err(AppError::validation(format!(
                        "Required capture expectation '{}' references missing or wrong-kind authored tag '{}'.",
                        expectation.expectation_id, name
                    )));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn validate_capture_brep_correspondences(
    guide: &CaptureReconstructionGuide,
    manifest: &ModelManifest,
    part_source_geometry_digests: &BTreeMap<String, String>,
    exact_target_geometries: &BTreeMap<String, ExactBrepTargetGeometry>,
    generated_source_digest: &str,
    geometry_digest: &str,
) -> AppResult<CaptureGuideResultProvenance> {
    validate_capture_brep_correspondences_with_bindings(
        guide,
        manifest,
        part_source_geometry_digests,
        exact_target_geometries,
        &BTreeMap::new(),
        &BTreeMap::new(),
        generated_source_digest,
        geometry_digest,
    )
}

pub(crate) fn validate_capture_brep_correspondences_with_bindings(
    guide: &CaptureReconstructionGuide,
    manifest: &ModelManifest,
    part_source_geometry_digests: &BTreeMap<String, String>,
    exact_target_geometries: &BTreeMap<String, ExactBrepTargetGeometry>,
    authored_binding_target_ids: &BTreeMap<(String, String), Vec<String>>,
    authored_binding_ordered_edge_target_ids: &BTreeMap<(String, String), Vec<String>>,
    generated_source_digest: &str,
    geometry_digest: &str,
) -> AppResult<CaptureGuideResultProvenance> {
    guide.validate().map_err(AppError::validation)?;
    if manifest.source_digest.as_deref() != Some(generated_source_digest) {
        return Err(AppError::conflict(
            "Generated source digest differs from exact preview manifest.",
        ));
    }
    require_digest("generated source", generated_source_digest)?;
    require_digest("generated geometry", geometry_digest)?;

    let mut correspondences = Vec::with_capacity(guide.feature_expectations.len());
    for expectation in &guide.feature_expectations {
        let result = match &expectation.expected_authored_selector {
            CaptureAuthoredSelector::Tag { name } => manifest
                .tagged_anchors
                .get(name)
                .ok_or_else(|| {
                    AppError::validation(format!(
                        "Required authored tag '{}' for capture expectation '{}' is missing.",
                        name, expectation.expectation_id
                    ))
                })
                .and_then(|binding| {
                    resolve_tagged_expectation(
                        expectation,
                        binding,
                        &manifest.selection_targets,
                        part_source_geometry_digests,
                    )
                })
                .and_then(|mut correspondence| {
                    correspondence.residual = Some(validate_exact_target_residual(
                        guide,
                        expectation,
                        &correspondence.canonical_target_ids,
                        exact_target_geometries,
                    )?);
                    Ok(correspondence)
                }),
            CaptureAuthoredSelector::Binding { name } => {
                let binding_key = (expectation.part_id.clone(), name.clone());
                let target_ids = if expectation.required_brep_topology_kind
                    == CaptureRequiredBrepTopologyKind::OrderedEdges
                {
                    authored_binding_ordered_edge_target_ids.get(&binding_key)
                } else {
                    authored_binding_target_ids.get(&binding_key)
                };
                target_ids
                .ok_or_else(|| {
                    let mapping_kind = if expectation.required_brep_topology_kind
                        == CaptureRequiredBrepTopologyKind::OrderedEdges
                    {
                        "exact ordered edge"
                    } else {
                        "exact topology"
                    };
                    AppError::validation(format!(
                        "Authored binding '{}' for capture expectation '{}' has no {mapping_kind} mapping in preview manifest.",
                        name, expectation.expectation_id,
                    ))
                })
                .and_then(|target_ids| {
                    resolve_authored_expectation(
                        expectation,
                        &expectation.part_id,
                        target_ids,
                        &format!("Authored binding '{name}'"),
                        true,
                        &manifest.selection_targets,
                        part_source_geometry_digests,
                    )
                })
                .and_then(|mut correspondence| {
                    correspondence.residual = Some(validate_exact_target_residual(
                        guide,
                        expectation,
                        &correspondence.canonical_target_ids,
                        exact_target_geometries,
                    )?);
                    Ok(correspondence)
                })
            }
        };
        match result {
            Ok(correspondence) => correspondences.push(correspondence),
            Err(error) if expectation.required_for_acceptance => return Err(error),
            Err(_) => correspondences.push(CaptureEvidenceCorrespondence {
                expectation_id: expectation.expectation_id.clone(),
                guide_item_ids: expectation.guide_item_ids.clone(),
                part_id: expectation.part_id.clone(),
                instance_path: expectation.instance_path.clone(),
                authored_selector: expectation.expected_authored_selector.clone(),
                selector_cardinality: expectation.cardinality.clone(),
                brep_target_kind: expectation.required_brep_topology_kind.clone(),
                canonical_target_ids: Vec::new(),
                durable_target_ids: Vec::new(),
                source_stable_node_keys: Vec::new(),
                source_geometry_digest: String::new(),
                relation: expectation_relation(expectation.expected_geometry_kind.clone()),
                residual: None,
                status: CaptureCorrespondenceStatus::Missing,
            }),
        }
    }

    let feature_operation_traces = feature_operation_traces(guide, &correspondences)?;
    Ok(CaptureGuideResultProvenance {
        guide_id: guide.guide_id.clone(),
        guide_revision: guide.revision,
        guide_canonical_digest: guide.canonical_digest.clone(),
        source_mesh_artifact_digest: guide.source_mesh.artifact_digest.clone(),
        source_mesh_content_digest: guide.source_mesh.content_digest.clone(),
        target_source_digest: guide.target_source_digest.clone(),
        target_version_id: guide.target_version_id.clone(),
        generated_source_digest: generated_source_digest.to_string(),
        geometry_digest: geometry_digest.to_string(),
        assumptions: Vec::new(),
        inferred_regions: Vec::new(),
        selected_feature_plan_id: guide.selected_feature_plan_id.clone(),
        feature_operation_traces,
        correspondences,
    })
}

fn feature_operation_traces(
    guide: &CaptureReconstructionGuide,
    correspondences: &[crate::contracts::CaptureEvidenceCorrespondence],
) -> AppResult<Vec<crate::contracts::CaptureFeatureOperationTrace>> {
    let Some(selected_id) = guide.selected_feature_plan_id.as_deref() else {
        return Ok(Vec::new());
    };
    let plan = guide
        .feature_plan_candidates
        .iter()
        .find(|plan| plan.plan_id == selected_id)
        .ok_or_else(|| AppError::validation("Capture selected feature plan is missing."))?;
    let profile_source = |candidate_id: &str| {
        guide
            .reconstructed_profiles
            .iter()
            .find(|profile| profile.candidate_id == candidate_id)
            .map(|profile| profile.source_profile_id.clone())
    };
    plan.operations
        .iter()
        .enumerate()
        .map(|(index, operation)| {
            let (kind, evidence_ids) = match operation {
                crate::contracts::CaptureFeatureOperation::Extrude {
                    profile_candidate_id,
                    distance_dimension_id,
                } => (
                    "extrude",
                    profile_source(profile_candidate_id)
                        .into_iter()
                        .chain(std::iter::once(distance_dimension_id.clone()))
                        .collect::<Vec<_>>(),
                ),
                crate::contracts::CaptureFeatureOperation::Revolve {
                    profile_candidate_id,
                    axis_id,
                    ..
                } => (
                    "revolve",
                    profile_source(profile_candidate_id)
                        .into_iter()
                        .chain(std::iter::once(axis_id.clone()))
                        .collect(),
                ),
                crate::contracts::CaptureFeatureOperation::Sweep {
                    profile_candidate_id,
                    path_id,
                } => (
                    "sweep",
                    profile_source(profile_candidate_id)
                        .into_iter()
                        .chain(std::iter::once(path_id.clone()))
                        .collect(),
                ),
                crate::contracts::CaptureFeatureOperation::Mirror { plane_id } => {
                    ("mirror", vec![plane_id.clone()])
                }
                crate::contracts::CaptureFeatureOperation::BooleanUnion { operand_plan_ids } => {
                    (
                        "booleanUnion",
                        operand_plan_ids
                            .iter()
                            .filter_map(|plan_id| {
                                guide
                                    .feature_plan_candidates
                                    .iter()
                                    .find(|plan| plan.plan_id == *plan_id)
                            })
                            .flat_map(|plan| plan.supporting_evidence_ids.iter().cloned())
                            .collect(),
                    )
                }
                crate::contracts::CaptureFeatureOperation::BooleanDifference {
                    base_plan_id,
                    cutter_plan_ids,
                } => {
                    let plan_ids = std::iter::once(base_plan_id)
                        .chain(cutter_plan_ids.iter())
                        .collect::<Vec<_>>();
                    (
                        "booleanDifference",
                        plan_ids
                            .into_iter()
                            .filter_map(|plan_id| {
                                guide
                                    .feature_plan_candidates
                                    .iter()
                                    .find(|plan| plan.plan_id == *plan_id)
                            })
                            .flat_map(|plan| plan.supporting_evidence_ids.iter().cloned())
                            .collect(),
                    )
                }
            };
            let matching = correspondences
                .iter()
                .filter(|correspondence| {
                    correspondence.status
                        == crate::contracts::CaptureCorrespondenceStatus::Satisfied
                        && correspondence
                            .guide_item_ids
                            .iter()
                            .any(|id| evidence_ids.contains(id))
                })
                .collect::<Vec<_>>();
            if matching.is_empty() {
                return Err(AppError::validation(format!(
                    "Selected feature plan operation '{kind}' has no exact BRep correspondence."
                )));
            }
            let mut node_keys = matching
                .iter()
                .flat_map(|item| item.source_stable_node_keys.iter().cloned())
                .collect::<Vec<_>>();
            let mut binding_names = matching
                .iter()
                .map(|item| match &item.authored_selector {
                    crate::contracts::CaptureAuthoredSelector::Binding { name }
                    | crate::contracts::CaptureAuthoredSelector::Tag { name } => name.clone(),
                })
                .collect::<Vec<_>>();
            let mut target_ids = matching
                .iter()
                .flat_map(|item| item.durable_target_ids.iter().cloned())
                .collect::<Vec<_>>();
            for values in [&mut node_keys, &mut binding_names, &mut target_ids] {
                values.sort();
                values.dedup();
            }
            if node_keys.is_empty() || target_ids.is_empty() {
                return Err(AppError::validation(format!(
                    "Selected feature plan operation '{kind}' lacks authored-node or durable-target trace."
                )));
            }
            Ok(crate::contracts::CaptureFeatureOperationTrace {
                operation_index: index as u64,
                operation_kind: kind.into(),
                evidence_ids,
                authored_node_keys: node_keys,
                authored_binding_names: binding_names,
                brep_target_ids: target_ids,
            })
        })
        .collect()
}

fn validate_exact_target_residual(
    guide: &CaptureReconstructionGuide,
    expectation: &crate::contracts::CaptureFeatureExpectation,
    target_ids: &[String],
    geometries: &BTreeMap<String, ExactBrepTargetGeometry>,
) -> AppResult<crate::contracts::CaptureCorrespondenceResidual> {
    let targets = target_ids
        .iter()
        .map(|target_id| {
            geometries.get(target_id).ok_or_else(|| {
                AppError::validation(format!(
                    "Capture expectation '{}' target '{}' has no exact target-kind metric geometry.",
                    expectation.expectation_id, target_id
                ))
            })
        })
        .collect::<AppResult<Vec<_>>>()?;
    if targets.iter().any(|target| {
        !exact_geometry_matches_expectation(target, &expectation.expected_geometry_kind)
    }) {
        return Err(AppError::validation(format!(
            "Capture expectation '{}' exact target analytic geometry kind is incompatible.",
            expectation.expectation_id
        )));
    }
    let mut observation_points = Vec::new();
    let mut observation_normals = Vec::new();
    let mut observation_axes = Vec::new();
    let mut observation_profiles = Vec::new();
    for guide_item_id in &expectation.guide_item_ids {
        if let Some(landmark) = guide
            .landmarks
            .iter()
            .find(|landmark| landmark.landmark_id == *guide_item_id)
        {
            observation_points.push(landmark.local_position_mm);
            observation_normals.push(landmark.local_normal);
            continue;
        }
        if let Some(axis) = guide
            .axes
            .iter()
            .find(|axis| axis.axis_id == *guide_item_id)
        {
            observation_axes.push((axis.origin_mm, axis.direction));
            continue;
        }
        if let Some(plane) = guide
            .planes
            .iter()
            .find(|plane| plane.plane_id == *guide_item_id)
        {
            observation_points.push(plane.origin_mm);
            observation_normals.push(plane.normal);
            continue;
        }
        if let Some(profile) = guide
            .profiles
            .iter()
            .find(|profile| profile.profile_id == *guide_item_id)
        {
            observation_profiles.push(profile);
        }
    }
    if expectation.expected_geometry_kind == CaptureExpectedGeometryKind::Profile {
        if observation_profiles.len() != 1 {
            return Err(AppError::validation(format!(
                "Capture expectation '{}' requires exactly one ordered profile evidence item.",
                expectation.expectation_id
            )));
        }
        return validate_exact_profile_residual(
            guide,
            expectation,
            observation_profiles[0],
            &targets,
        );
    }
    if expectation.expected_geometry_kind == CaptureExpectedGeometryKind::Cylinder
        && !observation_axes.is_empty()
    {
        return validate_exact_axis_residual(
            expectation,
            &observation_axes,
            &observation_points,
            &targets,
            geometries,
        );
    }
    if observation_points.is_empty() {
        return Err(AppError::validation(format!(
            "Capture expectation '{}' has no metric observation points.",
            expectation.expectation_id
        )));
    }

    let residuals = observation_points
        .iter()
        .map(|point| {
            targets
                .iter()
                .filter_map(|geometry| exact_target_point_residual(geometry, *point, geometries))
                .fold(f64::INFINITY, f64::min)
        })
        .collect::<Vec<_>>();
    if residuals.iter().any(|residual| !residual.is_finite()) {
        return Err(AppError::validation(format!(
            "Capture expectation '{}' exact target kind cannot evaluate supplied guide evidence.",
            expectation.expectation_id
        )));
    }
    let maximum = residuals.iter().copied().fold(0.0, f64::max);
    let rms =
        (residuals.iter().map(|value| value * value).sum::<f64>() / residuals.len() as f64).sqrt();
    if expectation
        .position_tolerance_mm
        .is_some_and(|tolerance| maximum > tolerance)
    {
        return Err(AppError::validation(format!(
            "Capture expectation '{}' position residual {:.6} mm exceeds tolerance {:.6} mm.",
            expectation.expectation_id,
            maximum,
            expectation.position_tolerance_mm.unwrap_or_default()
        )));
    }

    let mut components = Vec::new();
    if let Some(tolerance_deg) = expectation.normal_tolerance_deg {
        let target_normals = targets
            .iter()
            .filter_map(|geometry| match geometry {
                ExactBrepTargetGeometry::PlaneFace { normal, .. } => Some(*normal),
                _ => None,
            })
            .collect::<Vec<_>>();
        if !observation_normals.is_empty() && !target_normals.is_empty() {
            let angle_residuals = observation_normals
                .iter()
                .map(|observed| {
                    target_normals
                        .iter()
                        .map(|target| normalized_dot_abs(*observed, *target).acos().to_degrees())
                        .fold(f64::INFINITY, f64::min)
                })
                .collect::<Vec<_>>();
            let maximum_angle = angle_residuals.iter().copied().fold(0.0, f64::max);
            let rms_angle = (angle_residuals
                .iter()
                .map(|value| value * value)
                .sum::<f64>()
                / angle_residuals.len() as f64)
                .sqrt();
            if maximum_angle > tolerance_deg {
                return Err(AppError::validation(format!(
                    "Capture expectation '{}' normal residual {:.6} deg exceeds tolerance {:.6} deg.",
                    expectation.expectation_id, maximum_angle, tolerance_deg
                )));
            }
            components.push(crate::contracts::CaptureCorrespondenceResidualComponent {
                metric: "normalAngle".into(),
                maximum: maximum_angle,
                rms: rms_angle,
                unit: "deg".into(),
            });
        }
    }

    Ok(crate::contracts::CaptureCorrespondenceResidual {
        metric: match expectation.required_brep_topology_kind {
            CaptureRequiredBrepTopologyKind::Vertex => "pointToVertex",
            CaptureRequiredBrepTopologyKind::Edge => "pointToExactEdge",
            CaptureRequiredBrepTopologyKind::Face => "pointToSupportingSurface",
            CaptureRequiredBrepTopologyKind::OrderedEdges => "profilePointToExactEdges",
        }
        .into(),
        maximum,
        rms,
        unit: "mm".into(),
        components,
    })
}

fn validate_exact_profile_residual(
    guide: &CaptureReconstructionGuide,
    expectation: &crate::contracts::CaptureFeatureExpectation,
    profile: &crate::contracts::CaptureOrderedProfile,
    targets: &[&ExactBrepTargetGeometry],
) -> AppResult<crate::contracts::CaptureCorrespondenceResidual> {
    let points = profile
        .landmark_ids
        .iter()
        .map(|landmark_id| {
            guide
                .landmarks
                .iter()
                .find(|landmark| landmark.landmark_id == *landmark_id)
                .map(|landmark| landmark.local_position_mm)
                .ok_or_else(|| {
                    AppError::validation(format!(
                        "Capture ordered profile '{}' references missing landmark '{}'.",
                        profile.profile_id, landmark_id
                    ))
                })
        })
        .collect::<AppResult<Vec<_>>>()?;
    let segment_count = match profile.kind {
        crate::contracts::CaptureProfileKind::Open => points.len().saturating_sub(1),
        crate::contracts::CaptureProfileKind::Closed => points.len(),
    };
    let mut residuals = Vec::new();

    if targets.len() == segment_count {
        for (index, target) in targets.iter().enumerate() {
            let observed_start = points[index];
            let observed_end = points[(index + 1) % points.len()];
            let (target_start, target_end) = exact_edge_endpoints(target).ok_or_else(|| {
                AppError::validation(format!(
                    "Capture expectation '{}' ordered edge {} has no exact endpoint geometry.",
                    expectation.expectation_id,
                    index + 1
                ))
            })?;
            let forward = [
                norm(sub(observed_start, target_start)),
                norm(sub(observed_end, target_end)),
            ];
            let reverse = [
                norm(sub(observed_start, target_end)),
                norm(sub(observed_end, target_start)),
            ];
            residuals.extend(
                if forward.iter().sum::<f64>() <= reverse.iter().sum::<f64>() {
                    forward
                } else {
                    reverse
                },
            );
        }
    } else if targets.len() == 1 {
        let target = targets[0];
        let mut parameters = Vec::with_capacity(points.len());
        for point in &points {
            let (parameter, residual) = exact_edge_parameter(target, *point).ok_or_else(|| {
                AppError::validation(format!(
                    "Capture expectation '{}' ordered profile cannot be parameterized on its exact edge.",
                    expectation.expectation_id
                ))
            })?;
            parameters.push(parameter);
            residuals.push(residual);
        }
        validate_single_edge_profile_order(expectation, profile, target, &points, &parameters)?;
    } else {
        return Err(AppError::validation(format!(
            "Capture expectation '{}' ordered profile has {} segments but resolves {} exact edges.",
            expectation.expectation_id,
            segment_count,
            targets.len()
        )));
    }

    if residuals.is_empty() || residuals.iter().any(|residual| !residual.is_finite()) {
        return Err(AppError::validation(format!(
            "Capture expectation '{}' ordered profile exact metric is degenerate.",
            expectation.expectation_id
        )));
    }
    let maximum = residuals.iter().copied().fold(0.0, f64::max);
    let rms =
        (residuals.iter().map(|value| value * value).sum::<f64>() / residuals.len() as f64).sqrt();
    if expectation
        .position_tolerance_mm
        .is_some_and(|tolerance| maximum > tolerance)
    {
        return Err(AppError::validation(format!(
            "Capture expectation '{}' ordered profile residual {:.6} mm exceeds tolerance {:.6} mm.",
            expectation.expectation_id,
            maximum,
            expectation.position_tolerance_mm.unwrap_or_default()
        )));
    }

    Ok(crate::contracts::CaptureCorrespondenceResidual {
        metric: "orderedProfileToExactEdges".into(),
        maximum,
        rms,
        unit: "mm".into(),
        components: vec![crate::contracts::CaptureCorrespondenceResidualComponent {
            metric: "orderedProfileEndpointOrCurveDistance".into(),
            maximum,
            rms,
            unit: "mm".into(),
        }],
    })
}

fn validate_single_edge_profile_order(
    expectation: &crate::contracts::CaptureFeatureExpectation,
    profile: &crate::contracts::CaptureOrderedProfile,
    target: &ExactBrepTargetGeometry,
    points: &[[f64; 3]],
    parameters: &[f64],
) -> AppResult<()> {
    const ORDER_EPSILON: f64 = 1.0e-9;
    match profile.kind {
        crate::contracts::CaptureProfileKind::Open => {
            let nondecreasing = parameters
                .windows(2)
                .all(|pair| pair[1] + ORDER_EPSILON >= pair[0]);
            let nonincreasing = parameters
                .windows(2)
                .all(|pair| pair[1] <= pair[0] + ORDER_EPSILON);
            if !nondecreasing && !nonincreasing {
                return Err(AppError::validation(format!(
                    "Capture expectation '{}' ordered profile reverses direction along its exact edge.",
                    expectation.expectation_id
                )));
            }
            let (target_start, target_end) = exact_edge_endpoints(target).ok_or_else(|| {
                AppError::validation(format!(
                    "Capture expectation '{}' ordered profile edge has no exact endpoints.",
                    expectation.expectation_id
                ))
            })?;
            let forward = norm(sub(points[0], target_start))
                .max(norm(sub(*points.last().unwrap_or(&points[0]), target_end)));
            let reverse = norm(sub(points[0], target_end)).max(norm(sub(
                *points.last().unwrap_or(&points[0]),
                target_start,
            )));
            if expectation
                .position_tolerance_mm
                .is_some_and(|tolerance| forward.min(reverse) > tolerance)
            {
                return Err(AppError::validation(format!(
                    "Capture expectation '{}' ordered profile endpoints do not span its exact edge.",
                    expectation.expectation_id
                )));
            }
        }
        crate::contracts::CaptureProfileKind::Closed => {
            if !matches!(target, ExactBrepTargetGeometry::CircleEdge { first_parameter, last_parameter, .. }
                if last_parameter - first_parameter >= std::f64::consts::TAU - 1.0e-10)
            {
                return Err(AppError::validation(format!(
                    "Capture expectation '{}' closed ordered profile cannot map to one non-closed exact edge.",
                    expectation.expectation_id
                )));
            }
            let forward_turns = parameters
                .iter()
                .zip(parameters.iter().cycle().skip(1))
                .take(parameters.len())
                .map(|(first, second)| (second - first).rem_euclid(1.0))
                .sum::<f64>();
            let reverse_turns = parameters
                .iter()
                .zip(parameters.iter().cycle().skip(1))
                .take(parameters.len())
                .map(|(first, second)| (first - second).rem_euclid(1.0))
                .sum::<f64>();
            if forward_turns > 1.0 + ORDER_EPSILON && reverse_turns > 1.0 + ORDER_EPSILON {
                return Err(AppError::validation(format!(
                    "Capture expectation '{}' closed ordered profile crosses its exact edge out of order.",
                    expectation.expectation_id
                )));
            }
        }
    }
    Ok(())
}

fn exact_edge_endpoints(geometry: &ExactBrepTargetGeometry) -> Option<([f64; 3], [f64; 3])> {
    match geometry {
        ExactBrepTargetGeometry::LineEdge { start, end } => Some((*start, *end)),
        ExactBrepTargetGeometry::CircleEdge {
            center,
            normal,
            x_direction,
            radius,
            first_parameter,
            last_parameter,
        } => Some((
            exact_circle_edge_point(*center, *normal, *x_direction, *radius, *first_parameter)?,
            exact_circle_edge_point(*center, *normal, *x_direction, *radius, *last_parameter)?,
        )),
        _ => None,
    }
}

fn exact_edge_parameter(geometry: &ExactBrepTargetGeometry, point: [f64; 3]) -> Option<(f64, f64)> {
    match geometry {
        ExactBrepTargetGeometry::LineEdge { start, end } => {
            let segment = sub(*end, *start);
            let length_squared = dot(segment, segment);
            if length_squared <= 1.0e-18 {
                return None;
            }
            let parameter = (dot(sub(point, *start), segment) / length_squared).clamp(0.0, 1.0);
            Some((
                parameter,
                norm(sub(point, add(*start, scale(segment, parameter)))),
            ))
        }
        ExactBrepTargetGeometry::CircleEdge {
            center,
            normal,
            x_direction,
            radius,
            first_parameter,
            last_parameter,
        } => {
            let normal = normalized(*normal)?;
            let x_direction =
                normalized(sub(*x_direction, scale(normal, dot(*x_direction, normal))))?;
            let y_direction = cross(normal, x_direction);
            let offset = sub(point, *center);
            let angle = dot(offset, y_direction).atan2(dot(offset, x_direction));
            let span = *last_parameter - *first_parameter;
            if !radius.is_finite() || *radius <= 0.0 || !span.is_finite() || span <= 0.0 {
                return None;
            }
            let parameter = if span >= std::f64::consts::TAU - 1.0e-10 {
                (angle - *first_parameter).rem_euclid(std::f64::consts::TAU) / span
            } else {
                let unwrapped = angle
                    + ((*first_parameter - angle) / std::f64::consts::TAU).ceil()
                        * std::f64::consts::TAU;
                ((unwrapped - *first_parameter) / span).clamp(0.0, 1.0)
            };
            Some((parameter, exact_point_residual(geometry, point)?))
        }
        _ => None,
    }
}

fn exact_circle_edge_point(
    center: [f64; 3],
    normal: [f64; 3],
    x_direction: [f64; 3],
    radius: f64,
    parameter: f64,
) -> Option<[f64; 3]> {
    if !radius.is_finite() || radius <= 0.0 || !parameter.is_finite() {
        return None;
    }
    let normal = normalized(normal)?;
    let x_direction = normalized(sub(x_direction, scale(normal, dot(x_direction, normal))))?;
    let y_direction = cross(normal, x_direction);
    Some(add(
        center,
        add(
            scale(x_direction, radius * parameter.cos()),
            scale(y_direction, radius * parameter.sin()),
        ),
    ))
}

fn validate_exact_axis_residual(
    expectation: &crate::contracts::CaptureFeatureExpectation,
    observation_axes: &[([f64; 3], [f64; 3])],
    observation_points: &[[f64; 3]],
    targets: &[&ExactBrepTargetGeometry],
    geometries: &BTreeMap<String, ExactBrepTargetGeometry>,
) -> AppResult<crate::contracts::CaptureCorrespondenceResidual> {
    let target_axes = targets
        .iter()
        .filter_map(|target| match target {
            ExactBrepTargetGeometry::CylinderFace {
                axis_origin,
                axis_direction,
                ..
            } => Some((*axis_origin, *axis_direction)),
            ExactBrepTargetGeometry::CircleEdge { center, normal, .. } => Some((*center, *normal)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if target_axes.is_empty() {
        return Err(AppError::validation(format!(
            "Capture expectation '{}' has no exact cylinder/circle axis geometry.",
            expectation.expectation_id
        )));
    }

    let offsets = observation_axes
        .iter()
        .map(|(origin, _)| {
            target_axes
                .iter()
                .filter_map(|(target_origin, target_direction)| {
                    let direction = normalized(*target_direction)?;
                    let offset = sub(*origin, *target_origin);
                    Some(norm(sub(offset, scale(direction, dot(offset, direction)))))
                })
                .fold(f64::INFINITY, f64::min)
        })
        .collect::<Vec<_>>();
    let angles = observation_axes
        .iter()
        .map(|(_, direction)| {
            target_axes
                .iter()
                .map(|(_, target_direction)| {
                    normalized_dot_abs(*direction, *target_direction)
                        .acos()
                        .to_degrees()
                })
                .fold(f64::INFINITY, f64::min)
        })
        .collect::<Vec<_>>();
    if offsets
        .iter()
        .chain(&angles)
        .any(|value| !value.is_finite())
    {
        return Err(AppError::validation(format!(
            "Capture expectation '{}' exact axis metric is degenerate.",
            expectation.expectation_id
        )));
    }
    let maximum = offsets.iter().copied().fold(0.0, f64::max);
    let rms =
        (offsets.iter().map(|value| value * value).sum::<f64>() / offsets.len() as f64).sqrt();
    let offset_tolerance = expectation.position_tolerance_mm;
    if offset_tolerance.is_some_and(|tolerance| maximum > tolerance) {
        return Err(AppError::validation(format!(
            "Capture expectation '{}' axis offset residual {:.6} mm exceeds tolerance {:.6} mm.",
            expectation.expectation_id,
            maximum,
            offset_tolerance.unwrap_or_default()
        )));
    }
    let maximum_angle = angles.iter().copied().fold(0.0, f64::max);
    let rms_angle =
        (angles.iter().map(|value| value * value).sum::<f64>() / angles.len() as f64).sqrt();
    if expectation
        .normal_tolerance_deg
        .is_some_and(|tolerance| maximum_angle > tolerance)
    {
        return Err(AppError::validation(format!(
            "Capture expectation '{}' axis angle residual {:.6} deg exceeds tolerance {:.6} deg.",
            expectation.expectation_id,
            maximum_angle,
            expectation.normal_tolerance_deg.unwrap_or_default()
        )));
    }

    let mut components = vec![crate::contracts::CaptureCorrespondenceResidualComponent {
        metric: "axisAngle".into(),
        maximum: maximum_angle,
        rms: rms_angle,
        unit: "deg".into(),
    }];
    if !observation_points.is_empty() {
        let radial_residuals = observation_points
            .iter()
            .map(|point| {
                targets
                    .iter()
                    .filter_map(|target| exact_target_point_residual(target, *point, geometries))
                    .fold(f64::INFINITY, f64::min)
            })
            .collect::<Vec<_>>();
        if radial_residuals.iter().any(|value| !value.is_finite()) {
            return Err(AppError::validation(format!(
                "Capture expectation '{}' exact cylinder surface metric is degenerate.",
                expectation.expectation_id
            )));
        }
        let maximum_radial = radial_residuals.iter().copied().fold(0.0, f64::max);
        let rms_radial = (radial_residuals
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            / radial_residuals.len() as f64)
            .sqrt();
        if expectation
            .radial_tolerance_mm
            .is_some_and(|tolerance| maximum_radial > tolerance)
        {
            return Err(AppError::validation(format!(
                "Capture expectation '{}' cylinder radial residual {:.6} mm exceeds tolerance {:.6} mm.",
                expectation.expectation_id,
                maximum_radial,
                expectation.radial_tolerance_mm.unwrap_or_default()
            )));
        }
        components.push(crate::contracts::CaptureCorrespondenceResidualComponent {
            metric: "pointToExactCylinderSurface".into(),
            maximum: maximum_radial,
            rms: rms_radial,
            unit: "mm".into(),
        });
    } else if expectation.radial_tolerance_mm.is_some() {
        return Err(AppError::validation(format!(
            "Capture expectation '{}' declares radial tolerance without cylinder surface evidence points.",
            expectation.expectation_id
        )));
    }

    Ok(crate::contracts::CaptureCorrespondenceResidual {
        metric: "axisToExactCylinderAxis".into(),
        maximum,
        rms,
        unit: "mm".into(),
        components,
    })
}

fn exact_geometry_matches_expectation(
    geometry: &ExactBrepTargetGeometry,
    expected: &CaptureExpectedGeometryKind,
) -> bool {
    match expected {
        CaptureExpectedGeometryKind::Point => {
            matches!(geometry, ExactBrepTargetGeometry::Vertex { .. })
        }
        CaptureExpectedGeometryKind::Curve | CaptureExpectedGeometryKind::Profile => matches!(
            geometry,
            ExactBrepTargetGeometry::LineEdge { .. } | ExactBrepTargetGeometry::CircleEdge { .. }
        ),
        CaptureExpectedGeometryKind::Plane => {
            matches!(geometry, ExactBrepTargetGeometry::PlaneFace { .. })
        }
        CaptureExpectedGeometryKind::Cylinder => matches!(
            geometry,
            ExactBrepTargetGeometry::CylinderFace { .. }
                | ExactBrepTargetGeometry::CircleEdge { .. }
        ),
    }
}

fn exact_point_residual(geometry: &ExactBrepTargetGeometry, point: [f64; 3]) -> Option<f64> {
    match geometry {
        ExactBrepTargetGeometry::Vertex { point: target } => Some(norm(sub(point, *target))),
        ExactBrepTargetGeometry::LineEdge { start, end } => {
            let segment = sub(*end, *start);
            let length_squared = dot(segment, segment);
            if length_squared <= 1.0e-18 {
                return None;
            }
            let parameter = (dot(sub(point, *start), segment) / length_squared).clamp(0.0, 1.0);
            Some(norm(sub(point, add(*start, scale(segment, parameter)))))
        }
        ExactBrepTargetGeometry::CircleEdge {
            center,
            normal,
            x_direction,
            radius,
            first_parameter,
            last_parameter,
        } => exact_circle_edge_residual(
            point,
            *center,
            *normal,
            *x_direction,
            *radius,
            *first_parameter,
            *last_parameter,
        ),
        ExactBrepTargetGeometry::PlaneFace { origin, normal, .. } => {
            Some(dot(sub(point, *origin), normalized(*normal)?).abs())
        }
        ExactBrepTargetGeometry::CylinderFace {
            axis_origin,
            axis_direction,
            radius,
            ..
        } => {
            let axis = normalized(*axis_direction)?;
            let offset = sub(point, *axis_origin);
            let radial = sub(offset, scale(axis, dot(offset, axis)));
            Some((norm(radial) - radius).abs())
        }
    }
}

fn exact_target_point_residual(
    geometry: &ExactBrepTargetGeometry,
    point: [f64; 3],
    geometries: &BTreeMap<String, ExactBrepTargetGeometry>,
) -> Option<f64> {
    match geometry {
        ExactBrepTargetGeometry::PlaneFace {
            origin,
            normal,
            boundary_edge_target_ids,
        } => exact_bounded_plane_face_residual(
            point,
            *origin,
            *normal,
            boundary_edge_target_ids,
            geometries,
        ),
        ExactBrepTargetGeometry::CylinderFace {
            axis_origin,
            axis_direction,
            radius,
            boundary_edge_target_ids,
        } => exact_bounded_cylinder_face_residual(
            point,
            *axis_origin,
            *axis_direction,
            *radius,
            boundary_edge_target_ids,
            geometries,
        ),
        _ => exact_point_residual(geometry, point),
    }
}

fn exact_bounded_cylinder_face_residual(
    point: [f64; 3],
    axis_origin: [f64; 3],
    axis_direction: [f64; 3],
    radius: f64,
    boundary_edge_target_ids: &[Vec<String>],
    geometries: &BTreeMap<String, ExactBrepTargetGeometry>,
) -> Option<f64> {
    if !radius.is_finite() || radius <= 0.0 || boundary_edge_target_ids.is_empty() {
        return None;
    }
    let axis = normalized(axis_direction)?;
    let mut axial_bounds = Vec::new();
    let mut full_circle_count = 0usize;
    for target_id in boundary_edge_target_ids.iter().flatten() {
        match geometries.get(target_id)? {
            ExactBrepTargetGeometry::CircleEdge {
                center,
                normal,
                radius: edge_radius,
                first_parameter,
                last_parameter,
                ..
            } => {
                if normalized_dot_abs(*normal, axis) < 1.0 - 1.0e-10
                    || (*edge_radius - radius).abs() > 1.0e-9
                    || *last_parameter - *first_parameter < std::f64::consts::TAU - 1.0e-10
                    || norm(sub(
                        sub(*center, axis_origin),
                        scale(axis, dot(sub(*center, axis_origin), axis)),
                    )) > 1.0e-9
                {
                    return None;
                }
                axial_bounds.push(dot(sub(*center, axis_origin), axis));
                full_circle_count += 1;
            }
            ExactBrepTargetGeometry::LineEdge { start, end } => {
                let segment = sub(*end, *start);
                if normalized_dot_abs(segment, axis) < 1.0 - 1.0e-10 {
                    return None;
                }
                for endpoint in [*start, *end] {
                    let offset = sub(endpoint, axis_origin);
                    if (norm(sub(offset, scale(axis, dot(offset, axis)))) - radius).abs() > 1.0e-9 {
                        return None;
                    }
                    axial_bounds.push(dot(offset, axis));
                }
            }
            _ => return None,
        }
    }
    if full_circle_count != 2 || axial_bounds.len() < 2 {
        return None;
    }
    let minimum_axis = axial_bounds.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum_axis = axial_bounds
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    if !minimum_axis.is_finite() || maximum_axis - minimum_axis <= 1.0e-10 {
        return None;
    }
    let offset = sub(point, axis_origin);
    let axial = dot(offset, axis);
    let radial = norm(sub(offset, scale(axis, axial)));
    let axial_excess = if axial < minimum_axis {
        minimum_axis - axial
    } else if axial > maximum_axis {
        axial - maximum_axis
    } else {
        0.0
    };
    Some(((radial - radius).powi(2) + axial_excess.powi(2)).sqrt())
}

fn exact_bounded_plane_face_residual(
    point: [f64; 3],
    origin: [f64; 3],
    normal: [f64; 3],
    boundary_edge_target_ids: &[Vec<String>],
    geometries: &BTreeMap<String, ExactBrepTargetGeometry>,
) -> Option<f64> {
    if boundary_edge_target_ids.is_empty() {
        return None;
    }
    let normal = normalized(normal)?;
    let signed_distance = dot(sub(point, origin), normal);
    let projected = sub(point, scale(normal, signed_distance));
    let mut inside = false;
    let mut boundary_residual = f64::INFINITY;
    for loop_ids in boundary_edge_target_ids {
        if loop_ids.is_empty() {
            return None;
        }
        let loop_geometries = loop_ids
            .iter()
            .map(|target_id| geometries.get(target_id))
            .collect::<Option<Vec<_>>>()?;
        if point_inside_exact_planar_loop(projected, origin, normal, &loop_geometries)? {
            inside = !inside;
        }
        boundary_residual = boundary_residual.min(
            loop_geometries
                .iter()
                .filter_map(|edge| exact_point_residual(edge, point))
                .fold(f64::INFINITY, f64::min),
        );
    }
    if inside {
        Some(signed_distance.abs())
    } else {
        boundary_residual.is_finite().then_some(boundary_residual)
    }
}

fn point_inside_exact_planar_loop(
    point: [f64; 3],
    origin: [f64; 3],
    normal: [f64; 3],
    edges: &[&ExactBrepTargetGeometry],
) -> Option<bool> {
    if edges.len() == 1 {
        if let ExactBrepTargetGeometry::CircleEdge {
            center,
            normal: circle_normal,
            radius,
            first_parameter,
            last_parameter,
            ..
        } = edges[0]
        {
            if last_parameter - first_parameter < std::f64::consts::TAU - 1.0e-10
                || normalized_dot_abs(normal, *circle_normal) < 1.0 - 1.0e-10
            {
                return None;
            }
            let offset = sub(point, *center);
            let planar = sub(offset, scale(normal, dot(offset, normal)));
            return Some(norm(planar) <= *radius + 1.0e-10);
        }
    }
    if !edges
        .iter()
        .all(|edge| matches!(edge, ExactBrepTargetGeometry::LineEdge { .. }))
    {
        return None;
    }
    let u_axis = edges.iter().find_map(|edge| match edge {
        ExactBrepTargetGeometry::LineEdge { start, end } => {
            let direction = sub(*end, *start);
            normalized(sub(direction, scale(normal, dot(direction, normal))))
        }
        _ => None,
    })?;
    let v_axis = cross(normal, u_axis);
    let point_2d = [
        dot(sub(point, origin), u_axis),
        dot(sub(point, origin), v_axis),
    ];
    let mut crossings = 0_u32;
    for edge in edges {
        let ExactBrepTargetGeometry::LineEdge { start, end } = edge else {
            return None;
        };
        if exact_point_residual(edge, point).is_some_and(|distance| distance <= 1.0e-10) {
            return Some(true);
        }
        let start = [
            dot(sub(*start, origin), u_axis),
            dot(sub(*start, origin), v_axis),
        ];
        let end = [
            dot(sub(*end, origin), u_axis),
            dot(sub(*end, origin), v_axis),
        ];
        if (start[1] > point_2d[1]) != (end[1] > point_2d[1]) {
            let intersection_x =
                (end[0] - start[0]) * (point_2d[1] - start[1]) / (end[1] - start[1]) + start[0];
            if point_2d[0] < intersection_x {
                crossings += 1;
            }
        }
    }
    Some(crossings % 2 == 1)
}

fn exact_circle_edge_residual(
    point: [f64; 3],
    center: [f64; 3],
    normal: [f64; 3],
    x_direction: [f64; 3],
    radius: f64,
    first_parameter: f64,
    last_parameter: f64,
) -> Option<f64> {
    if !radius.is_finite()
        || radius <= 0.0
        || !first_parameter.is_finite()
        || !last_parameter.is_finite()
        || last_parameter < first_parameter
    {
        return None;
    }
    let normal = normalized(normal)?;
    let x_direction = normalized(sub(x_direction, scale(normal, dot(x_direction, normal))))?;
    let y_direction = cross(normal, x_direction);
    let offset = sub(point, center);
    let projected_x = dot(offset, x_direction);
    let projected_y = dot(offset, y_direction);
    let angle = projected_y.atan2(projected_x);
    let tau = std::f64::consts::TAU;
    let span = last_parameter - first_parameter;
    let interior_parameter = if span >= tau - 1.0e-10 {
        Some(angle)
    } else {
        let turns = ((first_parameter - angle) / tau).ceil();
        let parameter = angle + turns * tau;
        (parameter <= last_parameter + 1.0e-12).then_some(parameter)
    };
    let circle_point = |parameter: f64| {
        add(
            center,
            add(
                scale(x_direction, radius * parameter.cos()),
                scale(y_direction, radius * parameter.sin()),
            ),
        )
    };
    if let Some(parameter) = interior_parameter {
        return Some(norm(sub(point, circle_point(parameter))));
    }
    Some(
        norm(sub(point, circle_point(first_parameter)))
            .min(norm(sub(point, circle_point(last_parameter)))),
    )
}

fn normalized_dot_abs(first: [f64; 3], second: [f64; 3]) -> f64 {
    normalized(first)
        .zip(normalized(second))
        .map(|(first, second)| dot(first, second).abs().clamp(0.0, 1.0))
        .unwrap_or(0.0)
}

fn normalized(value: [f64; 3]) -> Option<[f64; 3]> {
    let length = norm(value);
    (length.is_finite() && length > 1.0e-12).then(|| scale(value, 1.0 / length))
}

fn add(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[0] + second[0],
        first[1] + second[1],
        first[2] + second[2],
    ]
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

fn cross(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ]
}

fn norm(value: [f64; 3]) -> f64 {
    dot(value, value).sqrt()
}

fn resolve_tagged_expectation(
    expectation: &crate::contracts::CaptureFeatureExpectation,
    binding: &TaggedAnchorBinding,
    selection_targets: &[SelectionTarget],
    part_source_geometry_digests: &BTreeMap<String, String>,
) -> AppResult<CaptureEvidenceCorrespondence> {
    let requested_ids = binding
        .target_ids
        .iter()
        .chain(&binding.durable_target_ids)
        .chain(&binding.canonical_target_ids)
        .chain(&binding.alias_ids)
        .cloned()
        .collect::<Vec<_>>();
    resolve_authored_expectation(
        expectation,
        &binding.target,
        &requested_ids,
        "Authored tag",
        false,
        selection_targets,
        part_source_geometry_digests,
    )
}

fn resolve_authored_expectation(
    expectation: &crate::contracts::CaptureFeatureExpectation,
    target_part_id: &str,
    requested_ids: &[String],
    selector_label: &str,
    filter_unrelated_topology_kinds: bool,
    selection_targets: &[SelectionTarget],
    part_source_geometry_digests: &BTreeMap<String, String>,
) -> AppResult<CaptureEvidenceCorrespondence> {
    if expectation.instance_path.is_some() {
        return Err(AppError::validation(format!(
            "Capture expectation '{}' requests instancePath, but exact preview target has no instance identity.",
            expectation.expectation_id
        )));
    }
    if target_part_id != expectation.part_id {
        return Err(AppError::conflict(format!(
            "{} for capture expectation '{}' targets part '{}', expected '{}'.",
            selector_label, expectation.expectation_id, target_part_id, expectation.part_id
        )));
    }
    let expected_selection_kind =
        expected_selection_kind(&expectation.required_brep_topology_kind)?;
    let mut resolved = Vec::<&SelectionTarget>::new();
    let mut seen = HashSet::<String>::new();
    for requested_id in requested_ids {
        let Some(target) = selection_targets
            .iter()
            .find(|target| selection_target_matches_id(target, requested_id))
        else {
            return Err(AppError::conflict(format!(
                "{} for capture expectation '{}' references stale or unknown target '{}'.",
                selector_label, expectation.expectation_id, requested_id
            )));
        };
        let identity = target
            .canonical_target_id
            .as_ref()
            .or(target.target_id.as_ref())
            .cloned()
            .ok_or_else(|| AppError::validation("Exact BRep selection target has no identity."))?;
        if target.part_id != expectation.part_id {
            return Err(AppError::validation(format!(
                "{} for capture expectation '{}' resolved wrong part.",
                selector_label, expectation.expectation_id
            )));
        }
        if target.kind != expected_selection_kind {
            if filter_unrelated_topology_kinds {
                continue;
            }
            return Err(AppError::validation(format!(
                "{} for capture expectation '{}' resolved wrong BRep topology kind.",
                selector_label, expectation.expectation_id
            )));
        }
        if seen.insert(identity) {
            resolved.push(target);
        }
    }
    if resolved.is_empty() {
        return Err(AppError::validation(format!(
            "{} for capture expectation '{}' resolved no exact BRep targets.",
            selector_label, expectation.expectation_id
        )));
    }
    if expectation.cardinality == CaptureSelectorCardinality::One && resolved.len() != 1 {
        return Err(AppError::validation(format!(
            "Capture expectation '{}' requires exactly one target, resolved {}.",
            expectation.expectation_id,
            resolved.len()
        )));
    }
    let source_geometry_digest = part_source_geometry_digests
        .get(&expectation.part_id)
        .ok_or_else(|| {
            AppError::validation(format!(
                "Exact preview has no source geometry digest for part '{}'.",
                expectation.part_id
            ))
        })?
        .clone();
    require_digest("part source geometry", &source_geometry_digest)?;

    let canonical_target_ids = resolved
        .iter()
        .map(|target| {
            target
                .canonical_target_id
                .clone()
                .or_else(|| target.target_id.clone())
                .ok_or_else(|| AppError::validation("Exact BRep target has no canonical identity."))
        })
        .collect::<AppResult<Vec<_>>>()?;
    let durable_target_ids = resolved
        .iter()
        .map(|target| {
            target.durable_target_id.clone().ok_or_else(|| {
                AppError::validation(format!(
                    "Exact BRep target for capture expectation '{}' has no durable identity.",
                    expectation.expectation_id
                ))
            })
        })
        .collect::<AppResult<Vec<_>>>()?;
    let source_stable_node_keys = durable_target_ids
        .iter()
        .filter_map(|target_id| stable_node_key_from_durable_target(target_id))
        .collect::<Vec<_>>();

    Ok(CaptureEvidenceCorrespondence {
        expectation_id: expectation.expectation_id.clone(),
        guide_item_ids: expectation.guide_item_ids.clone(),
        part_id: expectation.part_id.clone(),
        instance_path: None,
        authored_selector: expectation.expected_authored_selector.clone(),
        selector_cardinality: expectation.cardinality.clone(),
        brep_target_kind: expectation.required_brep_topology_kind.clone(),
        canonical_target_ids,
        durable_target_ids,
        source_stable_node_keys,
        source_geometry_digest,
        relation: expectation_relation(expectation.expected_geometry_kind.clone()),
        residual: None,
        status: CaptureCorrespondenceStatus::Satisfied,
    })
}

fn expected_selection_kind(
    kind: &CaptureRequiredBrepTopologyKind,
) -> AppResult<SelectionTargetKind> {
    match kind {
        CaptureRequiredBrepTopologyKind::Vertex => Ok(SelectionTargetKind::Vertex),
        CaptureRequiredBrepTopologyKind::Edge | CaptureRequiredBrepTopologyKind::OrderedEdges => {
            Ok(SelectionTargetKind::Edge)
        }
        CaptureRequiredBrepTopologyKind::Face => Ok(SelectionTargetKind::Face),
    }
}

fn selection_target_matches_id(target: &SelectionTarget, requested_id: &str) -> bool {
    target.target_id.as_deref() == Some(requested_id)
        || target.durable_target_id.as_deref() == Some(requested_id)
        || target.canonical_target_id.as_deref() == Some(requested_id)
        || target.alias_ids.iter().any(|alias| alias == requested_id)
}

fn stable_node_key_from_durable_target(target_id: &str) -> Option<String> {
    let (_, remainder) = target_id.split_once(":stable-node-key:")?;
    let marker = [":vertex:", ":edge:", ":face:"]
        .into_iter()
        .filter_map(|marker| remainder.find(marker).map(|index| (index, marker)))
        .min_by_key(|(index, _)| *index)?;
    let value = remainder[..marker.0].trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn expectation_relation(kind: CaptureExpectedGeometryKind) -> CaptureCorrespondenceRelation {
    match kind {
        CaptureExpectedGeometryKind::Point | CaptureExpectedGeometryKind::Curve => {
            CaptureCorrespondenceRelation::Observes
        }
        CaptureExpectedGeometryKind::Plane => CaptureCorrespondenceRelation::DefinesSurface,
        CaptureExpectedGeometryKind::Cylinder => CaptureCorrespondenceRelation::DefinesAxis,
        CaptureExpectedGeometryKind::Profile => CaptureCorrespondenceRelation::Profiles,
    }
}

fn require_digest(label: &str, value: &str) -> AppResult<()> {
    if value.starts_with("sha256:") && value.len() > "sha256:".len() {
        Ok(())
    } else {
        Err(AppError::validation(format!(
            "Capture {label} digest is invalid."
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{CaptureAuthoredSelector, CaptureReconstructionGuide};

    fn manifest() -> ModelManifest {
        serde_json::from_value(serde_json::json!({
            "modelId": "preview-1",
            "sourceKind": "generated",
            "sourceDigest": "sha256:generated",
            "document": { "documentName": "preview", "documentLabel": "Preview" },
            "parts": [{
                "partId": "part-1",
                "freecadObjectName": "part-1",
                "label": "Part",
                "kind": "Part",
                "editable": false
            }],
            "selectionTargets": [{
                "targetId": "part-1:face:0-0-0:100",
                "durableTargetId": "part-1:stable-node-key:sha256:node:face:0-0-0:100",
                "canonicalTargetId": "part-1:face:0:0-0-0:100",
                "partId": "part-1",
                "viewerNodeId": "part-1",
                "label": "support",
                "kind": "face",
                "editable": false
            }],
            "taggedAnchors": {
                "support-face": {
                    "kind": "face",
                    "authoredSelector": "bottom",
                    "target": "part-1",
                    "targetIds": ["part-1:face:0-0-0:100"],
                    "durableTargetIds": ["part-1:stable-node-key:sha256:node:face:0-0-0:100"],
                    "canonicalTargetIds": ["part-1:face:0:0-0-0:100"]
                }
            }
        }))
        .expect("manifest")
    }

    fn ready_guide() -> CaptureReconstructionGuide {
        let mut guide = CaptureReconstructionGuide::test_fixture();
        guide
            .calibration
            .measurements
            .push(crate::contracts::CaptureKnownDistanceMeasurement {
                measurement_id: "calibration-1".into(),
                label: "known".into(),
                first_landmark_id: "landmark-1".into(),
                second_landmark_id: "landmark-2".into(),
                known_distance_mm: 1.0,
                fitted_distance_mm: 1.0,
                residual_mm: 0.0,
                accepted_tolerance_mm: 0.1,
            });
        guide.canonical_digest = guide.compute_canonical_digest().unwrap();
        guide
    }

    fn exact_geometries() -> BTreeMap<String, ExactBrepTargetGeometry> {
        BTreeMap::from([
            (
                "part-1:face:0:0-0-0:100".into(),
                ExactBrepTargetGeometry::PlaneFace {
                    origin: [0.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    boundary_edge_target_ids: vec![vec![
                        "support-edge-a".into(),
                        "support-edge-b".into(),
                        "support-edge-c".into(),
                        "support-edge-d".into(),
                    ]],
                },
            ),
            (
                "support-edge-a".into(),
                ExactBrepTargetGeometry::LineEdge {
                    start: [-2.0, -2.0, 0.0],
                    end: [2.0, -2.0, 0.0],
                },
            ),
            (
                "support-edge-b".into(),
                ExactBrepTargetGeometry::LineEdge {
                    start: [2.0, -2.0, 0.0],
                    end: [2.0, 2.0, 0.0],
                },
            ),
            (
                "support-edge-c".into(),
                ExactBrepTargetGeometry::LineEdge {
                    start: [2.0, 2.0, 0.0],
                    end: [-2.0, 2.0, 0.0],
                },
            ),
            (
                "support-edge-d".into(),
                ExactBrepTargetGeometry::LineEdge {
                    start: [-2.0, 2.0, 0.0],
                    end: [-2.0, -2.0, 0.0],
                },
            ),
        ])
    }

    #[test]
    fn exact_tag_resolution_records_canonical_durable_and_geometry_identity() {
        let guide = ready_guide();
        let provenance = validate_capture_brep_correspondences(
            &guide,
            &manifest(),
            &BTreeMap::from([("part-1".into(), "sha256:part-geometry".into())]),
            &exact_geometries(),
            "sha256:generated",
            "sha256:bundle-geometry",
        )
        .expect("exact correspondence");

        let correspondence = &provenance.correspondences[0];
        assert_eq!(
            correspondence.status,
            CaptureCorrespondenceStatus::Satisfied
        );
        assert_eq!(
            correspondence.canonical_target_ids,
            vec!["part-1:face:0:0-0-0:100"]
        );
        assert_eq!(
            correspondence.source_geometry_digest,
            "sha256:part-geometry"
        );
        assert_eq!(correspondence.source_stable_node_keys, vec!["sha256:node"]);
    }

    #[test]
    fn guided_source_semantics_require_named_fit_binding_and_explicit_symmetry() {
        let mut guide = ready_guide();
        guide.symmetry_completion = crate::contracts::CaptureSymmetryCompletion::Half {
            plane_id: "plane-1".into(),
        };
        guide.feature_expectations[0].expected_authored_selector =
            CaptureAuthoredSelector::Binding {
                name: "support".into(),
            };
        guide
            .measurements
            .push(crate::contracts::CaptureNamedMeasurement {
                measurement_id: "clearance-measurement".into(),
                label: "clearance".into(),
                landmark_ids: vec!["landmark-1".into(), "landmark-2".into()],
                value: 0.2,
                unit: "mm".into(),
                fit_critical: true,
                authored_parameter_name: Some("clearance".into()),
                constraint_kind: Some(crate::contracts::CaptureConstraintKind::Clearance),
            });

        validate_capture_guided_source_semantics(
            &guide,
            r#"(model
              (params (number clearance 0.2))
              (part part-1 (build
                (shape support (box 10 10 clearance))
                (result (union support (mirror "x" 0 support))))))"#,
        )
        .expect("named parameter and explicit mirror");

        let error = validate_capture_guided_source_semantics(
            &guide,
            r#"(model
              (params (number clearance 0.2))
              (part part-1 (build
                (shape support (box 10 10 clearance))
                (result support))))"#,
        )
        .expect_err("implicit symmetry")
        .message;
        assert!(error.contains("requires 1 explicit mirror"), "{error}");

        let error = validate_capture_guided_source_semantics(
            &guide,
            r#"(model
              (part part-1 (build
                (shape support (box 10 10 1))
                (result (union support (mirror "x" 0 support))))))"#,
        )
        .expect_err("missing fit parameter")
        .message;
        assert!(
            error.contains("missing authored parameter/binding 'clearance'"),
            "{error}"
        );

        guide.symmetry_completion = crate::contracts::CaptureSymmetryCompletion::Quarter {
            first_plane_id: "plane-1".into(),
            second_plane_id: "plane-2".into(),
        };
        let error = validate_capture_guided_source_semantics(
            &guide,
            r#"(model
              (params (number clearance 0.2))
              (part part-1 (build
                (shape support (box 10 10 clearance))
                (shape first-half (union support (mirror "x" 0 support)))
                (result (union first-half (mirror "x" 0 first-half))))))"#,
        )
        .expect_err("quarter needs distinct mirror planes")
        .message;
        assert!(
            error.contains("two distinct explicit mirror planes"),
            "{error}"
        );

        validate_capture_guided_source_semantics(
            &guide,
            r#"(model
              (params (number clearance 0.2))
              (part part-1 (build
                (shape support (box 10 10 clearance))
                (shape first-half (union support (mirror "x" 0 support)))
                (result (union first-half (mirror "y" 0 first-half))))))"#,
        )
        .expect("quarter uses distinct X/Y mirror operations");
    }

    #[test]
    fn exact_authored_binding_resolution_records_topology_correspondence() {
        let mut guide = ready_guide();
        guide.feature_expectations[0].expected_authored_selector =
            CaptureAuthoredSelector::Binding {
                name: "base".into(),
            };
        guide.canonical_digest = guide.compute_canonical_digest().unwrap();
        let binding_targets = BTreeMap::from([(
            ("part-1".into(), "base".into()),
            vec![
                "part-1:vertex:0:0-0-0".into(),
                "part-1:face:0:0-0-0:100".into(),
            ],
        )]);
        let mut manifest = manifest();
        manifest.selection_targets.push(SelectionTarget {
            target_id: Some("part-1:vertex:0-0-0".into()),
            durable_target_id: Some("part-1:stable-node-key:sha256:node:vertex:0-0-0".into()),
            canonical_target_id: Some("part-1:vertex:0:0-0-0".into()),
            alias_ids: Vec::new(),
            part_id: "part-1".into(),
            viewer_node_id: "part-1".into(),
            label: "corner".into(),
            kind: SelectionTargetKind::Vertex,
            editable: false,
            parameter_keys: Vec::new(),
            primitive_ids: Vec::new(),
            view_ids: Vec::new(),
        });

        let provenance = validate_capture_brep_correspondences_with_bindings(
            &guide,
            &manifest,
            &BTreeMap::from([("part-1".into(), "sha256:part-geometry".into())]),
            &exact_geometries(),
            &binding_targets,
            &BTreeMap::new(),
            "sha256:generated",
            "sha256:bundle-geometry",
        )
        .expect("exact binding correspondence");

        assert_eq!(
            provenance.correspondences[0].canonical_target_ids,
            vec!["part-1:face:0:0-0-0:100"]
        );
        assert_eq!(
            provenance.correspondences[0].status,
            CaptureCorrespondenceStatus::Satisfied
        );
    }

    #[test]
    fn exact_gate_rejects_source_divergence_wrong_kind_cardinality_and_unmapped_binding() {
        let guide = ready_guide();
        let digests = BTreeMap::from([("part-1".into(), "sha256:part-geometry".into())]);
        let error = validate_capture_brep_correspondences(
            &guide,
            &manifest(),
            &digests,
            &exact_geometries(),
            "sha256:other",
            "sha256:bundle-geometry",
        )
        .expect_err("source divergence")
        .message;
        assert!(error.contains("differs"), "{error}");

        let mut wrong_kind = manifest();
        wrong_kind.selection_targets[0].kind = SelectionTargetKind::Edge;
        let error = validate_capture_brep_correspondences(
            &guide,
            &wrong_kind,
            &digests,
            &exact_geometries(),
            "sha256:generated",
            "sha256:bundle-geometry",
        )
        .expect_err("wrong kind")
        .message;
        assert!(error.contains("wrong BRep topology kind"), "{error}");

        let mut binding_guide = guide.clone();
        binding_guide.feature_expectations[0].expected_authored_selector =
            CaptureAuthoredSelector::Binding {
                name: "support".into(),
            };
        binding_guide.canonical_digest = binding_guide.compute_canonical_digest().unwrap();
        let error = validate_capture_brep_correspondences(
            &binding_guide,
            &manifest(),
            &digests,
            &exact_geometries(),
            "sha256:generated",
            "sha256:bundle-geometry",
        )
        .expect_err("unmapped binding")
        .message;
        assert!(error.contains("no exact topology mapping"), "{error}");
    }

    #[test]
    fn target_kind_metrics_are_exact_and_tolerance_gated() {
        assert_eq!(
            exact_point_residual(
                &ExactBrepTargetGeometry::Vertex {
                    point: [1.0, 2.0, 3.0],
                },
                [1.0, 2.0, 5.0],
            ),
            Some(2.0)
        );
        let bounded_cylinder = BTreeMap::from([
            (
                "cylinder".into(),
                ExactBrepTargetGeometry::CylinderFace {
                    axis_origin: [0.0, 0.0, 0.0],
                    axis_direction: [0.0, 0.0, 1.0],
                    radius: 1.0,
                    boundary_edge_target_ids: vec![vec!["bottom".into(), "top".into()]],
                },
            ),
            (
                "bottom".into(),
                ExactBrepTargetGeometry::CircleEdge {
                    center: [0.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    x_direction: [1.0, 0.0, 0.0],
                    radius: 1.0,
                    first_parameter: 0.0,
                    last_parameter: std::f64::consts::TAU,
                },
            ),
            (
                "top".into(),
                ExactBrepTargetGeometry::CircleEdge {
                    center: [0.0, 0.0, 2.0],
                    normal: [0.0, 0.0, 1.0],
                    x_direction: [1.0, 0.0, 0.0],
                    radius: 1.0,
                    first_parameter: 0.0,
                    last_parameter: std::f64::consts::TAU,
                },
            ),
        ]);
        assert_eq!(
            exact_target_point_residual(
                bounded_cylinder.get("cylinder").unwrap(),
                [1.0, 0.0, 10.0],
                &bounded_cylinder,
            ),
            Some(8.0)
        );
        assert_eq!(
            exact_point_residual(
                &ExactBrepTargetGeometry::LineEdge {
                    start: [0.0, 0.0, 0.0],
                    end: [1.0, 0.0, 0.0],
                },
                [2.0, 1.0, 0.0],
            ),
            Some(2.0_f64.sqrt())
        );
        assert_eq!(
            exact_point_residual(
                &ExactBrepTargetGeometry::CylinderFace {
                    axis_origin: [0.0, 0.0, 0.0],
                    axis_direction: [0.0, 0.0, 1.0],
                    radius: 5.0,
                    boundary_edge_target_ids: Vec::new(),
                },
                [7.0, 0.0, 10.0],
            ),
            Some(2.0)
        );
        assert!(exact_point_residual(
            &ExactBrepTargetGeometry::CircleEdge {
                center: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                x_direction: [1.0, 0.0, 0.0],
                radius: 2.0,
                first_parameter: 0.0,
                last_parameter: std::f64::consts::FRAC_PI_2,
            },
            [3.0 / 2.0_f64.sqrt(), 3.0 / 2.0_f64.sqrt(), 0.0],
        )
        .is_some_and(|residual| (residual - 1.0).abs() < 1.0e-12));
        assert!(exact_point_residual(
            &ExactBrepTargetGeometry::CircleEdge {
                center: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                x_direction: [1.0, 0.0, 0.0],
                radius: 2.0,
                first_parameter: 0.0,
                last_parameter: std::f64::consts::FRAC_PI_2,
            },
            [-2.0, 0.0, 0.0],
        )
        .is_some_and(|residual| (residual - 8.0_f64.sqrt()).abs() < 1.0e-12));

        let guide = ready_guide();
        let mut shifted_plane = exact_geometries();
        for geometry in shifted_plane.values_mut() {
            match geometry {
                ExactBrepTargetGeometry::PlaneFace { origin, .. } => origin[2] = 1.0,
                ExactBrepTargetGeometry::LineEdge { start, end } => {
                    start[2] = 1.0;
                    end[2] = 1.0;
                }
                _ => {}
            }
        }
        let error = validate_capture_brep_correspondences(
            &guide,
            &manifest(),
            &BTreeMap::from([("part-1".into(), "sha256:part-geometry".into())]),
            &shifted_plane,
            "sha256:generated",
            "sha256:bundle-geometry",
        )
        .expect_err("plane residual above tolerance")
        .message;
        assert!(error.contains("position residual 1.000000 mm"), "{error}");

        let wrong_normal = BTreeMap::from([
            (
                "part-1:face:0:0-0-0:100".into(),
                ExactBrepTargetGeometry::PlaneFace {
                    origin: [0.0, 0.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                    boundary_edge_target_ids: vec![vec![
                        "normal-edge-a".into(),
                        "normal-edge-b".into(),
                        "normal-edge-c".into(),
                        "normal-edge-d".into(),
                    ]],
                },
            ),
            (
                "normal-edge-a".into(),
                ExactBrepTargetGeometry::LineEdge {
                    start: [-2.0, 0.0, -2.0],
                    end: [2.0, 0.0, -2.0],
                },
            ),
            (
                "normal-edge-b".into(),
                ExactBrepTargetGeometry::LineEdge {
                    start: [2.0, 0.0, -2.0],
                    end: [2.0, 0.0, 2.0],
                },
            ),
            (
                "normal-edge-c".into(),
                ExactBrepTargetGeometry::LineEdge {
                    start: [2.0, 0.0, 2.0],
                    end: [-2.0, 0.0, 2.0],
                },
            ),
            (
                "normal-edge-d".into(),
                ExactBrepTargetGeometry::LineEdge {
                    start: [-2.0, 0.0, 2.0],
                    end: [-2.0, 0.0, -2.0],
                },
            ),
        ]);
        let error = validate_capture_brep_correspondences(
            &guide,
            &manifest(),
            &BTreeMap::from([("part-1".into(), "sha256:part-geometry".into())]),
            &wrong_normal,
            "sha256:generated",
            "sha256:bundle-geometry",
        )
        .expect_err("exact face normal above tolerance")
        .message;
        assert!(error.contains("normal residual 90.000000 deg"), "{error}");

        let error = validate_capture_brep_correspondences(
            &guide,
            &manifest(),
            &BTreeMap::from([("part-1".into(), "sha256:part-geometry".into())]),
            &BTreeMap::from([(
                "part-1:face:0:0-0-0:100".into(),
                ExactBrepTargetGeometry::CylinderFace {
                    axis_origin: [0.0, 0.0, 0.0],
                    axis_direction: [0.0, 0.0, 1.0],
                    radius: 1.0,
                    boundary_edge_target_ids: Vec::new(),
                },
            )]),
            "sha256:generated",
            "sha256:bundle-geometry",
        )
        .expect_err("plane expectation cannot use cylindrical supporting face")
        .message;
        assert!(
            error.contains("analytic geometry kind is incompatible"),
            "{error}"
        );

        let bounded_plane = BTreeMap::from([
            (
                "part-1:face:0:0-0-0:100".into(),
                ExactBrepTargetGeometry::PlaneFace {
                    origin: [0.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    boundary_edge_target_ids: vec![vec![
                        "edge-a".into(),
                        "edge-b".into(),
                        "edge-c".into(),
                        "edge-d".into(),
                    ]],
                },
            ),
            (
                "edge-a".into(),
                ExactBrepTargetGeometry::LineEdge {
                    start: [10.0, 10.0, 0.0],
                    end: [11.0, 10.0, 0.0],
                },
            ),
            (
                "edge-b".into(),
                ExactBrepTargetGeometry::LineEdge {
                    start: [11.0, 10.0, 0.0],
                    end: [11.0, 11.0, 0.0],
                },
            ),
            (
                "edge-c".into(),
                ExactBrepTargetGeometry::LineEdge {
                    start: [11.0, 11.0, 0.0],
                    end: [10.0, 11.0, 0.0],
                },
            ),
            (
                "edge-d".into(),
                ExactBrepTargetGeometry::LineEdge {
                    start: [10.0, 11.0, 0.0],
                    end: [10.0, 10.0, 0.0],
                },
            ),
        ]);
        let error = validate_capture_brep_correspondences(
            &guide,
            &manifest(),
            &BTreeMap::from([("part-1".into(), "sha256:part-geometry".into())]),
            &bounded_plane,
            "sha256:generated",
            "sha256:bundle-geometry",
        )
        .expect_err("point outside bounded planar face")
        .message;
        assert!(error.contains("position residual 14.142136 mm"), "{error}");
    }

    #[test]
    fn cylinder_expectation_uses_named_axis_offset_and_angle_not_surface_distance() {
        let mut guide = ready_guide();
        guide.axes.push(crate::contracts::CaptureNamedAxis {
            axis_id: "axis-1".into(),
            label: "bore axis".into(),
            landmark_ids: vec!["landmark-1".into(), "landmark-3".into()],
            origin_mm: [0.05, 0.0, 0.0],
            direction: [0.0, 0.0, 1.0],
            fit: crate::contracts::CaptureFitResidual {
                rms_mm: 0.0,
                max_mm: 0.0,
                tolerance_mm: 0.1,
            },
        });
        let expectation = &mut guide.feature_expectations[0];
        expectation.guide_item_ids = vec!["axis-1".into(), "landmark-2".into()];
        expectation.expected_geometry_kind = CaptureExpectedGeometryKind::Cylinder;
        expectation.radial_tolerance_mm = Some(0.1);
        expectation.normal_tolerance_deg = Some(1.0);
        guide.canonical_digest = guide.compute_canonical_digest().unwrap();
        let cylinder = BTreeMap::from([
            (
                "part-1:face:0:0-0-0:100".into(),
                ExactBrepTargetGeometry::CylinderFace {
                    axis_origin: [0.0, 0.0, 0.0],
                    axis_direction: [0.0, 0.0, 1.0],
                    radius: 1.0,
                    boundary_edge_target_ids: vec![vec![
                        "cylinder-bottom".into(),
                        "cylinder-top".into(),
                    ]],
                },
            ),
            (
                "cylinder-bottom".into(),
                ExactBrepTargetGeometry::CircleEdge {
                    center: [0.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    x_direction: [1.0, 0.0, 0.0],
                    radius: 1.0,
                    first_parameter: 0.0,
                    last_parameter: std::f64::consts::TAU,
                },
            ),
            (
                "cylinder-top".into(),
                ExactBrepTargetGeometry::CircleEdge {
                    center: [0.0, 0.0, 2.0],
                    normal: [0.0, 0.0, 1.0],
                    x_direction: [1.0, 0.0, 0.0],
                    radius: 1.0,
                    first_parameter: 0.0,
                    last_parameter: std::f64::consts::TAU,
                },
            ),
        ]);

        let provenance = validate_capture_brep_correspondences(
            &guide,
            &manifest(),
            &BTreeMap::from([("part-1".into(), "sha256:part-geometry".into())]),
            &cylinder,
            "sha256:generated",
            "sha256:bundle-geometry",
        )
        .expect("axis correspondence");
        let residual = provenance.correspondences[0].residual.as_ref().unwrap();
        assert_eq!(residual.metric, "axisToExactCylinderAxis");
        assert!((residual.maximum - 0.05).abs() < 1.0e-12);
        assert!(residual.components.iter().any(|component| {
            component.metric == "pointToExactCylinderSurface" && component.maximum == 0.0
        }));

        guide.axes[0].origin_mm = [0.2, 0.0, 0.0];
        guide.canonical_digest = guide.compute_canonical_digest().unwrap();
        let error = validate_capture_brep_correspondences(
            &guide,
            &manifest(),
            &BTreeMap::from([("part-1".into(), "sha256:part-geometry".into())]),
            &cylinder,
            "sha256:generated",
            "sha256:bundle-geometry",
        )
        .expect_err("axis offset above tolerance")
        .message;
        assert!(
            error.contains("axis offset residual 0.200000 mm"),
            "{error}"
        );
    }

    #[test]
    fn capture_landmark_resolves_to_exact_authored_vertex_tag() {
        let mut guide = ready_guide();
        let expectation = &mut guide.feature_expectations[0];
        expectation.guide_item_ids = vec!["landmark-1".into()];
        expectation.expected_geometry_kind = CaptureExpectedGeometryKind::Point;
        expectation.required_brep_topology_kind = CaptureRequiredBrepTopologyKind::Vertex;
        expectation.expected_authored_selector = CaptureAuthoredSelector::Tag {
            name: "datum-origin".into(),
        };
        expectation.normal_tolerance_deg = None;
        guide.canonical_digest = guide.compute_canonical_digest().unwrap();

        let mut manifest = manifest();
        let target = &mut manifest.selection_targets[0];
        target.target_id = Some("part-1:vertex:0-0-0".into());
        target.durable_target_id = Some("part-1:stable-node-key:sha256:node:vertex:0-0-0".into());
        target.canonical_target_id = Some("part-1:vertex:0:0-0-0".into());
        target.kind = SelectionTargetKind::Vertex;
        manifest.tagged_anchors.clear();
        manifest.tagged_anchors.insert(
            "datum-origin".into(),
            TaggedAnchorBinding {
                kind: crate::contracts::TaggedAnchorKind::Vertex,
                authored_selector: "target-id:part-1:vertex:0:0-0-0".into(),
                target: "part-1".into(),
                target_ids: vec!["part-1:vertex:0-0-0".into()],
                durable_target_ids: vec!["part-1:stable-node-key:sha256:node:vertex:0-0-0".into()],
                canonical_target_ids: vec!["part-1:vertex:0:0-0-0".into()],
                alias_ids: Vec::new(),
            },
        );
        let provenance = validate_capture_brep_correspondences(
            &guide,
            &manifest,
            &BTreeMap::from([("part-1".into(), "sha256:part-geometry".into())]),
            &BTreeMap::from([(
                "part-1:vertex:0:0-0-0".into(),
                ExactBrepTargetGeometry::Vertex {
                    point: [0.0, 0.0, 0.0],
                },
            )]),
            "sha256:generated",
            "sha256:bundle-geometry",
        )
        .expect("vertex correspondence");

        assert_eq!(
            provenance.correspondences[0].brep_target_kind,
            CaptureRequiredBrepTopologyKind::Vertex
        );
        assert_eq!(
            provenance.correspondences[0]
                .residual
                .as_ref()
                .unwrap()
                .metric,
            "pointToVertex"
        );
    }

    #[test]
    fn ordered_profile_requires_corresponding_exact_edge_order() {
        let mut guide = ready_guide();
        let expectation = &mut guide.feature_expectations[0];
        expectation.guide_item_ids = vec!["profile-1".into()];
        expectation.expected_geometry_kind = CaptureExpectedGeometryKind::Profile;
        expectation.required_brep_topology_kind = CaptureRequiredBrepTopologyKind::OrderedEdges;
        expectation.cardinality = CaptureSelectorCardinality::OneOrMore;
        expectation.expected_authored_selector = CaptureAuthoredSelector::Tag {
            name: "profile-edges".into(),
        };
        expectation.position_tolerance_mm = Some(0.1);
        expectation.normal_tolerance_deg = None;
        guide.profiles[0].kind = crate::contracts::CaptureProfileKind::Open;
        guide.profiles[0].landmark_ids = vec![
            "landmark-1".into(),
            "landmark-2".into(),
            "landmark-3".into(),
        ];
        guide.landmarks[2].local_position_mm = [1.0, 1.0, 0.0];
        guide.canonical_digest = guide.compute_canonical_digest().unwrap();

        let edge_target = |suffix: &str| SelectionTarget {
            target_id: Some(format!("part-1:edge:{suffix}")),
            durable_target_id: Some(format!("part-1:stable-node-key:sha256:node:edge:{suffix}")),
            canonical_target_id: Some(format!("part-1:edge:0:{suffix}")),
            alias_ids: Vec::new(),
            part_id: "part-1".into(),
            viewer_node_id: "part-1".into(),
            label: suffix.into(),
            kind: SelectionTargetKind::Edge,
            editable: false,
            parameter_keys: Vec::new(),
            primitive_ids: Vec::new(),
            view_ids: Vec::new(),
        };
        let mut manifest = manifest();
        manifest.selection_targets = vec![edge_target("a"), edge_target("b")];
        manifest.tagged_anchors = BTreeMap::from([(
            "profile-edges".into(),
            TaggedAnchorBinding {
                kind: crate::contracts::TaggedAnchorKind::Edge,
                authored_selector: "profile-edges".into(),
                target: "part-1".into(),
                target_ids: vec!["part-1:edge:a".into(), "part-1:edge:b".into()],
                durable_target_ids: Vec::new(),
                canonical_target_ids: Vec::new(),
                alias_ids: Vec::new(),
            },
        )]);
        let exact = BTreeMap::from([
            (
                "part-1:edge:0:a".into(),
                ExactBrepTargetGeometry::LineEdge {
                    start: [0.0, 0.0, 0.0],
                    end: [1.0, 0.0, 0.0],
                },
            ),
            (
                "part-1:edge:0:b".into(),
                ExactBrepTargetGeometry::LineEdge {
                    start: [1.0, 0.0, 0.0],
                    end: [1.0, 1.0, 0.0],
                },
            ),
        ]);
        let digests = BTreeMap::from([("part-1".into(), "sha256:part-geometry".into())]);

        let provenance = validate_capture_brep_correspondences(
            &guide,
            &manifest,
            &digests,
            &exact,
            "sha256:generated",
            "sha256:bundle-geometry",
        )
        .expect("ordered profile correspondence");
        assert_eq!(
            provenance.correspondences[0]
                .residual
                .as_ref()
                .unwrap()
                .metric,
            "orderedProfileToExactEdges"
        );

        manifest
            .tagged_anchors
            .get_mut("profile-edges")
            .unwrap()
            .target_ids
            .reverse();
        let error = validate_capture_brep_correspondences(
            &guide,
            &manifest,
            &digests,
            &exact,
            "sha256:generated",
            "sha256:bundle-geometry",
        )
        .expect_err("reversed exact edge order")
        .message;
        assert!(error.contains("ordered profile"), "{error}");

        guide.feature_expectations[0].expected_authored_selector =
            CaptureAuthoredSelector::Binding {
                name: "outline".into(),
            };
        guide.canonical_digest = guide.compute_canonical_digest().unwrap();
        let binding_targets = BTreeMap::from([(
            ("part-1".into(), "outline".into()),
            vec!["part-1:edge:a".into(), "part-1:edge:b".into()],
        )]);
        let error = validate_capture_brep_correspondences_with_bindings(
            &guide,
            &manifest,
            &digests,
            &exact,
            &binding_targets,
            &BTreeMap::new(),
            "sha256:generated",
            "sha256:bundle-geometry",
        )
        .expect_err("unordered binding annotations are insufficient for a profile")
        .message;
        assert!(error.contains("no exact ordered edge mapping"), "{error}");

        validate_capture_brep_correspondences_with_bindings(
            &guide,
            &manifest,
            &digests,
            &exact,
            &binding_targets,
            &BTreeMap::from([(
                ("part-1".into(), "outline".into()),
                vec!["part-1:edge:a".into(), "part-1:edge:b".into()],
            )]),
            "sha256:generated",
            "sha256:bundle-geometry",
        )
        .expect("binding profile uses explicit OCCT wire order");
    }

    #[test]
    fn selected_feature_plan_traces_operation_to_authored_node_binding_and_brep_target() {
        let mut guide = CaptureReconstructionGuide::test_fixture();
        guide.reconstructed_profiles = vec![crate::contracts::CaptureReconstructedProfile {
            candidate_id: "profile-candidate:profile-1:polyline".into(),
            source_profile_id: "profile-1".into(),
            support_plane_id: "plane-1".into(),
            segments: Vec::new(),
            closed: true,
            continuous: true,
            closure_error_mm: 0.0,
            maximum_continuity_gap_mm: 0.0,
            support_plane_max_mm: 0.0,
            supporting_evidence_ids: vec!["profile-1".into()],
            rejected_hypotheses: Vec::new(),
        }];
        guide.selected_feature_plan_id = Some("plan:profile-1:extrude".into());
        guide.feature_plan_candidates = vec![crate::contracts::CaptureFeaturePlanCandidate {
            plan_id: "plan:profile-1:extrude".into(),
            label: "profile extrude".into(),
            operations: vec![crate::contracts::CaptureFeatureOperation::Extrude {
                profile_candidate_id: "profile-candidate:profile-1:polyline".into(),
                distance_dimension_id: "depth".into(),
            }],
            supporting_evidence_ids: vec!["profile-1".into()],
            rejecting_evidence: Vec::new(),
            score: 1.0,
            status: crate::contracts::CaptureFeaturePlanStatus::Supported,
        }];
        let correspondences = vec![crate::contracts::CaptureEvidenceCorrespondence {
            expectation_id: "expectation-profile".into(),
            guide_item_ids: vec!["profile-1".into()],
            part_id: "insert".into(),
            instance_path: None,
            authored_selector: CaptureAuthoredSelector::Binding {
                name: "profile_edges".into(),
            },
            selector_cardinality: crate::contracts::CaptureSelectorCardinality::OneOrMore,
            brep_target_kind: CaptureRequiredBrepTopologyKind::OrderedEdges,
            canonical_target_ids: vec!["edge:1".into()],
            durable_target_ids: vec!["durable:edge:1".into()],
            source_stable_node_keys: vec!["node:extrude".into()],
            source_geometry_digest: "sha256:geometry".into(),
            relation: crate::contracts::CaptureCorrespondenceRelation::Profiles,
            residual: None,
            status: CaptureCorrespondenceStatus::Satisfied,
        }];

        let traces = feature_operation_traces(&guide, &correspondences).unwrap();
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].operation_kind, "extrude");
        assert_eq!(traces[0].authored_node_keys, vec!["node:extrude"]);
        assert_eq!(traces[0].authored_binding_names, vec!["profile_edges"]);
        assert_eq!(traces[0].brep_target_ids, vec!["durable:edge:1"]);
    }
}
