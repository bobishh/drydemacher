use std::collections::BTreeMap;

use ecky_fem::{
    FemAcceptanceComparison, FemAcceptanceCriterion, FemApplicabilityCheck,
    FemApplicabilityCheckKind, FemApplicabilityStatus, FemBudgetLimits, FemConstraint,
    FemElementKind, FemEngineeringEvidenceLedger, FemEngineeringQuestion, FemEvidenceAuthority,
    FemEvidenceRecord, FemEvidenceSubject, FemFaceTarget, FemForceVector, FemIdealizationRecord,
    FemInputEvidenceBinding, FemLoad, FemLocalRefinement, FemMaterial, FemMeshControl,
    FemOptionalDisplacement, FemStressVector, FemStudyAssumption, FemStudyAssumptionCategory,
    FemStudyAssumptionStatus, FemValidationEvidence, FemValidationEvidenceKind, FEM_SCHEMA_VERSION,
};
use ecky_render::core_ir::{
    CoreAnalysisClauseKind, CoreAnalysisDecl, CoreAnalysisScalarExpr, CoreParameterValue,
    CoreProgram,
};

use crate::contracts::{AppError, AppResult, TaggedAnchorBinding, TaggedAnchorKind};
use crate::ecky_cad_host::analysis_boundary::AnalysisBoundarySurface;

