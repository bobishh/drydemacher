use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemBudgetLimitsDto {
    pub boundary_triangles: u64,
    pub tet4_cells: u64,
    pub nodes: u64,
    pub dofs: u64,
    pub sparse_nonzeros: u64,
    pub result_bytes: u64,
    pub convergence_levels: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemPipelineControlDto {
    pub envelope_mm: f64,
    pub minimum_scaled_jacobian: f64,
    pub maximum_runtime_ms: u64,
    pub relative_solver_tolerance: f64,
    /// Zero selects available performance cores.
    #[serde(default)]
    pub thread_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemStudyRequest {
    pub job_id: String,
    pub model_id: String,
    pub source: String,
    pub analysis_name: String,
    pub budgets: FemBudgetLimitsDto,
    pub control: FemPipelineControlDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemStudyValidationResponse {
    pub job_id: String,
    pub model_id: String,
    pub analysis_name: String,
    pub part_id: String,
    pub source_digest: String,
    pub source_geometry_digest: String,
    pub boundary_digest: String,
    pub boundary_node_count: u64,
    pub boundary_triangle_count: u64,
    pub face_group_count: u64,
    pub decision_readiness_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemMeshPreviewResponse {
    pub job_id: String,
    pub model_id: String,
    pub analysis_name: String,
    pub analysis_identity_digest: String,
    pub mesh_content_digest: String,
    pub source_boundary_digest: String,
    pub manifest_path: String,
    pub arrays: Vec<FemResultArrayDto>,
    pub node_count: u64,
    pub tet4_cell_count: u64,
    pub boundary_triangle_count: u64,
    pub face_group_count: u64,
    pub minimum_scaled_jacobian: f64,
    pub minimum_radius_ratio: f64,
    pub connected_component_count: u64,
    pub boundary_area_mm2_by_group: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemResultArrayDto {
    pub name: String,
    pub path: String,
    pub scalar_type: String,
    pub shape: Vec<u64>,
    pub byte_length: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemResultSummaryDto {
    pub maximum_displacement_mm: f64,
    pub maximum_von_mises_mpa: f64,
    pub maximum_principal_stress_mpa: f64,
    pub volume_mm3: f64,
    pub mass_kg: f64,
    pub minimum_yield_safety_factor: Option<f64>,
    pub equilibrium_relative_imbalance: f64,
    pub solver_relative_residual: f64,
    pub minimum_scaled_jacobian: f64,
    pub node_count: u64,
    pub tet4_cell_count: u64,
    pub extrema: Vec<FemExtremumDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemExtremumDto {
    pub field_kind: String,
    pub value: f64,
    pub unit: String,
    pub node_id: Option<u32>,
    pub element_id: Option<u32>,
    pub coordinate_mm: [f64; 3],
    pub mesh_content_digest: String,
    pub source_boundary_digest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemSupportReactionDto {
    pub name: String,
    pub face_group_indices: Vec<u32>,
    pub resultant_n: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemAcceptanceEvaluationDto {
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
    pub evidence_chain: FemAcceptanceEvidenceChainDto,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemAcceptanceEvidenceChainDto {
    pub source_geometry_digest: String,
    pub analysis_geometry_digest: String,
    pub idealization_accepted: bool,
    pub input_evidence_ids: Vec<String>,
    pub applicability_check_ids: Vec<String>,
    pub convergence_status: Option<String>,
    pub sensitivity_result_digests: Vec<String>,
    pub validation_evidence_ids: Vec<String>,
    pub gaps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemEngineeringQuestionDto {
    pub question_id: String,
    pub statement: String,
    pub decision: String,
    pub acceptance_metric_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemIdealizationDto {
    pub artifact_digest: String,
    pub kind: String,
    pub source_geometry_digest: String,
    pub analysis_geometry_digest: String,
    pub manufacturing_geometry_digest: String,
    pub affected_topology_ids: Vec<String>,
    pub justification: String,
    pub expected_influence_percent: f64,
    pub accepted_by_user: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemInputEvidenceDto {
    pub input_name: String,
    pub evidence_id: String,
    pub subject: String,
    pub source: String,
    pub authority: String,
    pub uncertainty_percent: Option<f64>,
    pub decision_critical: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemAssumptionDto {
    pub assumption_id: String,
    pub category: String,
    pub statement: String,
    pub status: String,
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemApplicabilityCheckDto {
    pub check_id: String,
    pub kind: String,
    pub status: String,
    pub observed: Option<f64>,
    pub limit: Option<f64>,
    pub unit: Option<String>,
    pub evidence_ids: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemSensitivityMetricDto {
    pub metric_id: String,
    pub nominal: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub unit: String,
    pub dominant_input_name: Option<String>,
    pub decision_changed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemSensitivityEvidenceDto {
    pub completed: bool,
    pub case_result_digests: Vec<String>,
    pub metric_ranges: Vec<FemSensitivityMetricDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemValidationEvidenceDto {
    pub validation_id: String,
    pub kind: String,
    pub source: String,
    pub result_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemVerificationLayerDto {
    pub layer: String,
    pub status: String,
    pub evidence_ids: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemEngineeringEvidenceDto {
    pub question: FemEngineeringQuestionDto,
    pub idealization: FemIdealizationDto,
    pub inputs: Vec<FemInputEvidenceDto>,
    pub assumptions: Vec<FemAssumptionDto>,
    pub applicability: Vec<FemApplicabilityCheckDto>,
    pub sensitivity: Option<FemSensitivityEvidenceDto>,
    pub validation_evidence: Vec<FemValidationEvidenceDto>,
    pub verification_layers: Vec<FemVerificationLayerDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemRunResponse {
    pub job_id: String,
    pub model_id: String,
    pub analysis_name: String,
    pub source_digest: String,
    pub analysis_identity_digest: String,
    pub solution_digest: String,
    pub result_digest: String,
    pub mesh_content_digest: String,
    pub source_boundary_digest: String,
    pub decision_ready: bool,
    pub decision_readiness_error: Option<String>,
    pub manifest_path: String,
    pub arrays: Vec<FemResultArrayDto>,
    pub summary: FemResultSummaryDto,
    pub support_reactions: Vec<FemSupportReactionDto>,
    pub engineering_evidence: FemEngineeringEvidenceDto,
    pub acceptance_evaluations: Vec<FemAcceptanceEvaluationDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemResultReadRequest {
    pub analysis_identity_digest: String,
    pub solution_digest: String,
    pub maximum_result_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemResultReadResponse {
    pub source_digest: String,
    pub analysis_identity_digest: String,
    pub solution_digest: String,
    pub result_digest: String,
    pub mesh_content_digest: String,
    pub source_boundary_digest: String,
    pub decision_ready: bool,
    pub decision_readiness_error: Option<String>,
    pub manifest_path: String,
    pub arrays: Vec<FemResultArrayDto>,
    pub summary: FemResultSummaryDto,
    pub support_reactions: Vec<FemSupportReactionDto>,
    pub engineering_evidence: FemEngineeringEvidenceDto,
    pub acceptance_evaluations: Vec<FemAcceptanceEvaluationDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemVtuExportResponse {
    pub path: String,
    pub byte_length: u64,
    pub sha256: String,
    pub result_digest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemCancelResponse {
    pub job_id: String,
    pub cancellation_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemConvergenceRequest {
    pub study: FemStudyRequest,
    pub mesh_sizes_mm: Vec<f64>,
    pub displacement_relative_tolerance: f64,
    pub stress_relative_tolerance: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemConvergenceLevelDto {
    pub mesh_size_mm: f64,
    pub status: String,
    pub error: Option<String>,
    pub analysis_identity_digest: Option<String>,
    pub solution_digest: Option<String>,
    pub result_digest: Option<String>,
    pub mesh_content_digest: Option<String>,
    pub node_count: Option<u64>,
    pub tet4_cell_count: Option<u64>,
    pub minimum_scaled_jacobian: Option<f64>,
    pub equilibrium_relative_imbalance: Option<f64>,
    pub solver_relative_residual: Option<f64>,
    pub maximum_displacement_mm: Option<f64>,
    pub maximum_von_mises_mpa: Option<f64>,
    pub displacement_relative_delta: Option<f64>,
    pub stress_relative_delta: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemConvergenceResponse {
    pub job_id: String,
    pub model_id: String,
    pub analysis_name: String,
    pub sequence_status: String,
    pub levels: Vec<FemConvergenceLevelDto>,
    pub displacement_status: String,
    pub stress_status: String,
    pub acceptance_evaluations: Vec<FemAcceptanceEvaluationDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemTopologyMaterialDto {
    pub name: String,
    pub young_modulus_mpa: f64,
    pub poisson_ratio: f64,
    pub density_kg_per_mm3: f64,
    pub yield_strength_mpa: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemTopologySurfaceLoadDto {
    pub id: String,
    pub weight: f64,
    pub face_group_indices: Vec<u32>,
    pub total_force_n: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemTopologyControlsDto {
    pub volume_fraction: f64,
    pub penalty: f64,
    pub minimum_density: f64,
    pub filter_radius_mm: f64,
    pub move_limit: f64,
    pub convergence_tolerance: f64,
    #[serde(default)]
    pub maximum_iterations: u64,
    #[serde(default)]
    pub maximum_new_iterations: u64,
    #[serde(default)]
    pub maximum_dimension: u64,
    #[serde(default)]
    pub maximum_elements: u64,
    #[serde(default)]
    pub maximum_solve_count: u64,
    #[serde(default)]
    pub maximum_working_memory_bytes: u64,
    #[serde(default)]
    pub maximum_result_bytes: u64,
    #[serde(default)]
    pub maximum_wall_time_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemTopologyRunRequest {
    pub study: FemStudyRequest,
    pub resume_state_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemTopologyRunResponse {
    pub job_id: String,
    pub analysis_identity_digest: String,
    pub mesh_content_digest: String,
    pub input_digest: String,
    pub state_digest: String,
    pub result_digest: Option<String>,
    pub termination: String,
    pub iteration_count: u64,
    pub initial_compliance: Option<f64>,
    pub final_compliance: Option<f64>,
    pub final_volume_fraction: Option<f64>,
    pub passive_solid_volume_fraction: Option<f64>,
    pub passive_void_volume_fraction: Option<f64>,
    pub gcmma_trace_edn: String,
    pub checkpoint_path: String,
    pub density_path: Option<String>,
    pub preview_vtu_path: Option<String>,
    pub exact_brep: bool,
    pub production_step: bool,
    pub engineering_accepted: bool,
    pub scope_disclaimer: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemTopologyReconstructRequest {
    pub study: FemStudyRequest,
    pub analysis_identity_digest: String,
    pub mesh_content_digest: String,
    pub input_digest: String,
    pub state_digest: String,
    pub density_threshold: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FemTopologyReconstructResponse {
    pub analysis_identity_digest: String,
    pub mesh_content_digest: String,
    pub input_digest: String,
    pub state_digest: String,
    pub result_digest: String,
    pub solid_expression: String,
    pub vertex_count: u64,
    pub triangle_count: u64,
    pub discarded_cell_count: u64,
    pub discarded_active_volume_fraction: f64,
    pub connected_anchor_ids: Vec<String>,
    pub signed_volume_mm3: f64,
    pub closed_manifold: bool,
    pub exact_brep: bool,
    pub independently_verified: bool,
    pub scope_disclaimer: String,
}

#[cfg(test)]
mod tests {
    use super::FemTopologyControlsDto;

    #[test]
    fn topology_controls_admit_explicit_working_memory_bound() {
        let controls = serde_json::from_value::<FemTopologyControlsDto>(serde_json::json!({
            "volumeFraction": 0.5,
            "penalty": 3.0,
            "minimumDensity": 0.001,
            "filterRadiusMm": 5.0,
            "moveLimit": 0.1,
            "convergenceTolerance": 0.0001,
            "maximumIterations": 10,
            "maximumNewIterations": 2,
            "maximumDimension": 1000,
            "maximumElements": 1000,
            "maximumSolveCount": 20,
            "maximumWorkingMemoryBytes": 100000000,
            "maximumResultBytes": 1000000,
            "maximumWallTimeMs": 10000
        }))
        .expect("bounded topology controls");
        assert_eq!(controls.maximum_working_memory_bytes, 100_000_000);
    }

    #[test]
    fn topology_controls_reject_manual_mesh_cell_masks() {
        let error = serde_json::from_value::<FemTopologyControlsDto>(serde_json::json!({
            "volumeFraction": 0.5,
            "penalty": 3.0,
            "minimumDensity": 0.001,
            "filterRadiusMm": 5.0,
            "moveLimit": 0.1,
            "convergenceTolerance": 0.0001,
            "maximumIterations": 10,
            "maximumNewIterations": 2,
            "maximumDimension": 1000,
            "maximumElements": 1000,
            "maximumSolveCount": 20,
            "maximumWorkingMemoryBytes": 100000000,
            "maximumResultBytes": 1000000,
            "maximumWallTimeMs": 10000,
            "passiveSolidCells": [0]
        }))
        .expect_err("mesh cell masks are internal topology state");
        assert!(error.to_string().contains("passiveSolidCells"));
    }
}
