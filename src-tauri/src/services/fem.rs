use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use ecky_fem::{
    audit_post_solve_applicability, audit_pre_solve_applicability, CanonicalDigest,
    FemAcceptanceComparison, FemAnalysisIdentity, FemApplicabilityCheck, FemBudgetLimits,
    FemConstraint, FemEngineeringEvidenceLedger, FemLinearStaticSolution, FemLoad,
    FemPostSolveApplicabilityInput, FemPreSolveApplicabilityInput, FemResultExtremum,
    FemSafetyFactor, FemSolveStage, FemVolumeMesh, FEM_SCHEMA_VERSION,
};
use ecky_render::core_ir::{CoreProgram, CoreVerifyClause, CoreVerifyValue};

use crate::contracts::{AppError, AppResult, TaggedAnchorBinding};
use crate::ecky_cad_host::analysis_boundary::AnalysisBoundarySurface;
use crate::fem_engineering::{
    authored_study_from_core, engineering_ledger_from_core, resolve_fem_face_tags, FemAuthoredStudy,
};
use crate::gmsh_mesher::{run_exact_brep_mesher, ExactBrepMesherRuntime, GmshBrepMeshRequest};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FemPipelineStage {
    Resolve,
    BoundaryMesh,
    VolumeMesh,
    ValidateMesh,
    Assemble,
    ApplyConstraints,
    Solve,
    Postprocess,
    Verify,
    Publish,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemProgressEvent {
    pub stage: FemPipelineStage,
    pub elapsed_ms: u64,
    pub node_count: Option<u64>,
    pub tet4_cell_count: Option<u64>,
    pub detail: String,
    pub cancellation_boundary: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemPipelineControl {
    pub envelope_mm: f64,
    pub minimum_scaled_jacobian: f64,
    pub maximum_runtime_ms: u64,
    pub relative_solver_tolerance: f64,
    pub thread_count: u32,
}

impl FemPipelineControl {
    pub fn validate(&self) -> AppResult<()> {
        for (field, value) in [
            ("envelopeMm", self.envelope_mm),
            ("minimumScaledJacobian", self.minimum_scaled_jacobian),
            ("relativeSolverTolerance", self.relative_solver_tolerance),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(AppError::validation(format!(
                    "FEM pipeline {field} must be finite and positive."
                )));
            }
        }
        if self.minimum_scaled_jacobian > 1.0 || self.maximum_runtime_ms == 0 {
            return Err(AppError::validation(
                "FEM pipeline quality threshold must not exceed 1 and timeout must be positive.",
            ));
        }
        if self.thread_count == 0 || self.thread_count > 64 {
            return Err(AppError::validation(
                "FEM pipeline thread count must be between 1 and 64.",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemPipelineResult {
    pub schema_version: u32,
    pub analysis_identity: FemAnalysisIdentity,
    pub engineering_evidence: FemEngineeringEvidenceLedger,
    pub pre_solve_applicability: Vec<FemApplicabilityCheck>,
    pub post_solve_applicability: Vec<FemApplicabilityCheck>,
    pub decision_readiness_error: Option<String>,
    pub acceptance_evaluations: Vec<FemAcceptanceEvaluation>,
    pub mesh: FemVolumeMesh,
    pub solution: FemLinearStaticSolution,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemAcceptanceEvaluation {
    pub study_name: String,
    pub metric_id: String,
    pub field: String,
    pub status: String,
    pub observed: Option<f64>,
    pub unit: String,
    pub threshold: f64,
    pub comparison: String,
    pub mesh_size_mm: f64,
    pub node_id: Option<u32>,
    pub element_id: Option<u32>,
    pub coordinate_mm: Option<[f64; 3]>,
    pub analysis_identity_digest: String,
    pub mesh_content_digest: String,
    pub result_digest: String,
    pub convergence_status: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FemMeshPipelineResult {
    pub analysis_identity: FemAnalysisIdentity,
    pub engineering_evidence: FemEngineeringEvidenceLedger,
    pub pre_solve_applicability: Vec<FemApplicabilityCheck>,
    pub study: FemAuthoredStudy,
    pub mesh: FemVolumeMesh,
}

#[allow(clippy::too_many_arguments)]
pub fn execute_fem_mesh_pipeline<F>(
    program: &CoreProgram,
    analysis_name: &str,
    tagged_anchors: &BTreeMap<String, TaggedAnchorBinding>,
    boundary: &AnalysisBoundarySurface,
    step_path: &Path,
    budgets: FemBudgetLimits,
    runtime: &ExactBrepMesherRuntime,
    scratch_dir: impl AsRef<Path>,
    control: &FemPipelineControl,
    mesh_size_override_mm: Option<f64>,
    cancelled: &AtomicBool,
    mut progress: F,
) -> AppResult<FemMeshPipelineResult>
where
    F: FnMut(FemProgressEvent),
{
    execute_fem_mesh_pipeline_started(
        program,
        analysis_name,
        tagged_anchors,
        boundary,
        step_path,
        budgets,
        runtime,
        scratch_dir.as_ref(),
        control,
        mesh_size_override_mm,
        cancelled,
        Instant::now(),
        &mut progress,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn execute_fem_pipeline<F>(
    program: &CoreProgram,
    analysis_name: &str,
    tagged_anchors: &BTreeMap<String, TaggedAnchorBinding>,
    boundary: &AnalysisBoundarySurface,
    step_path: &Path,
    budgets: FemBudgetLimits,
    runtime: &ExactBrepMesherRuntime,
    scratch_dir: impl AsRef<Path>,
    control: &FemPipelineControl,
    cancelled: &AtomicBool,
    progress: F,
) -> AppResult<FemPipelineResult>
where
    F: FnMut(FemProgressEvent),
{
    execute_fem_pipeline_with_mesh_size(
        program,
        analysis_name,
        tagged_anchors,
        boundary,
        step_path,
        budgets,
        runtime,
        scratch_dir,
        control,
        None,
        cancelled,
        progress,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn execute_fem_pipeline_with_mesh_size<F>(
    program: &CoreProgram,
    analysis_name: &str,
    tagged_anchors: &BTreeMap<String, TaggedAnchorBinding>,
    boundary: &AnalysisBoundarySurface,
    step_path: &Path,
    budgets: FemBudgetLimits,
    runtime: &ExactBrepMesherRuntime,
    scratch_dir: impl AsRef<Path>,
    control: &FemPipelineControl,
    mesh_size_override_mm: Option<f64>,
    cancelled: &AtomicBool,
    mut progress: F,
) -> AppResult<FemPipelineResult>
where
    F: FnMut(FemProgressEvent),
{
    control.validate()?;
    let scratch_dir = scratch_dir.as_ref();
    let started = Instant::now();
    let meshing = execute_fem_mesh_pipeline_started(
        program,
        analysis_name,
        tagged_anchors,
        boundary,
        step_path,
        budgets,
        runtime,
        scratch_dir,
        control,
        mesh_size_override_mm,
        cancelled,
        started,
        &mut progress,
    )?;
    let FemMeshPipelineResult {
        analysis_identity,
        engineering_evidence: authored_ledger,
        pre_solve_applicability,
        study,
        mesh,
    } = meshing;

    let maximum_dimension = usize::try_from(study.mesh_control.budgets.dofs).map_err(|_| {
        AppError::validation("FEM solver DOF budget exceeds this platform's address space.")
    })?;
    let mut last_solve_stage = None;
    let mut observe_solve = |stage| {
        let (pipeline_stage, detail) = match stage {
            FemSolveStage::Assemble => (
                FemPipelineStage::Assemble,
                "Assembling sparse Tet4 stiffness.",
            ),
            FemSolveStage::ApplyConstraints => (
                FemPipelineStage::ApplyConstraints,
                "Applying exact surface loads and Dirichlet elimination.",
            ),
            FemSolveStage::Solve => (
                FemPipelineStage::Solve,
                "Factoring and solving sparse reduced system.",
            ),
            FemSolveStage::Postprocess => (
                FemPipelineStage::Postprocess,
                "Computing unaveraged element and display fields.",
            ),
            FemSolveStage::Verify => (
                FemPipelineStage::Verify,
                "Checking residual, equilibrium, energy, and finite results.",
            ),
        };
        if last_solve_stage != Some(stage) {
            emit_progress(
                &mut progress,
                started,
                pipeline_stage,
                Some(mesh.nodes.len() as u64),
                Some(mesh.cells.len() as u64),
                detail,
                true,
            );
            last_solve_stage = Some(stage);
        }
        if cancelled.load(Ordering::Acquire) {
            return Err(ecky_fem::FemValidationError {
                field: "cancelled".to_string(),
                message: format!("FEM study '{analysis_name}' was cancelled."),
            });
        }
        Ok(())
    };
    let remaining_runtime_ms = control
        .maximum_runtime_ms
        .saturating_sub(started.elapsed().as_millis() as u64)
        .max(1);
    let solution = if let Some(worker) =
        crate::services::fem_solver_worker::KillableFaerWorkerSolver::for_current_app(
            scratch_dir,
            cancelled,
            remaining_runtime_ms,
            control.thread_count as usize,
        ) {
        ecky_fem::solve_linear_static_with_solver_and_observer(
            &mesh,
            &study.material,
            &study.loads,
            &study.constraints,
            control.relative_solver_tolerance,
            maximum_dimension,
            &worker,
            &mut observe_solve,
        )
    } else {
        ecky_fem::solve_linear_static_with_observer(
            &mesh,
            &study.material,
            &study.loads,
            &study.constraints,
            control.relative_solver_tolerance,
            maximum_dimension,
            &mut observe_solve,
        )
    }
    .map_err(|error| AppError::validation(format!("FEM linear-static solve failed: {error}")))?;

    let post_solve_applicability =
        audit_post_solve_applicability(&FemPostSolveApplicabilityInput {
            schema_version: FEM_SCHEMA_VERSION,
            characteristic_size_mm: characteristic_size(boundary)?,
            maximum_displacement_mm: solution.postprocess.summary.maximum_displacement.value,
            maximum_von_mises_mpa: solution.postprocess.summary.maximum_von_mises.value,
            yield_strength_mpa: study.material.yield_strength_mpa,
            hotspot_movement_mm: 0.0,
            boundary_condition_singularity: false,
        })
        .map_err(|error| {
            AppError::validation(format!("FEM post-solve applicability failed: {error}"))
        })?;
    let mut engineering_evidence = authored_ledger;
    engineering_evidence.applicability_checks = pre_solve_applicability
        .iter()
        .chain(&post_solve_applicability)
        .cloned()
        .collect();
    engineering_evidence.validate().map_err(|error| {
        AppError::validation(format!(
            "FEM runtime engineering evidence is invalid: {error}"
        ))
    })?;
    let mut acceptance_evaluations = evaluate_acceptance_criteria(
        &engineering_evidence,
        &solution,
        study.mesh_control.global_size_mm,
        &analysis_identity.study_name,
        &analysis_identity.canonical_digest(),
        &mesh.content_digest,
    );
    acceptance_evaluations.extend(evaluate_authored_fem_verify_clauses(
        &program.constraints.verify_clauses,
        &solution,
        study.mesh_control.global_size_mm,
        &analysis_identity.study_name,
        &analysis_identity.canonical_digest(),
        &mesh.content_digest,
    ));
    let decision_readiness_error = acceptance_evaluations
        .iter()
        .find(|evaluation| evaluation.status != "passed")
        .map(|evaluation| evaluation.detail.clone())
        .or_else(|| {
            engineering_evidence
                .validate_decision_readiness()
                .err()
                .map(|error| error.to_string())
        });
    Ok(FemPipelineResult {
        schema_version: FEM_SCHEMA_VERSION,
        analysis_identity,
        engineering_evidence,
        pre_solve_applicability,
        post_solve_applicability,
        decision_readiness_error,
        acceptance_evaluations,
        mesh,
        solution,
    })
}

fn evaluate_acceptance_criteria(
    ledger: &FemEngineeringEvidenceLedger,
    solution: &FemLinearStaticSolution,
    mesh_size_mm: f64,
    study_name: &str,
    analysis_identity_digest: &str,
    mesh_content_digest: &str,
) -> Vec<FemAcceptanceEvaluation> {
    ledger
        .acceptance_criteria
        .iter()
        .map(|criterion| {
            let (observed, expected_unit, extremum): (Option<f64>, &str, Option<&FemResultExtremum>) =
                match criterion.field.as_str() {
                    "von-mises-stress" | "vonMisesStress" => (
                        Some(solution.postprocess.summary.maximum_von_mises.value),
                        "MPa",
                        Some(&solution.postprocess.summary.maximum_von_mises),
                    ),
                    "maximum-displacement" | "maximumDisplacement" => (
                        Some(solution.postprocess.summary.maximum_displacement.value),
                        "mm",
                        Some(&solution.postprocess.summary.maximum_displacement),
                    ),
                    "maximum-principal-stress" | "maximumPrincipalStress" => (
                        Some(solution.postprocess.summary.maximum_principal_stress.value),
                        "MPa",
                        Some(&solution.postprocess.summary.maximum_principal_stress),
                    ),
                    "mass" => (Some(solution.postprocess.summary.mass_kg), "kg", None),
                    "reaction-resultant" | "reactionResultant" => (
                        solution
                            .support_reactions
                            .iter()
                            .map(|reaction| reaction.resultant_n.iter().map(|value| value * value).sum::<f64>().sqrt())
                            .reduce(f64::max),
                        "N",
                        None,
                    ),
                    "minimum-yield-safety-factor" | "minimumYieldSafetyFactor" => (
                        match solution.postprocess.summary.minimum_yield_safety_factor {
                            FemSafetyFactor::Finite { value } => Some(value),
                            FemSafetyFactor::Infinite => None,
                        },
                        "dimensionless",
                        None,
                    ),
                    _ => (None, "unsupported", None),
                };
            let comparison = match criterion.comparison {
                FemAcceptanceComparison::LessThanOrEqual => "lessThanOrEqual",
                FemAcceptanceComparison::GreaterThanOrEqual => "greaterThanOrEqual",
            };
            let mut status = "passed";
            let detail;
            if expected_unit == "unsupported" {
                status = "failed";
                detail = format!(
                    "FEM acceptance metric '{}' uses unsupported field '{}'.",
                    criterion.metric_id, criterion.field
                );
            } else if criterion.unit != expected_unit {
                status = "failed";
                detail = format!(
                    "FEM acceptance metric '{}' unit '{}' does not match field '{}' unit '{}'.",
                    criterion.metric_id, criterion.unit, criterion.field, expected_unit
                );
            } else if criterion.requires_convergence {
                status = "pending";
                detail = format!(
                    "FEM acceptance metric '{}' requires current convergence evidence; single-run result cannot pass it.",
                    criterion.metric_id
                );
            } else if let Some(value) = observed {
                let passed = match criterion.comparison {
                    FemAcceptanceComparison::LessThanOrEqual => value <= criterion.limit,
                    FemAcceptanceComparison::GreaterThanOrEqual => value >= criterion.limit,
                };
                if passed {
                    detail = format!("FEM acceptance metric '{}' passed.", criterion.metric_id);
                } else {
                    status = "failed";
                    detail = format!(
                        "FEM acceptance metric '{}' failed: observed {} {}, threshold {} {}.",
                        criterion.metric_id, value, expected_unit, criterion.limit, expected_unit
                    );
                }
            } else if matches!(
                solution.postprocess.summary.minimum_yield_safety_factor,
                FemSafetyFactor::Infinite
            ) && matches!(criterion.comparison, FemAcceptanceComparison::GreaterThanOrEqual)
            {
                detail = format!(
                    "FEM acceptance metric '{}' passed with typed infinite safety factor.",
                    criterion.metric_id
                );
            } else {
                status = "failed";
                detail = format!(
                    "FEM acceptance metric '{}' has no current numeric result.", criterion.metric_id
                );
            }
            FemAcceptanceEvaluation {
                study_name: study_name.to_string(),
                metric_id: criterion.metric_id.clone(),
                field: criterion.field.clone(),
                status: status.to_string(),
                observed,
                unit: expected_unit.to_string(),
                threshold: criterion.limit,
                comparison: comparison.to_string(),
                mesh_size_mm,
                node_id: extremum.and_then(|value| value.node_id),
                element_id: extremum.and_then(|value| value.element_id),
                coordinate_mm: extremum.map(|value| [
                    value.coordinate_mm.x_mm,
                    value.coordinate_mm.y_mm,
                    value.coordinate_mm.z_mm,
                ]),
                analysis_identity_digest: analysis_identity_digest.to_string(),
                mesh_content_digest: mesh_content_digest.to_string(),
                result_digest: solution.postprocess.result_digest.clone(),
                convergence_status: None,
                detail,
            }
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
struct FemMetricObservation {
    field: String,
    value: Option<f64>,
    infinite: bool,
    unit: String,
    node_id: Option<u32>,
    element_id: Option<u32>,
    coordinate_mm: Option<[f64; 3]>,
}

fn evaluate_authored_fem_verify_clauses(
    clauses: &[CoreVerifyClause],
    solution: &FemLinearStaticSolution,
    mesh_size_mm: f64,
    study_name: &str,
    analysis_identity_digest: &str,
    mesh_content_digest: &str,
) -> Vec<FemAcceptanceEvaluation> {
    evaluate_authored_fem_verify_clauses_with(
        clauses,
        mesh_size_mm,
        study_name,
        analysis_identity_digest,
        mesh_content_digest,
        solution.postprocess.result_digest.as_str(),
        |aggregate, field, support| resolve_solution_metric(solution, aggregate, field, support),
    )
}

#[allow(clippy::too_many_arguments)]
fn evaluate_authored_fem_verify_clauses_with<F>(
    clauses: &[CoreVerifyClause],
    mesh_size_mm: f64,
    study_name: &str,
    analysis_identity_digest: &str,
    mesh_content_digest: &str,
    result_digest: &str,
    mut resolve: F,
) -> Vec<FemAcceptanceEvaluation>
where
    F: FnMut(&str, &str, Option<&str>) -> Result<FemMetricObservation, String>,
{
    clauses
        .iter()
        .filter_map(|clause| {
            let alias = clause.metric.items.first().and_then(fem_verify_symbol)?;
            let CoreVerifyValue::List(metric) = clause.metric.items.get(1)? else {
                return None;
            };
            let aggregate = metric.first().and_then(fem_verify_symbol)?;
            if !matches!(aggregate, "fem-max" | "fem-min") {
                return None;
            }
            let metric_id = clause
                .tag
                .items
                .first()
                .and_then(fem_verify_symbol)
                .unwrap_or(alias)
                .to_string();
            let fail = |field: String, unit: String, threshold: f64, comparison: String, detail: String| {
                FemAcceptanceEvaluation {
                    study_name: study_name.to_string(),
                    metric_id: metric_id.clone(),
                    field,
                    status: "failed".into(),
                    observed: None,
                    unit,
                    threshold,
                    comparison,
                    mesh_size_mm,
                    node_id: None,
                    element_id: None,
                    coordinate_mm: None,
                    analysis_identity_digest: analysis_identity_digest.to_string(),
                    mesh_content_digest: mesh_content_digest.to_string(),
                    result_digest: result_digest.to_string(),
                    convergence_status: None,
                    detail,
                }
            };
            let Some(metric_study) = metric.get(1).and_then(fem_verify_symbol) else {
                return Some(fail(String::new(), String::new(), 0.0, String::new(), format!(
                    "FEM verify metric '{metric_id}' requires a named study."
                )));
            };
            let Some(field) = metric.get(2).and_then(fem_verify_symbol) else {
                return Some(fail(String::new(), String::new(), 0.0, String::new(), format!(
                    "FEM verify metric '{metric_id}' requires a result field."
                )));
            };
            if metric_study != study_name {
                return Some(fail(field.into(), String::new(), 0.0, String::new(), format!(
                    "FEM verify metric '{metric_id}' requested study '{metric_study}', but current result belongs to '{study_name}'; stale or cross-study results cannot pass."
                )));
            }
            let support = metric.get(3).and_then(fem_verify_symbol);
            let observation = match resolve(aggregate, field, support) {
                Ok(value) => value,
                Err(detail) => {
                    return Some(fail(field.into(), String::new(), 0.0, String::new(), detail));
                }
            };
            let Some(expect_alias) = clause.expect.items.first().and_then(fem_verify_symbol) else {
                return Some(fail(observation.field, observation.unit, 0.0, String::new(), format!(
                    "FEM verify metric '{metric_id}' requires an expectation alias."
                )));
            };
            if expect_alias != alias {
                return Some(fail(observation.field, observation.unit, 0.0, String::new(), format!(
                    "FEM verify metric '{metric_id}' expectation alias '{expect_alias}' does not match '{alias}'."
                )));
            }
            let expected = clause.expect.items.get(1).and_then(parse_fem_threshold);
            let Some((comparison, threshold, threshold_unit)) = expected else {
                return Some(fail(observation.field, observation.unit, 0.0, String::new(), format!(
                    "FEM verify metric '{metric_id}' requires `(<|<=|>|>= (quantity value unit))`."
                )));
            };
            if threshold_unit != observation.unit {
                return Some(fail(observation.field, observation.unit.clone(), threshold, comparison.into(), format!(
                    "FEM verify metric '{metric_id}' threshold unit '{threshold_unit}' does not match result unit '{}'.",
                    observation.unit
                )));
            }
            let passed = if observation.infinite {
                matches!(comparison, ">" | ">=")
            } else if let Some(value) = observation.value {
                match comparison {
                    "<" => value < threshold,
                    "<=" => value <= threshold,
                    ">" => value > threshold,
                    ">=" => value >= threshold,
                    _ => false,
                }
            } else {
                false
            };
            let observed = observation.value;
            Some(FemAcceptanceEvaluation {
                study_name: study_name.to_string(),
                metric_id: metric_id.clone(),
                field: observation.field,
                status: if passed { "passed" } else { "failed" }.into(),
                observed,
                unit: observation.unit.clone(),
                threshold,
                comparison: match comparison {
                    "<" | "<=" => "lessThanOrEqual",
                    ">" | ">=" => "greaterThanOrEqual",
                    _ => unreachable!(),
                }
                .into(),
                mesh_size_mm,
                node_id: observation.node_id,
                element_id: observation.element_id,
                coordinate_mm: observation.coordinate_mm,
                analysis_identity_digest: analysis_identity_digest.to_string(),
                mesh_content_digest: mesh_content_digest.to_string(),
                result_digest: result_digest.to_string(),
                convergence_status: None,
                detail: format!(
                    "FEM verify metric '{metric_id}' {}: {} {} {} {} {}.",
                    if passed { "passed" } else { "failed" },
                    observed.map_or_else(|| "infinite".into(), |value| value.to_string()),
                    observation.unit,
                    comparison,
                    threshold,
                    threshold_unit
                ),
            })
        })
        .collect()
}

fn fem_verify_symbol(value: &CoreVerifyValue) -> Option<&str> {
    match value {
        CoreVerifyValue::Symbol(value) | CoreVerifyValue::Text(value) => Some(value),
        _ => None,
    }
}

fn parse_fem_threshold(value: &CoreVerifyValue) -> Option<(&str, f64, &str)> {
    let CoreVerifyValue::List(items) = value else {
        return None;
    };
    let [operator, quantity] = items.as_slice() else {
        return None;
    };
    let operator = fem_verify_symbol(operator)?;
    if !matches!(operator, "<" | "<=" | ">" | ">=") {
        return None;
    }
    let CoreVerifyValue::List(quantity) = quantity else {
        return None;
    };
    let [kind, CoreVerifyValue::Number(number), unit] = quantity.as_slice() else {
        return None;
    };
    (fem_verify_symbol(kind)? == "quantity" && number.is_finite()).then_some((
        operator,
        *number,
        fem_verify_symbol(unit)?,
    ))
}

fn resolve_solution_metric(
    solution: &FemLinearStaticSolution,
    aggregate: &str,
    field: &str,
    support: Option<&str>,
) -> Result<FemMetricObservation, String> {
    let extremum = |field: &str, unit: &str, value: &FemResultExtremum| FemMetricObservation {
        field: field.into(),
        value: Some(value.value),
        infinite: false,
        unit: unit.into(),
        node_id: value.node_id,
        element_id: value.element_id,
        coordinate_mm: Some([
            value.coordinate_mm.x_mm,
            value.coordinate_mm.y_mm,
            value.coordinate_mm.z_mm,
        ]),
    };
    match (aggregate, field) {
        ("fem-max", "von-mises" | "von-mises-stress") => Ok(extremum(
            "von-mises-stress",
            "MPa",
            &solution.postprocess.summary.maximum_von_mises,
        )),
        ("fem-max", "displacement" | "displacement-magnitude") => Ok(extremum(
            "maximum-displacement",
            "mm",
            &solution.postprocess.summary.maximum_displacement,
        )),
        ("fem-max", "principal-stress" | "maximum-principal-stress") => Ok(extremum(
            "maximum-principal-stress",
            "MPa",
            &solution.postprocess.summary.maximum_principal_stress,
        )),
        ("fem-max", "mass") => Ok(FemMetricObservation {
            field: "mass".into(),
            value: Some(solution.postprocess.summary.mass_kg),
            infinite: false,
            unit: "kg".into(),
            node_id: None,
            element_id: None,
            coordinate_mm: None,
        }),
        ("fem-max", "reaction" | "reaction-resultant") => {
            let reactions = solution
                .support_reactions
                .iter()
                .filter(|reaction| support.is_none_or(|expected| reaction.name == expected));
            let value = reactions
                .map(|reaction| {
                    reaction
                        .resultant_n
                        .iter()
                        .map(|value| value * value)
                        .sum::<f64>()
                        .sqrt()
                })
                .reduce(f64::max)
                .ok_or_else(|| {
                    format!(
                        "FEM reaction metric found no current support group '{}'.",
                        support.unwrap_or("*")
                    )
                })?;
            Ok(FemMetricObservation {
                field: "reaction-resultant".into(),
                value: Some(value),
                infinite: false,
                unit: "N".into(),
                node_id: None,
                element_id: None,
                coordinate_mm: None,
            })
        }
        ("fem-min", "safety-factor" | "yield-safety-factor") => {
            let (value, infinite) = match solution.postprocess.summary.minimum_yield_safety_factor {
                FemSafetyFactor::Finite { value } => (Some(value), false),
                FemSafetyFactor::Infinite => (None, true),
            };
            Ok(FemMetricObservation {
                field: "minimum-yield-safety-factor".into(),
                value,
                infinite,
                unit: "dimensionless".into(),
                node_id: None,
                element_id: None,
                coordinate_mm: None,
            })
        }
        _ => Err(format!(
            "Unsupported FEM verification metric `({aggregate} <study> {field})`."
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_fem_mesh_pipeline_started<F>(
    program: &CoreProgram,
    analysis_name: &str,
    tagged_anchors: &BTreeMap<String, TaggedAnchorBinding>,
    boundary: &AnalysisBoundarySurface,
    step_path: &Path,
    budgets: FemBudgetLimits,
    runtime: &ExactBrepMesherRuntime,
    scratch_dir: &Path,
    control: &FemPipelineControl,
    mesh_size_override_mm: Option<f64>,
    cancelled: &AtomicBool,
    started: Instant,
    progress: &mut F,
) -> AppResult<FemMeshPipelineResult>
where
    F: FnMut(FemProgressEvent),
{
    control.validate()?;
    emit_progress(
        progress,
        started,
        FemPipelineStage::Resolve,
        None,
        None,
        "Resolving authored study and durable BRep face tags.",
        true,
    );
    ensure_not_cancelled(cancelled, analysis_name)?;
    let resolved_faces = resolve_fem_face_tags(tagged_anchors, boundary)?;
    let mut study = authored_study_from_core(program, analysis_name, &resolved_faces, budgets)?;
    if let Some(mesh_size_mm) = mesh_size_override_mm {
        apply_convergence_mesh_size(&mut study.mesh_control, mesh_size_mm)?;
    }
    let authored_ledger = engineering_ledger_from_core(
        program,
        analysis_name,
        &boundary.source_geometry_digest,
        &boundary.source_geometry_digest,
    )?;

    emit_progress(
        progress,
        started,
        FemPipelineStage::BoundaryMesh,
        Some(boundary.vertices.len() as u64),
        None,
        "Validated exact OCCT analysis boundary and source face groups.",
        true,
    );
    ensure_not_cancelled(cancelled, analysis_name)?;
    let pre_solve_input = pre_solve_input(
        boundary,
        &study.constraints,
        &study.loads,
        study.material.poisson_ratio,
    )?;
    let pre_solve_applicability =
        audit_pre_solve_applicability(&pre_solve_input).map_err(|error| {
            AppError::validation(format!("FEM pre-solve applicability failed: {error}"))
        })?;
    let mut required_face_targets = Vec::new();
    for constraint in &study.constraints {
        let faces = match constraint {
            FemConstraint::Fixed { faces, .. }
            | FemConstraint::PrescribedDisplacement { faces, .. } => faces,
        };
        for face in faces {
            if !required_face_targets.contains(face) {
                required_face_targets.push(face.clone());
            }
        }
    }
    for load in &study.loads {
        let faces = match load {
            FemLoad::SurfaceForce { faces, .. }
            | FemLoad::Traction { faces, .. }
            | FemLoad::Pressure { faces, .. } => faces,
        };
        for face in faces {
            if !required_face_targets.contains(face) {
                required_face_targets.push(face.clone());
            }
        }
    }

    emit_progress(
        progress,
        started,
        FemPipelineStage::VolumeMesh,
        Some(boundary.vertices.len() as u64),
        None,
        "Running exact-BRep Tet4 meshing through HXT with Netgen fallback.",
        true,
    );
    let mesh_request = GmshBrepMeshRequest::from_analysis_boundary(
        format!("{analysis_name}-volume-mesh"),
        step_path.to_path_buf(),
        boundary,
        &study.mesh_control,
        control.minimum_scaled_jacobian,
        control.maximum_runtime_ms,
        control.thread_count,
        &required_face_targets,
    )?;
    let mesh = run_exact_brep_mesher(runtime, &mesh_request, scratch_dir, cancelled)?;
    emit_progress(
        progress,
        started,
        FemPipelineStage::ValidateMesh,
        Some(mesh.nodes.len() as u64),
        Some(mesh.cells.len() as u64),
        "Validated Tet4 ownership, tags, quality, connectivity, and digest.",
        true,
    );
    ensure_not_cancelled(cancelled, analysis_name)?;

    let engineering_evidence_digest = authored_ledger.canonical_digest();
    let analysis_identity = FemAnalysisIdentity {
        schema_version: FEM_SCHEMA_VERSION,
        study_name: study.analysis_name.clone(),
        part_id: study.part_id.clone(),
        geometry_digest: boundary.source_geometry_digest.clone(),
        engineering_evidence_digest,
        material_digest: study.material.canonical_digest(),
        load_digests: study
            .loads
            .iter()
            .map(CanonicalDigest::canonical_digest)
            .collect(),
        constraint_digests: study
            .constraints
            .iter()
            .map(CanonicalDigest::canonical_digest)
            .collect(),
        mesh_control_digest: study.mesh_control.canonical_digest(),
        runtime_identity_digest: mesh.mesher_identity.canonical_digest(),
    };
    analysis_identity.validate().map_err(|error| {
        AppError::validation(format!("FEM analysis identity is invalid: {error}"))
    })?;

    Ok(FemMeshPipelineResult {
        analysis_identity,
        engineering_evidence: authored_ledger,
        pre_solve_applicability,
        study,
        mesh,
    })
}

fn apply_convergence_mesh_size(
    control: &mut ecky_fem::FemMeshControl,
    mesh_size_mm: f64,
) -> AppResult<()> {
    if !mesh_size_mm.is_finite() || mesh_size_mm <= 0.0 {
        return Err(AppError::validation(
            "FEM convergence mesh size must be finite and positive.",
        ));
    }
    control.global_size_mm = mesh_size_mm;
    control
        .local_refinements
        .retain(|refinement| refinement.size_mm < mesh_size_mm);
    control.validate().map_err(|error| {
        AppError::validation(format!("FEM convergence mesh control is invalid: {error}"))
    })
}

fn ensure_not_cancelled(cancelled: &AtomicBool, analysis_name: &str) -> AppResult<()> {
    if cancelled.load(Ordering::Acquire) {
        Err(AppError::conflict(format!(
            "FEM study '{analysis_name}' was cancelled."
        )))
    } else {
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_progress<F>(
    progress: &mut F,
    started: Instant,
    stage: FemPipelineStage,
    node_count: Option<u64>,
    tet4_cell_count: Option<u64>,
    detail: &str,
    cancellation_boundary: bool,
) where
    F: FnMut(FemProgressEvent),
{
    progress(FemProgressEvent {
        stage,
        elapsed_ms: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
        node_count,
        tet4_cell_count,
        detail: detail.to_string(),
        cancellation_boundary,
    });
}

fn pre_solve_input(
    boundary: &AnalysisBoundarySurface,
    constraints: &[FemConstraint],
    loads: &[FemLoad],
    poisson_ratio: f64,
) -> AppResult<FemPreSolveApplicabilityInput> {
    let (characteristic_size_mm, minimum_thickness_mm) = boundary_dimensions(boundary)?;
    let mut constrained_components = [false; 3];
    let mut support_targets = Vec::new();
    for constraint in constraints {
        match constraint {
            FemConstraint::Fixed { faces, .. } => {
                constrained_components.fill(true);
                support_targets.extend(faces);
            }
            FemConstraint::PrescribedDisplacement {
                faces,
                displacement_mm,
                ..
            } => {
                constrained_components[0] |= displacement_mm.x_mm.is_some();
                constrained_components[1] |= displacement_mm.y_mm.is_some();
                constrained_components[2] |= displacement_mm.z_mm.is_some();
                support_targets.extend(faces);
            }
        }
    }
    let load_targets = loads
        .iter()
        .flat_map(|load| match load {
            FemLoad::SurfaceForce { faces, .. }
            | FemLoad::Traction { faces, .. }
            | FemLoad::Pressure { faces, .. } => faces.iter(),
        })
        .collect::<Vec<_>>();
    Ok(FemPreSolveApplicabilityInput {
        schema_version: FEM_SCHEMA_VERSION,
        solid_count: 1,
        unsupported_interface_count: 0,
        characteristic_size_mm,
        minimum_thickness_mm,
        poisson_ratio,
        constrained_translation_components: constrained_components
            .into_iter()
            .filter(|constrained| *constrained)
            .count() as u8,
        selected_load_area_mm2: selected_area(boundary, load_targets)?,
        selected_support_area_mm2: selected_area(boundary, support_targets)?,
        has_point_load_or_support: false,
    })
}

fn selected_area<'a>(
    boundary: &AnalysisBoundarySurface,
    targets: impl IntoIterator<Item = &'a ecky_fem::FemFaceTarget>,
) -> AppResult<f64> {
    let mut seen = BTreeSet::new();
    let mut area = 0.0;
    for target in targets {
        let matches = boundary
            .face_groups
            .iter()
            .enumerate()
            .filter(|(_, group)| {
                group.part_id == target.part_id
                    && group.canonical_target_id == target.canonical_target_id
                    && group.durable_target_id.as_deref() == Some(target.durable_target_id.as_str())
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(AppError::validation(format!(
                "FEM selected area target '{}'/'{}' resolved to {} boundary groups.",
                target.canonical_target_id,
                target.durable_target_id,
                matches.len()
            )));
        }
        if seen.insert(matches[0].0) {
            area += matches[0].1.area;
        }
    }
    Ok(area)
}

fn characteristic_size(boundary: &AnalysisBoundarySurface) -> AppResult<f64> {
    Ok(boundary_dimensions(boundary)?.0)
}

fn boundary_dimensions(boundary: &AnalysisBoundarySurface) -> AppResult<(f64, f64)> {
    let first = boundary
        .vertices
        .first()
        .copied()
        .ok_or_else(|| AppError::validation("FEM boundary has no vertices."))?;
    let mut minimum = first;
    let mut maximum = first;
    for vertex in &boundary.vertices {
        if vertex.iter().any(|value| !value.is_finite()) {
            return Err(AppError::validation(
                "FEM boundary contains a non-finite coordinate.",
            ));
        }
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(vertex[axis]);
            maximum[axis] = maximum[axis].max(vertex[axis]);
        }
    }
    let extents = [
        maximum[0] - minimum[0],
        maximum[1] - minimum[1],
        maximum[2] - minimum[2],
    ];
    let characteristic =
        (extents[0] * extents[0] + extents[1] * extents[1] + extents[2] * extents[2]).sqrt();
    let minimum_thickness = extents
        .into_iter()
        .filter(|extent| *extent > 0.0)
        .fold(f64::INFINITY, f64::min);
    if !characteristic.is_finite() || characteristic <= 0.0 || !minimum_thickness.is_finite() {
        return Err(AppError::validation(
            "FEM boundary has invalid zero-size bounds.",
        ));
    }
    Ok((characteristic, minimum_thickness))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convergence_override_drops_redundant_equal_local_refinement() {
        let mut control = ecky_fem::FemMeshControl {
            schema_version: FEM_SCHEMA_VERSION,
            element_kind: ecky_fem::FemElementKind::Tet4,
            global_size_mm: 2.4,
            local_refinements: vec![ecky_fem::FemLocalRefinement {
                schema_version: FEM_SCHEMA_VERSION,
                faces: Vec::new(),
                size_mm: 1.2,
            }],
            budgets: FemBudgetLimits {
                schema_version: FEM_SCHEMA_VERSION,
                boundary_triangles: 1,
                tet4_cells: 1,
                nodes: 1,
                dofs: 1,
                sparse_nonzeros: 1,
                result_bytes: 1,
                convergence_levels: 3,
            },
        };

        apply_convergence_mesh_size(&mut control, 1.2).expect("valid convergence override");

        assert_eq!(control.global_size_mm, 1.2);
        assert!(control.local_refinements.is_empty());
    }

    #[test]
    fn compiler_and_runtime_resolve_typed_fem_max_min_metrics_from_current_study() {
        let source = r#"
          (model
            (part bracket (box 10 10 10))
            (analysis bracket-static
              (linear-static :part bracket)
              (material steel :young-modulus 210000MPa :poisson-ratio 0.3
                :density 7850kg-per-m3 :yield-strength 250MPa)
              (volume-mesh :element tet4 :size 2mm)
              (fixed :faces (tag mounting))
              (surface-force :faces (tag load-pad) :total [0N 0N -1000N])
              (solve :method sparse-direct))
            (verify (tag stress) (metric stress (fem-max bracket-static von-mises))
              (expect stress (< (quantity 138 MPa))))
            (verify (tag displacement) (metric displacement (fem-max bracket-static displacement-magnitude))
              (expect displacement (<= (quantity 0.5 mm))))
            (verify (tag mass) (metric mass (fem-max bracket-static mass))
              (expect mass (< (quantity 1 kg))))
            (verify (tag reaction) (metric reaction (fem-max bracket-static reaction-resultant mounting))
              (expect reaction (>= (quantity 900 N))))
            (verify (tag safety) (metric safety (fem-min bracket-static yield-safety-factor))
              (expect safety (>= (quantity 1.5 dimensionless))))
            (verify (tag wrong-unit) (metric wrong-unit (fem-max bracket-static mass))
              (expect wrong-unit (< (quantity 1 mm))))
            (verify (tag stale-study) (metric stale-study (fem-max old-study mass))
              (expect stale-study (< (quantity 1 kg)))))
        "#;
        let program = ecky_render::scheme::compile_to_core_program(source)
            .expect("typed FEM verification source compiles");
        assert_eq!(program.constraints.verify_clauses.len(), 7);

        let evaluations = evaluate_authored_fem_verify_clauses_with(
            &program.constraints.verify_clauses,
            2.0,
            "bracket-static",
            "sha256:analysis",
            "sha256:mesh",
            "sha256:result",
            |aggregate, field, support| {
                let (value, unit, node_id, element_id, coordinate_mm) = match (aggregate, field) {
                    ("fem-max", "von-mises") => {
                        (120.0, "MPa", None, Some(7), Some([1.0, 2.0, 3.0]))
                    }
                    ("fem-max", "displacement-magnitude") => {
                        (0.4, "mm", Some(9), None, Some([4.0, 5.0, 6.0]))
                    }
                    ("fem-max", "mass") => (0.2, "kg", None, None, None),
                    ("fem-max", "reaction-resultant") if support == Some("mounting") => {
                        (1000.0, "N", None, None, None)
                    }
                    ("fem-min", "yield-safety-factor") => (2.0, "dimensionless", None, None, None),
                    other => return Err(format!("unexpected metric {other:?}")),
                };
                Ok(FemMetricObservation {
                    field: field.into(),
                    value: Some(value),
                    infinite: false,
                    unit: unit.into(),
                    node_id,
                    element_id,
                    coordinate_mm,
                })
            },
        );

        assert_eq!(evaluations.len(), 7);
        assert!(evaluations[..5]
            .iter()
            .all(|evaluation| evaluation.status == "passed"));
        assert_eq!(evaluations[0].element_id, Some(7));
        assert_eq!(evaluations[0].coordinate_mm, Some([1.0, 2.0, 3.0]));
        assert_eq!(evaluations[1].node_id, Some(9));
        assert_eq!(evaluations[5].status, "failed");
        assert!(evaluations[5].detail.contains("unit 'mm'"));
        assert_eq!(evaluations[6].status, "failed");
        assert!(evaluations[6].detail.contains("stale or cross-study"));
        assert!(evaluations.iter().all(|evaluation| {
            evaluation.analysis_identity_digest == "sha256:analysis"
                && evaluation.mesh_content_digest == "sha256:mesh"
                && evaluation.result_digest == "sha256:result"
        }));
    }
}