#[derive(Debug, Clone, PartialEq)]
pub struct FemAuthoredStudy {
    pub analysis_name: String,
    pub part_id: String,
    pub material: FemMaterial,
    pub mesh_control: FemMeshControl,
    pub constraints: Vec<FemConstraint>,
    pub loads: Vec<FemLoad>,
    pub topology_controls: Option<FemAuthoredTopologyControls>,
    pub passive_solid_regions: Vec<FemAuthoredTopologyRegion>,
    pub passive_void_regions: Vec<FemAuthoredTopologyRegion>,
    pub solver_method: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FemAuthoredTopologyControls {
    pub volume_fraction: f64,
    pub penalty: f64,
    pub minimum_density: f64,
    pub filter_radius_mm: f64,
    pub move_limit: f64,
    pub convergence_tolerance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FemAuthoredTopologyRegion {
    pub faces: Vec<FemFaceTarget>,
    pub depth_mm: f64,
}

pub fn resolve_fem_face_tags(
    tagged_anchors: &BTreeMap<String, TaggedAnchorBinding>,
    boundary: &AnalysisBoundarySurface,
) -> AppResult<BTreeMap<String, Vec<FemFaceTarget>>> {
    let mut resolved = BTreeMap::new();
    for (tag_name, anchor) in tagged_anchors {
        if anchor.kind != TaggedAnchorKind::Face {
            continue;
        }
        if anchor.canonical_target_ids.is_empty() || anchor.durable_target_ids.is_empty() {
            return Err(AppError::validation(format!(
                "FEM face tag '{tag_name}' lacks canonical or durable BRep target identity."
            )));
        }
        if anchor.canonical_target_ids.len() != anchor.durable_target_ids.len() {
            return Err(AppError::validation(format!(
                "FEM face tag '{tag_name}' canonical/durable target cardinality differs."
            )));
        }
        let mut targets = Vec::with_capacity(anchor.canonical_target_ids.len());
        for (canonical_target_id, durable_target_id) in anchor
            .canonical_target_ids
            .iter()
            .zip(&anchor.durable_target_ids)
        {
            let matches = boundary
                .face_groups
                .iter()
                .filter(|group| {
                    group.part_id == boundary.part_id
                        && group.canonical_target_id == *canonical_target_id
                        && group.durable_target_id.as_deref() == Some(durable_target_id.as_str())
                })
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                return Err(AppError::validation(format!(
                    "FEM face tag '{tag_name}' target '{canonical_target_id}'/'{durable_target_id}' resolved to {} analysis-boundary groups; expected exactly 1.",
                    matches.len()
                )));
            }
            let group = matches[0];
            let target = FemFaceTarget {
                schema_version: FEM_SCHEMA_VERSION,
                part_id: group.part_id.clone(),
                canonical_target_id: group.canonical_target_id.clone(),
                durable_target_id: durable_target_id.clone(),
                source_geometry_digest: boundary.source_geometry_digest.clone(),
            };
            target.validate().map_err(|error| {
                AppError::validation(format!("FEM face tag '{tag_name}' is invalid: {error}"))
            })?;
            if targets.contains(&target) {
                return Err(AppError::validation(format!(
                    "FEM face tag '{tag_name}' contains duplicate durable face targets."
                )));
            }
            targets.push(target);
        }
        resolved.insert(tag_name.clone(), targets);
    }
    Ok(resolved)
}

pub fn authored_study_from_core(
    program: &CoreProgram,
    analysis_name: &str,
    resolved_faces: &BTreeMap<String, Vec<FemFaceTarget>>,
    budgets: FemBudgetLimits,
) -> AppResult<FemAuthoredStudy> {
    budgets
        .validate()
        .map_err(|error| AppError::validation(format!("FEM budgets are invalid: {error}")))?;
    let analysis = unique_analysis(program, analysis_name)?;
    if !program.parts.iter().any(|part| part.key == analysis.part) {
        return Err(AppError::validation(format!(
            "FEM analysis '{analysis_name}' references missing part '{}'.",
            analysis.part
        )));
    }

    let mut material = None;
    let mut mesh_control = None;
    let mut constraints = Vec::new();
    let mut loads = Vec::new();
    let mut topology_controls = None;
    let mut passive_solid_regions = Vec::new();
    let mut passive_void_regions = Vec::new();
    let mut solver_method = None;
    let mut source_geometry_digest = None::<String>;

    for clause in &analysis.clauses {
        match &clause.kind {
            CoreAnalysisClauseKind::Material {
                name,
                young_modulus,
                poisson_ratio,
                density,
                yield_strength,
            } => {
                let value = FemMaterial {
                    schema_version: FEM_SCHEMA_VERSION,
                    name: name.clone(),
                    young_modulus_mpa: resolve_analysis_scalar(
                        program,
                        young_modulus,
                        FemScalarUnit::Stress,
                        "material Young's modulus",
                    )?,
                    poisson_ratio: resolve_analysis_scalar(
                        program,
                        poisson_ratio,
                        FemScalarUnit::Dimensionless,
                        "material Poisson ratio",
                    )?,
                    density_kg_per_mm3: resolve_analysis_scalar(
                        program,
                        density,
                        FemScalarUnit::Density,
                        "material density",
                    )?,
                    yield_strength_mpa: resolve_analysis_scalar(
                        program,
                        yield_strength,
                        FemScalarUnit::Stress,
                        "material yield strength",
                    )?,
                };
                value.validate().map_err(|error| {
                    AppError::validation(format!("FEM material is invalid: {error}"))
                })?;
                set_once(&mut material, value, "material")?;
            }
            CoreAnalysisClauseKind::VolumeMesh {
                element,
                size,
                local_refinements,
            } => {
                if element != "tet4" {
                    return Err(AppError::validation(format!(
                        "FEM analysis '{analysis_name}' requests unsupported element '{element}'."
                    )));
                }
                let refinements = local_refinements
                    .iter()
                    .map(|refinement| {
                        Ok(FemLocalRefinement {
                            schema_version: FEM_SCHEMA_VERSION,
                            faces: resolve_face_tag(
                                resolved_faces,
                                &refinement.face_tag,
                                &analysis.part,
                                &mut source_geometry_digest,
                            )?,
                            size_mm: resolve_analysis_scalar(
                                program,
                                &refinement.size,
                                FemScalarUnit::Length,
                                "local refinement size",
                            )?,
                        })
                    })
                    .collect::<AppResult<Vec<_>>>()?;
                let value = FemMeshControl {
                    schema_version: FEM_SCHEMA_VERSION,
                    element_kind: FemElementKind::Tet4,
                    global_size_mm: resolve_analysis_scalar(
                        program,
                        size,
                        FemScalarUnit::Length,
                        "volume mesh size",
                    )?,
                    local_refinements: refinements,
                    budgets: budgets.clone(),
                };
                value.validate().map_err(|error| {
                    AppError::validation(format!("FEM mesh control is invalid: {error}"))
                })?;
                set_once(&mut mesh_control, value, "volume mesh")?;
            }
            CoreAnalysisClauseKind::Refine { .. } => {
                return Err(AppError::validation(
                    "FEM `refine` must be nested inside `volume-mesh`.",
                ));
            }
            CoreAnalysisClauseKind::TopologyControls {
                volume_fraction,
                penalty,
                minimum_density,
                filter_radius,
                move_limit,
                convergence_tolerance,
            } => {
                let value = FemAuthoredTopologyControls {
                    volume_fraction: resolve_analysis_scalar(
                        program,
                        volume_fraction,
                        FemScalarUnit::Dimensionless,
                        "topology volume fraction",
                    )?,
                    penalty: resolve_analysis_scalar(
                        program,
                        penalty,
                        FemScalarUnit::Dimensionless,
                        "topology penalty",
                    )?,
                    minimum_density: resolve_analysis_scalar(
                        program,
                        minimum_density,
                        FemScalarUnit::Dimensionless,
                        "topology minimum density",
                    )?,
                    filter_radius_mm: resolve_analysis_scalar(
                        program,
                        filter_radius,
                        FemScalarUnit::Length,
                        "topology filter radius",
                    )?,
                    move_limit: resolve_analysis_scalar(
                        program,
                        move_limit,
                        FemScalarUnit::Dimensionless,
                        "topology move limit",
                    )?,
                    convergence_tolerance: resolve_analysis_scalar(
                        program,
                        convergence_tolerance,
                        FemScalarUnit::Dimensionless,
                        "topology convergence tolerance",
                    )?,
                };
                validate_authored_topology_controls(&value)?;
                set_once(&mut topology_controls, value, "topology controls")?;
            }
            CoreAnalysisClauseKind::PassiveSolid { face_tag, depth } => {
                passive_solid_regions.push(resolve_topology_region(
                    program,
                    resolved_faces,
                    &analysis.part,
                    face_tag,
                    depth,
                    &mut source_geometry_digest,
                    "passive-solid",
                )?);
            }
            CoreAnalysisClauseKind::PassiveVoid { face_tag, depth } => {
                passive_void_regions.push(resolve_topology_region(
                    program,
                    resolved_faces,
                    &analysis.part,
                    face_tag,
                    depth,
                    &mut source_geometry_digest,
                    "passive-void",
                )?);
            }
            CoreAnalysisClauseKind::Fixed { face_tag } => {
                let value = FemConstraint::Fixed {
                    schema_version: FEM_SCHEMA_VERSION,
                    name: format!("fixed:{face_tag}"),
                    faces: resolve_face_tag(
                        resolved_faces,
                        face_tag,
                        &analysis.part,
                        &mut source_geometry_digest,
                    )?,
                };
                validate_constraint(&value)?;
                constraints.push(value);
            }
            CoreAnalysisClauseKind::PrescribedDisplacement {
                face_tag,
                displacement,
            } => {
                let value = FemConstraint::PrescribedDisplacement {
                    schema_version: FEM_SCHEMA_VERSION,
                    name: format!("prescribed-displacement:{face_tag}"),
                    faces: resolve_face_tag(
                        resolved_faces,
                        face_tag,
                        &analysis.part,
                        &mut source_geometry_digest,
                    )?,
                    displacement_mm: FemOptionalDisplacement {
                        x_mm: resolve_optional_scalar(
                            program,
                            &displacement[0],
                            FemScalarUnit::Length,
                            "prescribed x displacement",
                        )?,
                        y_mm: resolve_optional_scalar(
                            program,
                            &displacement[1],
                            FemScalarUnit::Length,
                            "prescribed y displacement",
                        )?,
                        z_mm: resolve_optional_scalar(
                            program,
                            &displacement[2],
                            FemScalarUnit::Length,
                            "prescribed z displacement",
                        )?,
                    },
                };
                validate_constraint(&value)?;
                constraints.push(value);
            }
            CoreAnalysisClauseKind::SurfaceForce { face_tag, total } => {
                let value = FemLoad::SurfaceForce {
                    schema_version: FEM_SCHEMA_VERSION,
                    name: format!("surface-force:{face_tag}"),
                    faces: resolve_face_tag(
                        resolved_faces,
                        face_tag,
                        &analysis.part,
                        &mut source_geometry_digest,
                    )?,
                    total_force_n: FemForceVector {
                        x_n: resolve_analysis_scalar(
                            program,
                            &total[0],
                            FemScalarUnit::Force,
                            "surface force x",
                        )?,
                        y_n: resolve_analysis_scalar(
                            program,
                            &total[1],
                            FemScalarUnit::Force,
                            "surface force y",
                        )?,
                        z_n: resolve_analysis_scalar(
                            program,
                            &total[2],
                            FemScalarUnit::Force,
                            "surface force z",
                        )?,
                    },
                };
                validate_load(&value)?;
                loads.push(value);
            }
            CoreAnalysisClauseKind::Traction { face_tag, vector } => {
                let value = FemLoad::Traction {
                    schema_version: FEM_SCHEMA_VERSION,
                    name: format!("traction:{face_tag}"),
                    faces: resolve_face_tag(
                        resolved_faces,
                        face_tag,
                        &analysis.part,
                        &mut source_geometry_digest,
                    )?,
                    traction_mpa: FemStressVector {
                        x_mpa: resolve_analysis_scalar(
                            program,
                            &vector[0],
                            FemScalarUnit::Stress,
                            "traction x",
                        )?,
                        y_mpa: resolve_analysis_scalar(
                            program,
                            &vector[1],
                            FemScalarUnit::Stress,
                            "traction y",
                        )?,
                        z_mpa: resolve_analysis_scalar(
                            program,
                            &vector[2],
                            FemScalarUnit::Stress,
                            "traction z",
                        )?,
                    },
                };
                validate_load(&value)?;
                loads.push(value);
            }
            CoreAnalysisClauseKind::Pressure { face_tag, pressure } => {
                let value = FemLoad::Pressure {
                    schema_version: FEM_SCHEMA_VERSION,
                    name: format!("pressure:{face_tag}"),
                    faces: resolve_face_tag(
                        resolved_faces,
                        face_tag,
                        &analysis.part,
                        &mut source_geometry_digest,
                    )?,
                    pressure_mpa: resolve_analysis_scalar(
                        program,
                        pressure,
                        FemScalarUnit::Stress,
                        "pressure",
                    )?,
                };
                validate_load(&value)?;
                loads.push(value);
            }
            CoreAnalysisClauseKind::Solve { method } => {
                if method != "sparse-direct" {
                    return Err(AppError::validation(format!(
                        "FEM analysis '{analysis_name}' requests unsupported solver '{method}'."
                    )));
                }
                set_once(&mut solver_method, method.clone(), "solver")?;
            }
            _ => {}
        }
    }

    if constraints.is_empty() {
        return Err(AppError::validation(format!(
            "FEM analysis '{analysis_name}' needs at least one displacement constraint."
        )));
    }
    if loads.is_empty() {
        return Err(AppError::validation(format!(
            "FEM analysis '{analysis_name}' needs at least one load."
        )));
    }

    Ok(FemAuthoredStudy {
        analysis_name: analysis_name.to_string(),
        part_id: analysis.part.clone(),
        material: material.ok_or_else(|| {
            AppError::validation(format!(
                "FEM analysis '{analysis_name}' is missing material."
            ))
        })?,
        mesh_control: mesh_control.ok_or_else(|| {
            AppError::validation(format!(
                "FEM analysis '{analysis_name}' is missing volume mesh controls."
            ))
        })?,
        constraints,
        loads,
        topology_controls,
        passive_solid_regions,
        passive_void_regions,
        solver_method: solver_method.ok_or_else(|| {
            AppError::validation(format!("FEM analysis '{analysis_name}' is missing solver."))
        })?,
    })
}

fn validate_authored_topology_controls(value: &FemAuthoredTopologyControls) -> AppResult<()> {
    let open_unit = |number: f64| number.is_finite() && number > 0.0 && number < 1.0;
    if !open_unit(value.volume_fraction) {
        return Err(AppError::validation(
            "FEM topology volume fraction must be finite and between 0 and 1.",
        ));
    }
    if !value.penalty.is_finite() || value.penalty < 1.0 {
        return Err(AppError::validation(
            "FEM topology penalty must be finite and at least 1.",
        ));
    }
    if !open_unit(value.minimum_density) {
        return Err(AppError::validation(
            "FEM topology minimum density must be finite and between 0 and 1.",
        ));
    }
    if !value.filter_radius_mm.is_finite() || value.filter_radius_mm <= 0.0 {
        return Err(AppError::validation(
            "FEM topology filter radius must be finite and positive.",
        ));
    }
    if !open_unit(value.move_limit) {
        return Err(AppError::validation(
            "FEM topology move limit must be finite and between 0 and 1.",
        ));
    }
    if !value.convergence_tolerance.is_finite() || value.convergence_tolerance <= 0.0 {
        return Err(AppError::validation(
            "FEM topology convergence tolerance must be finite and positive.",
        ));
    }
    Ok(())
}

fn resolve_topology_region(
    program: &CoreProgram,
    resolved_faces: &BTreeMap<String, Vec<FemFaceTarget>>,
    part_id: &str,
    face_tag: &str,
    depth: &CoreAnalysisScalarExpr,
    source_geometry_digest: &mut Option<String>,
    label: &str,
) -> AppResult<FemAuthoredTopologyRegion> {
    let depth_mm = resolve_analysis_scalar(program, depth, FemScalarUnit::Length, label)?;
    if !depth_mm.is_finite() || depth_mm <= 0.0 {
        return Err(AppError::validation(format!(
            "FEM {label} depth must be finite and positive."
        )));
    }
    Ok(FemAuthoredTopologyRegion {
        faces: resolve_face_tag(resolved_faces, face_tag, part_id, source_geometry_digest)?,
        depth_mm,
    })
}

#[derive(Clone, Copy)]
enum FemScalarUnit {
    Dimensionless,
    Length,
    Force,
    Stress,
    Density,
}

fn resolve_analysis_scalar(
    program: &CoreProgram,
    expression: &CoreAnalysisScalarExpr,
    expected: FemScalarUnit,
    label: &str,
) -> AppResult<f64> {
    let (value, unit) = match expression {
        CoreAnalysisScalarExpr::Literal { value, unit } => (*value, unit.as_str()),
        CoreAnalysisScalarExpr::Parameter { key, scale } => {
            let parameter = program
                .parameters
                .iter()
                .find(|parameter| parameter.key == *key)
                .ok_or_else(|| {
                    AppError::validation(format!(
                        "FEM {label} references unknown parameter '{key}'."
                    ))
                })?;
            let CoreParameterValue::Number(value) = parameter.default_value else {
                return Err(AppError::validation(format!(
                    "FEM {label} parameter '{key}' is not numeric."
                )));
            };
            let unit = parameter.constraints.unit.as_deref().unwrap_or("");
            (value * scale, unit)
        }
    };
    if !value.is_finite() {
        return Err(AppError::validation(format!("FEM {label} must be finite.")));
    }
    normalize_fem_scalar(value, unit, expected).ok_or_else(|| {
        AppError::validation(format!(
            "FEM {label} has unit '{}'; expected {}.",
            if unit.is_empty() {
                "dimensionless"
            } else {
                unit
            },
            expected_unit_name(expected)
        ))
    })
}

fn resolve_optional_scalar(
    program: &CoreProgram,
    expression: &Option<CoreAnalysisScalarExpr>,
    expected: FemScalarUnit,
    label: &str,
) -> AppResult<Option<f64>> {
    expression
        .as_ref()
        .map(|value| resolve_analysis_scalar(program, value, expected, label))
        .transpose()
}

fn normalize_fem_scalar(value: f64, unit: &str, expected: FemScalarUnit) -> Option<f64> {
    match expected {
        FemScalarUnit::Dimensionless if unit.is_empty() => Some(value),
        FemScalarUnit::Length if matches!(unit, "mm" | "length") => Some(value),
        FemScalarUnit::Force if unit == "N" => Some(value),
        FemScalarUnit::Stress if unit == "MPa" => Some(value),
        FemScalarUnit::Density if unit == "kg-per-mm3" => Some(value),
        FemScalarUnit::Density if unit == "kg-per-m3" => Some(value * 1.0e-9),
        _ => None,
    }
}

fn expected_unit_name(unit: FemScalarUnit) -> &'static str {
    match unit {
        FemScalarUnit::Dimensionless => "dimensionless",
        FemScalarUnit::Length => "mm",
        FemScalarUnit::Force => "N",
        FemScalarUnit::Stress => "MPa",
        FemScalarUnit::Density => "kg/mm^3",
    }
}

fn resolve_face_tag(
    resolved_faces: &BTreeMap<String, Vec<FemFaceTarget>>,
    face_tag: &str,
    part_id: &str,
    source_geometry_digest: &mut Option<String>,
) -> AppResult<Vec<FemFaceTarget>> {
    let faces = resolved_faces.get(face_tag).ok_or_else(|| {
        AppError::validation(format!(
            "FEM face tag '{face_tag}' did not resolve to durable BRep faces."
        ))
    })?;
    if faces.is_empty() {
        return Err(AppError::validation(format!(
            "FEM face tag '{face_tag}' resolved to zero faces."
        )));
    }
    for face in faces {
        face.validate().map_err(|error| {
            AppError::validation(format!("FEM face tag '{face_tag}' is invalid: {error}"))
        })?;
        if face.part_id != part_id {
            return Err(AppError::validation(format!(
                "FEM face tag '{face_tag}' resolved across part boundary: expected '{part_id}', got '{}'.",
                face.part_id
            )));
        }
        match source_geometry_digest {
            Some(expected) if expected != &face.source_geometry_digest => {
                return Err(AppError::conflict(format!(
                    "FEM face tag '{face_tag}' resolved against stale source geometry."
                )))
            }
            None => *source_geometry_digest = Some(face.source_geometry_digest.clone()),
            _ => {}
        }
    }
    Ok(faces.clone())
}

fn validate_constraint(value: &FemConstraint) -> AppResult<()> {
    value.validate().map_err(|error| {
        AppError::validation(format!("FEM displacement constraint is invalid: {error}"))
    })
}

fn validate_load(value: &FemLoad) -> AppResult<()> {
    value
        .validate()
        .map_err(|error| AppError::validation(format!("FEM load is invalid: {error}")))
}

pub fn engineering_ledger_from_core(
    program: &CoreProgram,
    analysis_name: &str,
    source_geometry_digest: &str,
    analysis_geometry_digest: &str,
) -> AppResult<FemEngineeringEvidenceLedger> {
    let analysis = unique_analysis(program, analysis_name)?;
    let mut question = None;
    let mut idealization = None;
    let mut acceptance_criteria = Vec::new();
    let mut evidence = Vec::new();
    let mut input_bindings = Vec::new();
    let mut assumptions = Vec::new();
    let mut validation_evidence = Vec::new();

    for clause in &analysis.clauses {
        match &clause.kind {
            CoreAnalysisClauseKind::Question {
                question_id,
                statement,
                decision,
                acceptance_metric_ids,
            } => set_once(
                &mut question,
                FemEngineeringQuestion {
                    question_id: question_id.clone(),
                    statement: statement.clone(),
                    decision: decision.clone(),
                    acceptance_metric_ids: acceptance_metric_ids.clone(),
                },
                "engineering question",
            )?,
            CoreAnalysisClauseKind::AcceptanceCriterion {
                metric_id,
                field,
                comparison,
                limit,
                unit,
                requires_convergence,
            } => acceptance_criteria.push(FemAcceptanceCriterion {
                metric_id: metric_id.clone(),
                field: field.clone(),
                comparison: parse_comparison(comparison)?,
                limit: limit.parse::<f64>().map_err(|_| {
                    AppError::validation(format!(
                        "FEM acceptance criterion '{metric_id}' limit '{limit}' is not numeric."
                    ))
                })?,
                unit: unit.clone(),
                requires_convergence: *requires_convergence,
            }),
            CoreAnalysisClauseKind::Idealization {
                kind,
                justification,
                accepted_by_user,
            } => {
                if kind != "exact-solid" {
                    return Err(AppError::validation(format!(
                        "FEM idealization '{kind}' is not supported yet; no silent defeaturing is allowed."
                    )));
                }
                if source_geometry_digest != analysis_geometry_digest {
                    return Err(AppError::conflict(
                        "Exact-solid FEM idealization requires identical source and analysis geometry digests.",
                    ));
                }
                set_once(
                    &mut idealization,
                    FemIdealizationRecord {
                        source_geometry_digest: source_geometry_digest.to_string(),
                        analysis_geometry_digest: analysis_geometry_digest.to_string(),
                        affected_topology_ids: Vec::new(),
                        justification: justification.clone(),
                        expected_influence_percent: 0.0,
                        accepted_by_user: *accepted_by_user,
                    },
                    "analysis idealization",
                )?;
            }
            CoreAnalysisClauseKind::Evidence {
                evidence_id,
                subject,
                source,
                authority,
                uncertainty_percent,
                decision_critical,
            } => evidence.push(FemEvidenceRecord {
                evidence_id: evidence_id.clone(),
                subject: parse_subject(subject)?,
                label: evidence_id.clone(),
                source: source.clone(),
                authority: parse_authority(authority)?,
                uncertainty_percent: Some(*uncertainty_percent),
                decision_critical: *decision_critical,
            }),
            CoreAnalysisClauseKind::InputEvidence {
                input_name,
                evidence_id,
            } => input_bindings.push(FemInputEvidenceBinding {
                input_name: input_name.clone(),
                evidence_id: evidence_id.clone(),
            }),
            CoreAnalysisClauseKind::Assumption {
                assumption_id,
                category,
                statement,
                status,
                evidence_ids,
            } => assumptions.push(FemStudyAssumption {
                assumption_id: assumption_id.clone(),
                category: parse_assumption_category(category)?,
                statement: statement.clone(),
                status: parse_assumption_status(status)?,
                evidence_ids: evidence_ids.clone(),
            }),
            CoreAnalysisClauseKind::ValidationEvidence {
                validation_id,
                kind,
                source,
                result_digest,
            } => validation_evidence.push(FemValidationEvidence {
                validation_id: validation_id.clone(),
                kind: parse_validation_kind(kind)?,
                source: source.clone(),
                result_digest: result_digest.clone(),
            }),
            _ => {}
        }
    }

    let ledger = FemEngineeringEvidenceLedger {
        schema_version: FEM_SCHEMA_VERSION,
        question: question.ok_or_else(|| {
            AppError::validation(format!(
                "FEM analysis '{analysis_name}' is missing an engineering question."
            ))
        })?,
        acceptance_criteria,
        idealization: idealization.ok_or_else(|| {
            AppError::validation(format!(
                "FEM analysis '{analysis_name}' is missing an explicit idealization."
            ))
        })?,
        evidence,
        input_bindings,
        assumptions,
        applicability_checks: pending_applicability_checks(),
        sensitivity: None,
        validation_evidence,
    };
    ledger.validate().map_err(|error| {
        AppError::validation(format!("FEM engineering evidence is invalid: {error}"))
    })?;
    Ok(ledger)
}

fn unique_analysis<'a>(
    program: &'a CoreProgram,
    analysis_name: &str,
) -> AppResult<&'a CoreAnalysisDecl> {
    let matches = program
        .analyses
        .iter()
        .filter(|analysis| analysis.name == analysis_name)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Err(AppError::not_found(format!(
            "FEM analysis '{analysis_name}' was not found."
        ))),
        [analysis] => Ok(*analysis),
        _ => Err(AppError::validation(format!(
            "FEM analysis name '{analysis_name}' is duplicate ({} declarations).",
            matches.len()
        ))),
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T, label: &str) -> AppResult<()> {
    if slot.replace(value).is_some() {
        Err(AppError::validation(format!(
            "FEM analysis contains duplicate {label}."
        )))
    } else {
        Ok(())
    }
}

fn pending_applicability_checks() -> Vec<FemApplicabilityCheck> {
    [
        ("one-solid-scope", FemApplicabilityCheckKind::OneSolidScope),
        (
            "interfaces",
            FemApplicabilityCheckKind::UnsupportedInterfaces,
        ),
        (
            "tet4-slenderness",
            FemApplicabilityCheckKind::ThinSlenderTet4Risk,
        ),
        (
            "locking",
            FemApplicabilityCheckKind::NearIncompressibleLocking,
        ),
        ("constraints", FemApplicabilityCheckKind::ConstraintRealism),
        (
            "singularity",
            FemApplicabilityCheckKind::ConcentratedLoadSingularity,
        ),
    ]
    .into_iter()
    .map(|(check_id, kind)| FemApplicabilityCheck {
        check_id: check_id.to_string(),
        kind,
        status: FemApplicabilityStatus::NotEvaluated,
        observed: None,
        limit: None,
        unit: None,
        evidence_ids: Vec::new(),
        detail: "Runtime applicability audit has not run.".to_string(),
    })
    .collect()
}

fn parse_comparison(value: &str) -> AppResult<FemAcceptanceComparison> {
    match value {
        "less-than-or-equal" => Ok(FemAcceptanceComparison::LessThanOrEqual),
        "greater-than-or-equal" => Ok(FemAcceptanceComparison::GreaterThanOrEqual),
        _ => Err(AppError::validation(format!(
            "Unsupported FEM acceptance comparison '{value}'."
        ))),
    }
}

fn parse_subject(value: &str) -> AppResult<FemEvidenceSubject> {
    match value {
        "material" => Ok(FemEvidenceSubject::Material),
        "load" => Ok(FemEvidenceSubject::Load),
        "support" => Ok(FemEvidenceSubject::Support),
        "connection" => Ok(FemEvidenceSubject::Connection),
        "geometry" => Ok(FemEvidenceSubject::Geometry),
        "acceptance-criterion" => Ok(FemEvidenceSubject::AcceptanceCriterion),
        _ => Err(AppError::validation(format!(
            "Unsupported FEM evidence subject '{value}'."
        ))),
    }
}

fn parse_authority(value: &str) -> AppResult<FemEvidenceAuthority> {
    match value {
        "unknown" => Ok(FemEvidenceAuthority::Unknown),
        "proposed" => Ok(FemEvidenceAuthority::Proposed),
        "user-accepted" => Ok(FemEvidenceAuthority::UserAccepted),
        "recorded-source" => Ok(FemEvidenceAuthority::RecordedSource),
        _ => Err(AppError::validation(format!(
            "Unsupported FEM evidence authority '{value}'."
        ))),
    }
}

fn parse_assumption_category(value: &str) -> AppResult<FemStudyAssumptionCategory> {
    match value {
        "geometry" => Ok(FemStudyAssumptionCategory::Geometry),
        "physics" => Ok(FemStudyAssumptionCategory::Physics),
        "material" => Ok(FemStudyAssumptionCategory::Material),
        "load" => Ok(FemStudyAssumptionCategory::Load),
        "support" => Ok(FemStudyAssumptionCategory::Support),
        "connection" => Ok(FemStudyAssumptionCategory::Connection),
        _ => Err(AppError::validation(format!(
            "Unsupported FEM assumption category '{value}'."
        ))),
    }
}

fn parse_assumption_status(value: &str) -> AppResult<FemStudyAssumptionStatus> {
    match value {
        "unknown" => Ok(FemStudyAssumptionStatus::Unknown),
        "proposed" => Ok(FemStudyAssumptionStatus::Proposed),
        "accepted" => Ok(FemStudyAssumptionStatus::Accepted),
        "rejected" => Ok(FemStudyAssumptionStatus::Rejected),
        _ => Err(AppError::validation(format!(
            "Unsupported FEM assumption status '{value}'."
        ))),
    }
}

fn parse_validation_kind(value: &str) -> AppResult<FemValidationEvidenceKind> {
    match value {
        "analytical" => Ok(FemValidationEvidenceKind::Analytical),
        "differential-solver" => Ok(FemValidationEvidenceKind::DifferentialSolver),
        "qualified-reference" => Ok(FemValidationEvidenceKind::QualifiedReference),
        "physical-test" => Ok(FemValidationEvidenceKind::PhysicalTest),
        _ => Err(AppError::validation(format!(
            "Unsupported FEM validation kind '{value}'."
        ))),
    }
}
