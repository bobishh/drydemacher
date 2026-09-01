use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use specta::Type;
use std::collections::HashSet;

pub const CAPTURE_RECONSTRUCTION_GUIDE_SCHEMA_VERSION: u32 = 1;
const MAX_ABS_COORDINATE: f64 = 1.0e9;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureSourceBounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureMeshSelection {
    Raw,
    Crop,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureGuideSourceMesh {
    pub artifact_digest: String,
    pub content_digest: String,
    pub selection: CaptureMeshSelection,
    pub crop_digest: Option<String>,
    pub triangle_count: u64,
    pub source_bounds: CaptureSourceBounds,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureGuideContext {
    pub source_mesh: CaptureGuideSourceMesh,
    pub target_source_digest: String,
    pub target_version_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnsureCaptureReconstructionGuideResult {
    pub guide: CaptureReconstructionGuide,
    pub state: crate::contracts::CaptureReconstructionGuideState,
    pub created: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureSurfaceAnchor {
    pub source_mesh_content_digest: String,
    pub triangle_index: u64,
    pub barycentric: [f64; 3],
    pub source_position: [f64; 3],
    pub source_normal: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureSurfaceNeighborhood {
    pub neighborhood_id: String,
    pub landmark_id: String,
    pub source_mesh_content_digest: String,
    pub seed_triangle_index: u64,
    pub triangle_indices: Vec<u64>,
    pub adjacency_edges: Vec<[u64; 2]>,
    pub vertex_indices: Vec<u64>,
    pub sample_count: u64,
    pub radius_source_units: f64,
    pub sampled_area_source_units_squared: f64,
    pub radial_coverage_ratio: f64,
    pub centroid_source: [f64; 3],
    pub mean_normal: [f64; 3],
    pub normal_spread_deg: f64,
    pub normal_variation_rms_deg: f64,
    pub estimated_curvature_per_source_unit: f64,
    pub position_rms_source_units: f64,
    pub planarity_rms_source_units: f64,
    pub planarity_max_source_units: f64,
    pub position_uncertainty_source_units: f64,
    pub reached_mesh_boundary: bool,
    pub truncated_by_budget: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureEvidenceComputationPolicy {
    pub neighborhood_radius_mm: f64,
    pub max_neighborhood_triangles: u64,
}

impl Default for CaptureEvidenceComputationPolicy {
    fn default() -> Self {
        Self {
            neighborhood_radius_mm: 2.0,
            max_neighborhood_triangles: 64,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureLandmarkRole {
    CalibrationEndpoint,
    FrameOrigin,
    FrameDirection,
    SymmetrySample,
    RotationAxisEndpoint,
    ProfileVertex,
    MatingSurfaceSample,
    BoreSample,
    OuterExtent,
    ClearanceBoundary,
    IgnoredDamagedRegion,
    NamedReference,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureLandmark {
    pub landmark_id: String,
    pub label: String,
    pub role: CaptureLandmarkRole,
    pub anchor: CaptureSurfaceAnchor,
    pub local_position_mm: [f64; 3],
    pub local_normal: [f64; 3],
    pub uncertainty_mm: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureKnownDistanceMeasurement {
    pub measurement_id: String,
    pub label: String,
    pub first_landmark_id: String,
    pub second_landmark_id: String,
    pub known_distance_mm: f64,
    pub fitted_distance_mm: f64,
    pub residual_mm: f64,
    pub accepted_tolerance_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum CaptureCalibrationMethod {
    KnownDistance,
    #[specta(rename_all = "camelCase")]
    TrustedMetricMetadata {
        provenance: String,
        accepted_by_user: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureCalibration {
    pub source_units: String,
    pub millimetres_per_source_unit: f64,
    pub method: CaptureCalibrationMethod,
    pub measurements: Vec<CaptureKnownDistanceMeasurement>,
    pub residual_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureReconstructionFrame {
    pub origin_mm: [f64; 3],
    pub x_axis: [f64; 3],
    pub y_axis: [f64; 3],
    pub z_axis: [f64; 3],
    pub source_landmark_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureExpectedGeometryKind {
    Point,
    Curve,
    Plane,
    Cylinder,
    Profile,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureRequiredBrepTopologyKind {
    Vertex,
    Edge,
    Face,
    OrderedEdges,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureSelectorCardinality {
    One,
    OneOrMore,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum CaptureAuthoredSelector {
    Binding { name: String },
    Tag { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureFeatureExpectation {
    pub expectation_id: String,
    pub guide_item_ids: Vec<String>,
    pub label: String,
    pub expected_geometry_kind: CaptureExpectedGeometryKind,
    pub required_brep_topology_kind: CaptureRequiredBrepTopologyKind,
    pub cardinality: CaptureSelectorCardinality,
    pub part_id: String,
    pub instance_path: Option<String>,
    pub expected_authored_selector: CaptureAuthoredSelector,
    pub required_for_acceptance: bool,
    pub position_tolerance_mm: Option<f64>,
    pub normal_tolerance_deg: Option<f64>,
    pub radial_tolerance_mm: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureNamedMeasurement {
    pub measurement_id: String,
    pub label: String,
    pub landmark_ids: Vec<String>,
    pub value: f64,
    pub unit: String,
    pub fit_critical: bool,
    pub authored_parameter_name: Option<String>,
    #[serde(default)]
    pub constraint_kind: Option<CaptureConstraintKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureFitResidual {
    pub rms_mm: f64,
    pub max_mm: f64,
    pub tolerance_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum CaptureAnalyticPrimitive {
    #[specta(rename_all = "camelCase")]
    Line {
        origin_mm: [f64; 3],
        direction: [f64; 3],
    },
    #[specta(rename_all = "camelCase")]
    Plane {
        origin_mm: [f64; 3],
        normal: [f64; 3],
    },
    #[specta(rename_all = "camelCase")]
    Circle {
        center_mm: [f64; 3],
        normal: [f64; 3],
        radius_mm: f64,
    },
    #[specta(rename_all = "camelCase")]
    Cylinder {
        origin_mm: [f64; 3],
        axis_direction: [f64; 3],
        radius_mm: f64,
        min_axis_mm: f64,
        max_axis_mm: f64,
    },
    #[specta(rename_all = "camelCase")]
    Cone {
        apex_mm: [f64; 3],
        axis_direction: [f64; 3],
        half_angle_deg: f64,
        min_axis_mm: f64,
        max_axis_mm: f64,
    },
    #[specta(rename_all = "camelCase")]
    Sphere { center_mm: [f64; 3], radius_mm: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapturePrimitiveCandidate {
    pub candidate_id: String,
    pub label: String,
    pub guide_item_ids: Vec<String>,
    pub neighborhood_ids: Vec<String>,
    pub geometry: CaptureAnalyticPrimitive,
    pub fit: CaptureFitResidual,
    pub support_sample_count: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CapturePrimitiveKind {
    Line,
    Plane,
    Circle,
    Cylinder,
    Cone,
    Sphere,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CapturePrimitiveHypothesisStatus {
    Supported,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapturePrimitiveFitDomain {
    pub parameter_name: Option<String>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub observed_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapturePrimitiveHypothesis {
    pub hypothesis_id: String,
    pub guide_item_ids: Vec<String>,
    pub kind: CapturePrimitiveKind,
    pub status: CapturePrimitiveHypothesisStatus,
    pub candidate_id: Option<String>,
    pub domain: CapturePrimitiveFitDomain,
    pub fit: Option<CaptureFitResidual>,
    #[serde(default)]
    pub robust_evidence: Option<CapturePrimitiveRobustEvidence>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapturePrimitiveRobustEvidence {
    pub method: String,
    pub excluded_guide_item_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureSurfaceRegionKind {
    Plane,
    Cylinder,
    Cone,
    Sphere,
    Freeform,
    IgnoredDamage,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureSurfaceRegion {
    pub region_id: String,
    pub source_mesh_content_digest: String,
    pub triangle_indices: Vec<u64>,
    pub landmark_ids: Vec<String>,
    pub primitive_candidate_ids: Vec<String>,
    pub kind: CaptureSurfaceRegionKind,
    pub area_source_units_squared: f64,
    pub boundary_edge_count: u64,
    pub ignored: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureRegionRelation {
    Smooth,
    Sharp,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureRegionAdjacency {
    pub first_region_id: String,
    pub second_region_id: String,
    pub shared_edge_count: u64,
    pub relation: CaptureRegionRelation,
    pub maximum_normal_angle_deg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureNamedAxis {
    pub axis_id: String,
    pub label: String,
    pub landmark_ids: Vec<String>,
    pub origin_mm: [f64; 3],
    pub direction: [f64; 3],
    pub fit: CaptureFitResidual,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CapturePlaneRole {
    Symmetry,
    MatingSurface,
    Support,
    Reference,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureNamedPlane {
    pub plane_id: String,
    pub label: String,
    pub role: CapturePlaneRole,
    pub landmark_ids: Vec<String>,
    pub origin_mm: [f64; 3],
    pub normal: [f64; 3],
    pub fit: CaptureFitResidual,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureProfileKind {
    Open,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureProfileOperationHint {
    Extrude,
    Revolve,
    Sweep,
    ReferenceOnly,
    AgentDecide,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureOrderedProfile {
    pub profile_id: String,
    pub label: String,
    pub kind: CaptureProfileKind,
    pub support_plane_id: String,
    pub landmark_ids: Vec<String>,
    pub operation_hint: CaptureProfileOperationHint,
    pub feature_label: Option<String>,
    pub fit_role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum CaptureProfileSegmentGeometry {
    #[specta(rename_all = "camelCase")]
    Line {
        start_mm: [f64; 3],
        end_mm: [f64; 3],
    },
    #[specta(rename_all = "camelCase")]
    Arc {
        center_mm: [f64; 3],
        normal: [f64; 3],
        radius_mm: f64,
        start_angle_deg: f64,
        end_angle_deg: f64,
    },
    #[specta(rename_all = "camelCase")]
    Circle {
        center_mm: [f64; 3],
        normal: [f64; 3],
        radius_mm: f64,
    },
    #[specta(rename_all = "camelCase")]
    Spline {
        degree: u32,
        control_points_mm: Vec<[f64; 3]>,
        knots: Vec<f64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureProfileSegment {
    pub segment_id: String,
    pub source_landmark_ids: Vec<String>,
    pub neighborhood_ids: Vec<String>,
    pub parameter_range: [f64; 2],
    pub geometry: CaptureProfileSegmentGeometry,
    pub fit: CaptureFitResidual,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureReconstructedProfile {
    pub candidate_id: String,
    pub source_profile_id: String,
    pub support_plane_id: String,
    pub segments: Vec<CaptureProfileSegment>,
    pub closed: bool,
    pub continuous: bool,
    pub closure_error_mm: f64,
    pub maximum_continuity_gap_mm: f64,
    pub support_plane_max_mm: f64,
    pub supporting_evidence_ids: Vec<String>,
    pub rejected_hypotheses: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureConstraintKind {
    Symmetry,
    Coaxial,
    Coplanar,
    Parallel,
    Perpendicular,
    Tangent,
    EqualRadius,
    Thickness,
    Extent,
    Clearance,
    Tolerance,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureNamedConstraint {
    pub constraint_id: String,
    pub label: String,
    pub kind: CaptureConstraintKind,
    pub entity_ids: Vec<String>,
    pub parameter_name: Option<String>,
    pub value: Option<f64>,
    pub unit: Option<String>,
    pub tolerance: f64,
    pub residual: Option<f64>,
    pub user_confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureDimensionEvidence {
    pub dimension_id: String,
    pub label: String,
    pub landmark_ids: Vec<String>,
    pub value: f64,
    pub unit: String,
    pub fit_critical: bool,
    pub parameter_name: Option<String>,
    pub constraint_kind: Option<CaptureConstraintKind>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureConstraintGraph {
    pub dimensions: Vec<CaptureDimensionEvidence>,
    pub relations: Vec<CaptureNamedConstraint>,
    pub content_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum CaptureFeatureOperation {
    #[specta(rename_all = "camelCase")]
    Extrude {
        profile_candidate_id: String,
        distance_dimension_id: String,
    },
    #[specta(rename_all = "camelCase")]
    Revolve {
        profile_candidate_id: String,
        axis_id: String,
        angle_deg: f64,
    },
    #[specta(rename_all = "camelCase")]
    Sweep {
        profile_candidate_id: String,
        path_id: String,
    },
    #[specta(rename_all = "camelCase")]
    Mirror { plane_id: String },
    #[specta(rename_all = "camelCase")]
    BooleanUnion { operand_plan_ids: Vec<String> },
    #[specta(rename_all = "camelCase")]
    BooleanDifference {
        base_plan_id: String,
        cutter_plan_ids: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureFeaturePlanStatus {
    Supported,
    Rejected,
    NeedsConfirmation,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureFeaturePlanCandidate {
    pub plan_id: String,
    pub label: String,
    pub operations: Vec<CaptureFeatureOperation>,
    pub supporting_evidence_ids: Vec<String>,
    pub rejecting_evidence: Vec<String>,
    pub score: f64,
    pub status: CaptureFeaturePlanStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum CaptureReconstructionStage {
    Neighborhood,
    PrimitiveFit,
    Segmentation,
    ProfileReconstruction,
    ConstraintGraph,
    FeaturePlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureStageBypass {
    pub stage: CaptureReconstructionStage,
    pub affected_evidence_ids: Vec<String>,
    pub explicit_constraint_ids: Vec<String>,
    pub rationale: String,
    pub accepted_by_user: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureReadinessStageStatus {
    Satisfied,
    Missing,
    Ambiguous,
    Bypassed,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureReadinessStageEvidence {
    pub stage: CaptureReconstructionStage,
    pub status: CaptureReadinessStageStatus,
    pub affected_evidence_ids: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureReconstructionReadiness {
    pub ready: bool,
    pub stages: Vec<CaptureReadinessStageEvidence>,
    pub missing_stages: Vec<CaptureReconstructionStage>,
    pub ambiguous_stages: Vec<CaptureReconstructionStage>,
    pub selected_feature_plan_id: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureIgnoredRegion {
    pub region_id: String,
    pub label: String,
    pub landmark_ids: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureEvidenceView {
    pub view_id: String,
    pub label: String,
    pub camera_position_mm: [f64; 3],
    pub camera_target_mm: [f64; 3],
    pub camera_up: [f64; 3],
    pub landmark_ids: Vec<String>,
    pub profile_ids: Vec<String>,
    pub artifact_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum CaptureSymmetryCompletion {
    None,
    #[specta(rename_all = "camelCase")]
    Half {
        plane_id: String,
    },
    #[specta(rename_all = "camelCase")]
    Quarter {
        first_plane_id: String,
        second_plane_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureCorrespondenceRelation {
    Observes,
    Constrains,
    Profiles,
    DefinesAxis,
    DefinesSurface,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureCorrespondenceStatus {
    Satisfied,
    Ambiguous,
    Missing,
    WrongKind,
    OverTolerance,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureCorrespondenceResidual {
    pub metric: String,
    pub maximum: f64,
    pub rms: f64,
    pub unit: String,
    #[serde(default)]
    pub components: Vec<CaptureCorrespondenceResidualComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureCorrespondenceResidualComponent {
    pub metric: String,
    pub maximum: f64,
    pub rms: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureEvidenceCorrespondence {
    pub expectation_id: String,
    pub guide_item_ids: Vec<String>,
    pub part_id: String,
    pub instance_path: Option<String>,
    pub authored_selector: CaptureAuthoredSelector,
    pub selector_cardinality: CaptureSelectorCardinality,
    pub brep_target_kind: CaptureRequiredBrepTopologyKind,
    pub canonical_target_ids: Vec<String>,
    pub durable_target_ids: Vec<String>,
    pub source_stable_node_keys: Vec<String>,
    pub source_geometry_digest: String,
    pub relation: CaptureCorrespondenceRelation,
    pub residual: Option<CaptureCorrespondenceResidual>,
    pub status: CaptureCorrespondenceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureGuideResultProvenance {
    pub guide_id: String,
    pub guide_revision: u64,
    pub guide_canonical_digest: String,
    pub source_mesh_artifact_digest: String,
    pub source_mesh_content_digest: String,
    pub target_source_digest: String,
    pub target_version_id: Option<String>,
    pub generated_source_digest: String,
    pub geometry_digest: String,
    pub assumptions: Vec<String>,
    pub inferred_regions: Vec<String>,
    #[serde(default)]
    pub selected_feature_plan_id: Option<String>,
    #[serde(default)]
    pub feature_operation_traces: Vec<CaptureFeatureOperationTrace>,
    pub correspondences: Vec<CaptureEvidenceCorrespondence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureFeatureOperationTrace {
    pub operation_index: u64,
    pub operation_kind: String,
    pub evidence_ids: Vec<String>,
    pub authored_node_keys: Vec<String>,
    pub authored_binding_names: Vec<String>,
    pub brep_target_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureObservedDeviationReport {
    pub schema_version: u32,
    pub guide_id: String,
    pub guide_revision: u64,
    pub guide_canonical_digest: String,
    pub source_mesh_content_digest: String,
    pub generated_geometry_digest: String,
    pub parts: Vec<CaptureDeviationPartIdentity>,
    pub source_vertex_count: u64,
    pub sample_count: u64,
    pub maximum_mm: f64,
    pub rms_mm: f64,
    pub percentile_95_mm: f64,
    pub outlier_threshold_mm: f64,
    pub outlier_count: u64,
    pub evidence_scope: String,
    #[serde(default)]
    pub display_samples: Vec<CaptureDeviationDisplaySample>,
    pub content_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureDeviationDisplaySample {
    pub source_vertex_index: u64,
    pub local_position_mm: [f64; 3],
    pub distance_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureDeviationPartIdentity {
    pub part_id: String,
    pub source_geometry_digest: String,
    pub analysis_boundary_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureGuidedOutputRequirements {
    pub source_language: String,
    pub geometry_representation: String,
    pub require_parametric_source: bool,
    pub require_named_fit_constraints: bool,
    pub require_explicit_symmetry_operations: bool,
    pub forbid_mesh_solidification: bool,
    pub forbid_unbound_feature_operations: bool,
    pub selected_feature_plan_id: String,
    pub required_feature_expectation_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureGuidedReconstructionRequest {
    pub schema_version: u32,
    pub request_id: String,
    pub capture_run_id: String,
    pub guide_id: String,
    pub guide_revision: u64,
    pub guide_canonical_digest: String,
    pub target_thread_id: String,
    pub target_message_id: Option<String>,
    pub target_source_digest: String,
    pub target_version_id: Option<String>,
    pub source_mesh_artifact_digest: String,
    pub source_mesh_content_digest: String,
    pub instruction: String,
    pub guide: CaptureReconstructionGuide,
    pub evidence_views: Vec<CaptureEvidenceView>,
    pub requirements: CaptureGuidedOutputRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureGuidedCommitResult {
    pub schema_version: u32,
    pub request_id: String,
    pub guide_canonical_digest: String,
    pub unresolved_assumptions: Vec<String>,
    pub inferred_regions: Vec<String>,
}

impl CaptureGuidedCommitResult {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!(
                "Unsupported capture guided commit result schema version '{}'.",
                self.schema_version
            ));
        }
        if self.request_id.trim().is_empty() {
            return Err("Capture guided commit result request ID is empty.".into());
        }
        require_digest(
            "guided result guide canonical",
            &self.guide_canonical_digest,
        )?;
        if self.unresolved_assumptions.len() > 64 || self.inferred_regions.len() > 64 {
            return Err("Capture guided commit result exceeds bounded evidence lists.".into());
        }
        let mut seen = HashSet::new();
        for value in self
            .unresolved_assumptions
            .iter()
            .chain(&self.inferred_regions)
        {
            let value = value.trim();
            if value.is_empty() || value.len() > 2_048 {
                return Err(
                    "Capture guided commit result contains empty or oversized evidence.".into(),
                );
            }
            if !seen.insert(value) {
                return Err(format!(
                    "Capture guided commit result repeats evidence '{value}'."
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueuedCaptureGuidedReconstruction {
    pub request_id: String,
    pub thread_id: String,
    pub message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum TypedGeometryRef {
    #[specta(rename_all = "camelCase")]
    CaptureAnchor {
        source_mesh_content_digest: String,
        triangle_index: u64,
        barycentric: [f64; 3],
    },
    #[specta(rename_all = "camelCase")]
    BrepTarget {
        artifact_digest: String,
        source_geometry_digest: String,
        part_id: String,
        instance_path: Option<String>,
        topology_kind: CaptureRequiredBrepTopologyKind,
        target_id: String,
    },
    #[specta(rename_all = "camelCase")]
    PreviewRenderVertex {
        render_artifact_digest: String,
        vertex_index: u64,
    },
    #[specta(rename_all = "camelCase")]
    AnalysisBoundaryVertex {
        boundary_digest: String,
        vertex_index: u64,
    },
    #[specta(rename_all = "camelCase")]
    FemVolumeNode {
        volume_mesh_digest: String,
        node_index: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureAnchorRemapProposal {
    pub proposal_id: String,
    pub landmark_id: String,
    pub old_anchor: CaptureSurfaceAnchor,
    pub new_anchor: CaptureSurfaceAnchor,
    pub residual_mm: f64,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum CaptureGuideEditIntent {
    #[specta(rename_all = "camelCase")]
    AddLandmark {
        role: CaptureLandmarkRole,
        anchor: CaptureSurfaceAnchor,
    },
    #[specta(rename_all = "camelCase")]
    UpdateLandmark {
        landmark_id: String,
        label: String,
        role: CaptureLandmarkRole,
    },
    #[specta(rename_all = "camelCase")]
    DeleteLandmark { landmark_id: String },
    #[specta(rename_all = "camelCase")]
    ReplaceDraft {
        guide: Box<CaptureReconstructionGuide>,
    },
    #[specta(rename_all = "camelCase")]
    ConfigureProfile {
        profile_id: String,
        label: String,
        profile_kind: CaptureProfileKind,
        operation_hint: CaptureProfileOperationHint,
        support_plane_id: String,
        feature_label: Option<String>,
        fit_role: Option<String>,
    },
    #[specta(rename_all = "camelCase")]
    ReorderProfileLandmark {
        profile_id: String,
        landmark_id: String,
        target_index: u64,
    },
    #[specta(rename_all = "camelCase")]
    UpdateFeatureExpectation {
        expectation_id: String,
        label: String,
        expected_geometry_kind: CaptureExpectedGeometryKind,
        required_brep_topology_kind: CaptureRequiredBrepTopologyKind,
        cardinality: CaptureSelectorCardinality,
        part_id: String,
        instance_path: Option<String>,
        expected_authored_selector: CaptureAuthoredSelector,
        required_for_acceptance: bool,
        position_tolerance_mm: Option<f64>,
        normal_tolerance_deg: Option<f64>,
        radial_tolerance_mm: Option<f64>,
    },
    #[specta(rename_all = "camelCase")]
    SelectFeaturePlan { plan_id: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyCaptureGuideEditInput {
    pub run_id: String,
    pub expected_revision: u64,
    pub expected_mesh_digest: String,
    pub edit: CaptureGuideEditIntent,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyCaptureGuideEditResult {
    pub guide: CaptureReconstructionGuide,
    pub state: crate::contracts::CaptureReconstructionGuideState,
    pub base_revision: u64,
    pub expected_revision_matched: bool,
    pub source_digest_matched: bool,
    pub raw_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidateCaptureGuideIntentInput {
    pub run_id: String,
    pub expected_revision: u64,
    pub expected_mesh_digest: String,
    pub known_distance_mm: f64,
    pub instruction: String,
    pub feature_depth_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidateCaptureGuideIntentResult {
    pub guide: CaptureReconstructionGuide,
    pub state: crate::contracts::CaptureReconstructionGuideState,
    pub base_revision: u64,
    pub expected_revision_matched: bool,
    pub source_digest_matched: bool,
    pub raw_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureReconstructionGuide {
    pub schema_version: u32,
    pub guide_id: String,
    pub revision: u64,
    pub capture_run_id: String,
    pub target_thread_id: String,
    pub target_message_id: Option<String>,
    pub target_source_digest: String,
    pub target_version_id: Option<String>,
    pub source_mesh: CaptureGuideSourceMesh,
    pub calibration: CaptureCalibration,
    pub reconstruction_frame: CaptureReconstructionFrame,
    pub landmarks: Vec<CaptureLandmark>,
    #[serde(default)]
    pub evidence_computation_policy: CaptureEvidenceComputationPolicy,
    #[serde(default)]
    pub surface_neighborhoods: Vec<CaptureSurfaceNeighborhood>,
    #[serde(default)]
    pub primitive_candidates: Vec<CapturePrimitiveCandidate>,
    #[serde(default)]
    pub primitive_hypotheses: Vec<CapturePrimitiveHypothesis>,
    #[serde(default)]
    pub surface_regions: Vec<CaptureSurfaceRegion>,
    #[serde(default)]
    pub region_adjacency: Vec<CaptureRegionAdjacency>,
    #[serde(default)]
    pub reconstructed_profiles: Vec<CaptureReconstructedProfile>,
    pub feature_expectations: Vec<CaptureFeatureExpectation>,
    pub measurements: Vec<CaptureNamedMeasurement>,
    pub axes: Vec<CaptureNamedAxis>,
    pub planes: Vec<CaptureNamedPlane>,
    pub profiles: Vec<CaptureOrderedProfile>,
    pub ignored_regions: Vec<CaptureIgnoredRegion>,
    #[serde(default)]
    pub authored_constraints: Vec<CaptureNamedConstraint>,
    #[serde(default)]
    pub constraint_graph: CaptureConstraintGraph,
    #[serde(default)]
    pub feature_plan_candidates: Vec<CaptureFeaturePlanCandidate>,
    #[serde(default)]
    pub selected_feature_plan_id: Option<String>,
    #[serde(default)]
    pub stage_bypasses: Vec<CaptureStageBypass>,
    #[serde(default)]
    pub reconstruction_readiness: CaptureReconstructionReadiness,
    pub remap_proposals: Vec<CaptureAnchorRemapProposal>,
    pub symmetry_completion: CaptureSymmetryCompletion,
    pub instruction: String,
    pub evidence_views: Vec<CaptureEvidenceView>,
    pub canonical_digest: String,
}

impl CaptureReconstructionGuide {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != CAPTURE_RECONSTRUCTION_GUIDE_SCHEMA_VERSION {
            return Err(format!(
                "Unsupported capture reconstruction guide schema version '{}'.",
                self.schema_version
            ));
        }
        if self.revision == 0 {
            return Err("Capture reconstruction guide revision must be positive.".into());
        }
        require_digest("source mesh artifact", &self.source_mesh.artifact_digest)?;
        require_digest("source mesh content", &self.source_mesh.content_digest)?;
        require_digest("target source", &self.target_source_digest)?;
        validate_vec3("source bounds minimum", self.source_mesh.source_bounds.min)?;
        validate_vec3("source bounds maximum", self.source_mesh.source_bounds.max)?;
        if self.source_mesh.triangle_count == 0 {
            return Err("Capture source mesh has no triangles.".into());
        }
        if !self.calibration.millimetres_per_source_unit.is_finite()
            || self.calibration.millimetres_per_source_unit <= 0.0
        {
            return Err("Calibration scale must be finite and positive.".into());
        }
        if let CaptureCalibrationMethod::TrustedMetricMetadata {
            accepted_by_user: false,
            ..
        } = &self.calibration.method
        {
            return Err("Trusted metric metadata must be explicitly accepted.".into());
        }
        validate_vec3("frame origin", self.reconstruction_frame.origin_mm)?;
        validate_unitish("frame X axis", self.reconstruction_frame.x_axis)?;
        validate_unitish("frame Y axis", self.reconstruction_frame.y_axis)?;
        validate_unitish("frame Z axis", self.reconstruction_frame.z_axis)?;

        let mut all_ids = HashSet::new();
        let landmark_ids: HashSet<&str> = self
            .landmarks
            .iter()
            .map(|landmark| landmark.landmark_id.as_str())
            .collect();
        for landmark in &self.landmarks {
            insert_unique(&mut all_ids, &landmark.landmark_id)?;
            validate_anchor(&landmark.anchor, &self.source_mesh.content_digest)?;
            validate_vec3("landmark local position", landmark.local_position_mm)?;
            validate_unitish("landmark local normal", landmark.local_normal)?;
            if landmark
                .uncertainty_mm
                .is_some_and(|value| !value.is_finite() || value < 0.0)
            {
                return Err(format!(
                    "Landmark '{}' has invalid uncertainty.",
                    landmark.landmark_id
                ));
            }
        }
        if self.surface_neighborhoods.len() > 256 {
            return Err("Capture guide has too many surface neighborhoods.".into());
        }
        validate_positive(
            "surface neighborhood radius",
            self.evidence_computation_policy.neighborhood_radius_mm,
        )?;
        if !(1..=256).contains(&self.evidence_computation_policy.max_neighborhood_triangles) {
            return Err(
                "Capture surface neighborhood triangle budget must be between 1 and 256.".into(),
            );
        }
        for neighborhood in &self.surface_neighborhoods {
            insert_unique(&mut all_ids, &neighborhood.neighborhood_id)?;
            if !landmark_ids.contains(neighborhood.landmark_id.as_str()) {
                return Err(format!(
                    "Surface neighborhood '{}' references missing landmark '{}'.",
                    neighborhood.neighborhood_id, neighborhood.landmark_id
                ));
            }
            if neighborhood.source_mesh_content_digest != self.source_mesh.content_digest {
                return Err(format!(
                    "Surface neighborhood '{}' mesh digest differs from guide source mesh.",
                    neighborhood.neighborhood_id
                ));
            }
            if neighborhood.triangle_indices.is_empty()
                || !neighborhood
                    .triangle_indices
                    .contains(&neighborhood.seed_triangle_index)
                || neighborhood
                    .triangle_indices
                    .iter()
                    .any(|index| *index >= self.source_mesh.triangle_count)
            {
                return Err(format!(
                    "Surface neighborhood '{}' has invalid triangle provenance.",
                    neighborhood.neighborhood_id
                ));
            }
            if neighborhood.vertex_indices.is_empty()
                || neighborhood.sample_count != neighborhood.vertex_indices.len() as u64
            {
                return Err(format!(
                    "Surface neighborhood '{}' has invalid sample provenance.",
                    neighborhood.neighborhood_id
                ));
            }
            if neighborhood.adjacency_edges.iter().any(|edge| {
                edge[0] >= edge[1]
                    || !neighborhood.triangle_indices.contains(&edge[0])
                    || !neighborhood.triangle_indices.contains(&edge[1])
            }) {
                return Err(format!(
                    "Surface neighborhood '{}' has invalid adjacency provenance.",
                    neighborhood.neighborhood_id
                ));
            }
            validate_positive(
                "surface neighborhood radius",
                neighborhood.radius_source_units,
            )?;
            validate_nonnegative(
                "surface neighborhood sampled area",
                neighborhood.sampled_area_source_units_squared,
            )?;
            validate_nonnegative(
                "surface neighborhood radial coverage",
                neighborhood.radial_coverage_ratio,
            )?;
            if neighborhood.radial_coverage_ratio > 1.0 + 1.0e-12 {
                return Err(format!(
                    "Surface neighborhood '{}' radial coverage exceeds one.",
                    neighborhood.neighborhood_id
                ));
            }
            validate_vec3(
                "surface neighborhood centroid",
                neighborhood.centroid_source,
            )?;
            validate_unitish("surface neighborhood mean normal", neighborhood.mean_normal)?;
            validate_nonnegative(
                "surface neighborhood normal spread",
                neighborhood.normal_spread_deg,
            )?;
            if neighborhood.normal_spread_deg > 180.0 {
                return Err(format!(
                    "Surface neighborhood '{}' normal spread exceeds 180 degrees.",
                    neighborhood.neighborhood_id
                ));
            }
            validate_nonnegative(
                "surface neighborhood normal variation RMS",
                neighborhood.normal_variation_rms_deg,
            )?;
            validate_nonnegative(
                "surface neighborhood estimated curvature",
                neighborhood.estimated_curvature_per_source_unit,
            )?;
            validate_nonnegative(
                "surface neighborhood position RMS",
                neighborhood.position_rms_source_units,
            )?;
            validate_nonnegative(
                "surface neighborhood planarity RMS",
                neighborhood.planarity_rms_source_units,
            )?;
            validate_nonnegative(
                "surface neighborhood planarity maximum",
                neighborhood.planarity_max_source_units,
            )?;
            validate_nonnegative(
                "surface neighborhood position uncertainty",
                neighborhood.position_uncertainty_source_units,
            )?;
            if neighborhood.planarity_rms_source_units
                > neighborhood.planarity_max_source_units + 1.0e-12
            {
                return Err(format!(
                    "Surface neighborhood '{}' planarity RMS exceeds maximum.",
                    neighborhood.neighborhood_id
                ));
            }
        }
        for id in &self.reconstruction_frame.source_landmark_ids {
            require_landmark(&landmark_ids, "Reconstruction frame", id)?;
        }
        for measurement in &self.calibration.measurements {
            insert_unique(&mut all_ids, &measurement.measurement_id)?;
            require_landmark(
                &landmark_ids,
                "Calibration measurement",
                &measurement.first_landmark_id,
            )?;
            require_landmark(
                &landmark_ids,
                "Calibration measurement",
                &measurement.second_landmark_id,
            )?;
            if measurement.first_landmark_id == measurement.second_landmark_id {
                return Err(format!(
                    "Calibration measurement '{}' uses coincident endpoints.",
                    measurement.measurement_id
                ));
            }
            validate_positive("known distance", measurement.known_distance_mm)?;
            validate_nonnegative("calibration tolerance", measurement.accepted_tolerance_mm)?;
        }
        let plane_ids: HashSet<&str> = self
            .planes
            .iter()
            .map(|plane| plane.plane_id.as_str())
            .collect();
        for axis in &self.axes {
            insert_unique(&mut all_ids, &axis.axis_id)?;
            if axis.landmark_ids.len() < 2 {
                return Err(format!(
                    "Axis '{}' needs at least two landmarks.",
                    axis.axis_id
                ));
            }
            validate_refs(&landmark_ids, "Axis", &axis.axis_id, &axis.landmark_ids)?;
            validate_vec3("axis origin", axis.origin_mm)?;
            validate_unitish("axis direction", axis.direction)?;
            validate_fit(&axis.fit)?;
        }
        for plane in &self.planes {
            insert_unique(&mut all_ids, &plane.plane_id)?;
            if plane.landmark_ids.len() < 3 {
                return Err(format!(
                    "Plane '{}' needs at least three landmarks.",
                    plane.plane_id
                ));
            }
            validate_refs(&landmark_ids, "Plane", &plane.plane_id, &plane.landmark_ids)?;
            validate_vec3("plane origin", plane.origin_mm)?;
            validate_unitish("plane normal", plane.normal)?;
            validate_fit(&plane.fit)?;
        }
        for profile in &self.profiles {
            insert_unique(&mut all_ids, &profile.profile_id)?;
            let minimum = if profile.kind == CaptureProfileKind::Closed {
                3
            } else {
                2
            };
            if profile.landmark_ids.len() < minimum {
                return Err(format!(
                    "Profile '{}' has invalid point order.",
                    profile.profile_id
                ));
            }
            let mut seen = HashSet::new();
            for id in &profile.landmark_ids {
                if !landmark_ids.contains(id.as_str()) {
                    return Err(format!(
                        "Profile '{}' references missing landmark '{}'.",
                        profile.profile_id, id
                    ));
                }
                if !seen.insert(id) {
                    return Err(format!(
                        "Profile '{}' repeats landmark '{}'.",
                        profile.profile_id, id
                    ));
                }
            }
            if !plane_ids.contains(profile.support_plane_id.as_str()) {
                return Err(format!(
                    "Profile '{}' references missing support plane '{}'.",
                    profile.profile_id, profile.support_plane_id
                ));
            }
        }
        for measurement in &self.measurements {
            insert_unique(&mut all_ids, &measurement.measurement_id)?;
            validate_refs(
                &landmark_ids,
                "Measurement",
                &measurement.measurement_id,
                &measurement.landmark_ids,
            )?;
            if !measurement.value.is_finite() {
                return Err(format!(
                    "Measurement '{}' value must be finite.",
                    measurement.measurement_id
                ));
            }
            if measurement.fit_critical
                && measurement
                    .authored_parameter_name
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .is_empty()
            {
                return Err(format!(
                    "Fit-critical measurement '{}' needs a named authored parameter or constraint.",
                    measurement.measurement_id
                ));
            }
        }
        let candidate_input_ids = all_ids.clone();
        let neighborhood_ids = self
            .surface_neighborhoods
            .iter()
            .map(|item| item.neighborhood_id.as_str())
            .collect::<HashSet<_>>();
        if self.primitive_candidates.len() > 256 {
            return Err("Capture guide has too many primitive candidates.".into());
        }
        for candidate in &self.primitive_candidates {
            insert_unique(&mut all_ids, &candidate.candidate_id)?;
            if candidate.guide_item_ids.is_empty()
                || candidate
                    .guide_item_ids
                    .iter()
                    .any(|id| !candidate_input_ids.contains(id))
            {
                return Err(format!(
                    "Primitive candidate '{}' has invalid guide evidence.",
                    candidate.candidate_id
                ));
            }
            if candidate.neighborhood_ids.is_empty()
                || candidate
                    .neighborhood_ids
                    .iter()
                    .any(|id| !neighborhood_ids.contains(id.as_str()))
            {
                return Err(format!(
                    "Primitive candidate '{}' has invalid surface-neighborhood evidence.",
                    candidate.candidate_id
                ));
            }
            if candidate.support_sample_count == 0 {
                return Err(format!(
                    "Primitive candidate '{}' has no supporting samples.",
                    candidate.candidate_id
                ));
            }
            match candidate.geometry {
                CaptureAnalyticPrimitive::Line {
                    origin_mm,
                    direction,
                } => {
                    validate_vec3("primitive line origin", origin_mm)?;
                    validate_unitish("primitive line direction", direction)?;
                }
                CaptureAnalyticPrimitive::Plane { origin_mm, normal } => {
                    validate_vec3("primitive plane origin", origin_mm)?;
                    validate_unitish("primitive plane normal", normal)?;
                }
                CaptureAnalyticPrimitive::Circle {
                    center_mm,
                    normal,
                    radius_mm,
                } => {
                    validate_vec3("primitive circle center", center_mm)?;
                    validate_unitish("primitive circle normal", normal)?;
                    validate_positive("primitive circle radius", radius_mm)?;
                }
                CaptureAnalyticPrimitive::Cylinder {
                    origin_mm,
                    axis_direction,
                    radius_mm,
                    min_axis_mm,
                    max_axis_mm,
                } => {
                    validate_vec3("primitive cylinder origin", origin_mm)?;
                    validate_unitish("primitive cylinder axis", axis_direction)?;
                    validate_positive("primitive cylinder radius", radius_mm)?;
                    if !min_axis_mm.is_finite()
                        || !max_axis_mm.is_finite()
                        || min_axis_mm >= max_axis_mm
                    {
                        return Err(format!(
                            "Primitive candidate '{}' has invalid cylinder domain.",
                            candidate.candidate_id
                        ));
                    }
                }
                CaptureAnalyticPrimitive::Cone {
                    apex_mm,
                    axis_direction,
                    half_angle_deg,
                    min_axis_mm,
                    max_axis_mm,
                } => {
                    validate_vec3("primitive cone apex", apex_mm)?;
                    validate_unitish("primitive cone axis", axis_direction)?;
                    if !half_angle_deg.is_finite()
                        || half_angle_deg <= 0.0
                        || half_angle_deg >= 90.0
                        || !min_axis_mm.is_finite()
                        || !max_axis_mm.is_finite()
                        || min_axis_mm >= max_axis_mm
                    {
                        return Err(format!(
                            "Primitive candidate '{}' has invalid cone parameters.",
                            candidate.candidate_id
                        ));
                    }
                }
                CaptureAnalyticPrimitive::Sphere {
                    center_mm,
                    radius_mm,
                } => {
                    validate_vec3("primitive sphere center", center_mm)?;
                    validate_positive("primitive sphere radius", radius_mm)?;
                }
            }
            validate_fit(&candidate.fit)?;
        }
        let primitive_candidate_ids = self
            .primitive_candidates
            .iter()
            .map(|candidate| candidate.candidate_id.as_str())
            .collect::<HashSet<_>>();
        let mut hypothesis_ids = HashSet::new();
        for hypothesis in &self.primitive_hypotheses {
            if hypothesis.hypothesis_id.trim().is_empty()
                || !hypothesis_ids.insert(hypothesis.hypothesis_id.as_str())
                || hypothesis.guide_item_ids.is_empty()
                || hypothesis
                    .guide_item_ids
                    .iter()
                    .any(|id| !candidate_input_ids.contains(id))
                || hypothesis.reason.trim().is_empty()
                || hypothesis
                    .domain
                    .minimum
                    .is_some_and(|value| !value.is_finite())
                || hypothesis
                    .domain
                    .maximum
                    .is_some_and(|value| !value.is_finite())
                || matches!(
                    (hypothesis.domain.minimum, hypothesis.domain.maximum),
                    (Some(minimum), Some(maximum)) if minimum >= maximum
                )
            {
                return Err(format!(
                    "Primitive hypothesis '{}' is invalid.",
                    hypothesis.hypothesis_id
                ));
            }
            match hypothesis.status {
                CapturePrimitiveHypothesisStatus::Supported => {
                    if hypothesis
                        .candidate_id
                        .as_ref()
                        .is_none_or(|id| !primitive_candidate_ids.contains(id.as_str()))
                        || hypothesis.fit.is_none()
                    {
                        return Err(format!(
                            "Supported primitive hypothesis '{}' lacks candidate or fit evidence.",
                            hypothesis.hypothesis_id
                        ));
                    }
                }
                CapturePrimitiveHypothesisStatus::Rejected => {
                    if hypothesis.candidate_id.is_some() {
                        return Err(format!(
                            "Rejected primitive hypothesis '{}' names a supported candidate.",
                            hypothesis.hypothesis_id
                        ));
                    }
                }
            }
            if let Some(fit) = &hypothesis.fit {
                validate_fit(fit)?;
            }
            if let Some(robust) = &hypothesis.robust_evidence {
                let mut excluded = HashSet::new();
                if robust.method != "deterministicLeaveOneOut"
                    || robust.excluded_guide_item_ids.len() != 1
                    || robust.excluded_guide_item_ids.iter().any(|id| {
                        !candidate_input_ids.contains(id) || !excluded.insert(id.as_str())
                    })
                {
                    return Err(format!(
                        "Primitive hypothesis '{}' has invalid robust exclusion evidence.",
                        hypothesis.hypothesis_id
                    ));
                }
            }
        }
        let mut covered_triangles = HashSet::new();
        let mut surface_region_ids = HashSet::new();
        if self.surface_regions.len() > 512 {
            return Err("Capture guide has too many surface regions.".into());
        }
        for region in &self.surface_regions {
            insert_unique(&mut all_ids, &region.region_id)?;
            surface_region_ids.insert(region.region_id.as_str());
            if region.source_mesh_content_digest != self.source_mesh.content_digest {
                return Err(format!(
                    "Surface region '{}' mesh digest differs from guide source mesh.",
                    region.region_id
                ));
            }
            if region.triangle_indices.is_empty()
                || region.triangle_indices.iter().any(|triangle| {
                    *triangle >= self.source_mesh.triangle_count
                        || !covered_triangles.insert(*triangle)
                })
            {
                return Err(format!(
                    "Surface region '{}' has missing, duplicate, or invalid triangle coverage.",
                    region.region_id
                ));
            }
            validate_refs(
                &landmark_ids,
                "Surface region",
                &region.region_id,
                &region.landmark_ids,
            )?;
            if region
                .primitive_candidate_ids
                .iter()
                .any(|id| !primitive_candidate_ids.contains(id.as_str()))
            {
                return Err(format!(
                    "Surface region '{}' references missing primitive candidate.",
                    region.region_id
                ));
            }
            validate_nonnegative("surface region area", region.area_source_units_squared)?;
            if region.ignored != (region.kind == CaptureSurfaceRegionKind::IgnoredDamage) {
                return Err(format!(
                    "Surface region '{}' ignored flag and kind disagree.",
                    region.region_id
                ));
            }
        }
        if !self.surface_regions.is_empty()
            && covered_triangles.len() != self.source_mesh.triangle_count as usize
        {
            return Err(format!(
                "Capture surface segmentation covers {} of {} source triangles.",
                covered_triangles.len(),
                self.source_mesh.triangle_count
            ));
        }
        let mut adjacency_pairs = HashSet::new();
        for adjacency in &self.region_adjacency {
            if adjacency.first_region_id >= adjacency.second_region_id
                || !surface_region_ids.contains(adjacency.first_region_id.as_str())
                || !surface_region_ids.contains(adjacency.second_region_id.as_str())
                || !adjacency_pairs.insert((
                    adjacency.first_region_id.as_str(),
                    adjacency.second_region_id.as_str(),
                ))
                || adjacency.shared_edge_count == 0
                || !adjacency.maximum_normal_angle_deg.is_finite()
                || !(0.0..=180.0 + 1.0e-9).contains(&adjacency.maximum_normal_angle_deg)
            {
                return Err("Capture region adjacency is invalid or duplicated.".into());
            }
        }
        let source_profile_ids = self
            .profiles
            .iter()
            .map(|profile| profile.profile_id.as_str())
            .collect::<HashSet<_>>();
        let mut reconstructed_profile_ids = HashSet::new();
        for profile in &self.reconstructed_profiles {
            insert_unique(&mut all_ids, &profile.candidate_id)?;
            reconstructed_profile_ids.insert(profile.candidate_id.as_str());
            if !source_profile_ids.contains(profile.source_profile_id.as_str())
                || !plane_ids.contains(profile.support_plane_id.as_str())
                || profile.segments.is_empty()
                || profile.segments.len() > 256
            {
                return Err(format!(
                    "Reconstructed profile '{}' has invalid source, support plane, or segment count.",
                    profile.candidate_id
                ));
            }
            validate_nonnegative("profile closure error", profile.closure_error_mm)?;
            validate_nonnegative("profile continuity gap", profile.maximum_continuity_gap_mm)?;
            validate_nonnegative(
                "profile support-plane residual",
                profile.support_plane_max_mm,
            )?;
            if profile.closed && profile.closure_error_mm > 1.0e-9 {
                return Err(format!(
                    "Reconstructed profile '{}' claims closure with a non-zero gap.",
                    profile.candidate_id
                ));
            }
            for segment in &profile.segments {
                insert_unique(&mut all_ids, &segment.segment_id)?;
                validate_refs(
                    &landmark_ids,
                    "Profile segment",
                    &segment.segment_id,
                    &segment.source_landmark_ids,
                )?;
                if segment
                    .neighborhood_ids
                    .iter()
                    .any(|id| !neighborhood_ids.contains(id.as_str()))
                    || !segment.parameter_range[0].is_finite()
                    || !segment.parameter_range[1].is_finite()
                    || (segment.parameter_range[0] - segment.parameter_range[1]).abs() <= 1.0e-12
                {
                    return Err(format!(
                        "Profile segment '{}' has invalid evidence or parameter domain.",
                        segment.segment_id
                    ));
                }
                validate_profile_segment_geometry(&segment.geometry)?;
                validate_fit(&segment.fit)?;
            }
        }
        let dimension_ids = self
            .constraint_graph
            .dimensions
            .iter()
            .map(|dimension| dimension.dimension_id.as_str())
            .collect::<HashSet<_>>();
        let mut constraint_ids = HashSet::new();
        for dimension in &self.constraint_graph.dimensions {
            if !dimension.value.is_finite()
                || dimension.unit.trim().is_empty()
                || dimension.fit_critical
                    && dimension
                        .parameter_name
                        .as_deref()
                        .unwrap_or("")
                        .trim()
                        .is_empty()
            {
                return Err(format!(
                    "Constraint-graph dimension '{}' is invalid.",
                    dimension.dimension_id
                ));
            }
            validate_refs(
                &landmark_ids,
                "Constraint-graph dimension",
                &dimension.dimension_id,
                &dimension.landmark_ids,
            )?;
        }
        for constraint in self
            .authored_constraints
            .iter()
            .chain(&self.constraint_graph.relations)
        {
            if constraint.constraint_id.trim().is_empty()
                || constraint.entity_ids.is_empty()
                || !constraint.tolerance.is_finite()
                || constraint.tolerance < 0.0
                || constraint
                    .residual
                    .is_some_and(|value| !value.is_finite() || value < 0.0)
            {
                return Err(format!(
                    "Named constraint '{}' is invalid.",
                    constraint.constraint_id
                ));
            }
            constraint_ids.insert(constraint.constraint_id.as_str());
        }
        if !self.constraint_graph.dimensions.is_empty()
            || !self.constraint_graph.relations.is_empty()
        {
            require_digest("constraint graph", &self.constraint_graph.content_digest)?;
        }
        let axis_ids = self
            .axes
            .iter()
            .map(|axis| axis.axis_id.as_str())
            .collect::<HashSet<_>>();
        let mut plan_ids = HashSet::new();
        for plan in &self.feature_plan_candidates {
            insert_unique(&mut all_ids, &plan.plan_id)?;
            plan_ids.insert(plan.plan_id.as_str());
            if plan.status != CaptureFeaturePlanStatus::Rejected && plan.operations.is_empty()
                || !plan.score.is_finite()
                || !(0.0..=1.0).contains(&plan.score)
                || plan.status == CaptureFeaturePlanStatus::Supported
                    && !plan.rejecting_evidence.is_empty()
            {
                return Err(format!("Feature plan '{}' is invalid.", plan.plan_id));
            }
            for operation in &plan.operations {
                validate_feature_operation(
                    operation,
                    &reconstructed_profile_ids,
                    &dimension_ids,
                    &axis_ids,
                    &plane_ids,
                )?;
            }
        }
        for plan in &self.feature_plan_candidates {
            for operation in &plan.operations {
                let referenced = match operation {
                    CaptureFeatureOperation::BooleanUnion { operand_plan_ids } => {
                        operand_plan_ids.clone()
                    }
                    CaptureFeatureOperation::BooleanDifference {
                        base_plan_id,
                        cutter_plan_ids,
                    } => std::iter::once(base_plan_id.clone())
                        .chain(cutter_plan_ids.iter().cloned())
                        .collect(),
                    _ => continue,
                };
                if referenced.iter().any(|id| {
                    id == &plan.plan_id
                        || !plan_ids.contains(id.as_str())
                        || self.feature_plan_candidates.iter().any(|candidate| {
                            candidate.plan_id == *id
                                && candidate.operations.iter().any(|operation| {
                                    matches!(
                                        operation,
                                        CaptureFeatureOperation::BooleanUnion { .. }
                                            | CaptureFeatureOperation::BooleanDifference { .. }
                                    )
                                })
                        })
                }) {
                    return Err(format!(
                        "Boolean feature plan '{}' has missing, self, or nested boolean operands.",
                        plan.plan_id
                    ));
                }
            }
        }
        if self
            .selected_feature_plan_id
            .as_ref()
            .is_some_and(|id| !plan_ids.contains(id.as_str()))
        {
            return Err("Selected feature plan does not exist.".into());
        }
        for bypass in &self.stage_bypasses {
            if bypass.affected_evidence_ids.is_empty()
                || bypass.explicit_constraint_ids.is_empty()
                || bypass.rationale.trim().is_empty()
                || !bypass.accepted_by_user
                || bypass
                    .explicit_constraint_ids
                    .iter()
                    .any(|id| !constraint_ids.contains(id.as_str()))
            {
                return Err(format!(
                    "Reconstruction stage bypass '{:?}' lacks confirmed explicit constraints.",
                    bypass.stage
                ));
            }
        }
        if self.reconstruction_readiness.ready
            && (!self.reconstruction_readiness.missing_stages.is_empty()
                || !self.reconstruction_readiness.ambiguous_stages.is_empty()
                || self.reconstruction_readiness.selected_feature_plan_id
                    != self.selected_feature_plan_id
                || self
                    .selected_feature_plan_id
                    .as_ref()
                    .is_none_or(|selected| {
                        self.feature_plan_candidates.iter().all(|candidate| {
                            candidate.plan_id != *selected
                                || candidate.status != CaptureFeaturePlanStatus::Supported
                        })
                    }))
        {
            return Err(
                    "Capture reconstruction readiness claims green without one supported selected plan."
                        .into(),
                );
        }
        let evidence_ids = all_ids.clone();
        for expectation in &self.feature_expectations {
            insert_unique(&mut all_ids, &expectation.expectation_id)?;
            if expectation.guide_item_ids.is_empty() {
                return Err(format!(
                    "Feature expectation '{}' has no guide items.",
                    expectation.expectation_id
                ));
            }
            for guide_item_id in &expectation.guide_item_ids {
                if !evidence_ids.contains(guide_item_id) {
                    return Err(format!(
                        "Feature expectation '{}' references missing guide item '{}'.",
                        expectation.expectation_id, guide_item_id
                    ));
                }
            }
            validate_expected_topology(expectation)?;
        }
        for region in &self.ignored_regions {
            insert_unique(&mut all_ids, &region.region_id)?;
            validate_refs(
                &landmark_ids,
                "Ignored region",
                &region.region_id,
                &region.landmark_ids,
            )?;
        }
        for proposal in &self.remap_proposals {
            insert_unique(&mut all_ids, &proposal.proposal_id)?;
            require_landmark(&landmark_ids, "Remap proposal", &proposal.landmark_id)?;
            validate_nonnegative("remap residual", proposal.residual_mm)?;
        }
        match &self.symmetry_completion {
            CaptureSymmetryCompletion::None => {}
            CaptureSymmetryCompletion::Half { plane_id } => {
                if !plane_ids.contains(plane_id.as_str()) {
                    return Err(format!(
                        "Half completion references missing symmetry plane '{plane_id}'."
                    ));
                }
            }
            CaptureSymmetryCompletion::Quarter {
                first_plane_id,
                second_plane_id,
            } => {
                if first_plane_id == second_plane_id {
                    return Err("Quarter completion needs two distinct symmetry planes.".into());
                }
                for plane_id in [first_plane_id, second_plane_id] {
                    if !plane_ids.contains(plane_id.as_str()) {
                        return Err(format!(
                            "Quarter completion references missing symmetry plane '{plane_id}'."
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn compute_canonical_digest(&self) -> Result<String, String> {
        let mut canonical = self.clone();
        canonical.canonical_digest.clear();
        canonical
            .landmarks
            .sort_by(|a, b| a.landmark_id.cmp(&b.landmark_id));
        canonical
            .surface_neighborhoods
            .sort_by(|a, b| a.neighborhood_id.cmp(&b.neighborhood_id));
        canonical
            .primitive_candidates
            .sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));
        canonical
            .primitive_hypotheses
            .sort_by(|a, b| a.hypothesis_id.cmp(&b.hypothesis_id));
        canonical
            .surface_regions
            .sort_by(|a, b| a.region_id.cmp(&b.region_id));
        canonical.region_adjacency.sort_by(|a, b| {
            (&a.first_region_id, &a.second_region_id)
                .cmp(&(&b.first_region_id, &b.second_region_id))
        });
        canonical
            .reconstructed_profiles
            .sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));
        canonical
            .feature_expectations
            .sort_by(|a, b| a.expectation_id.cmp(&b.expectation_id));
        canonical
            .measurements
            .sort_by(|a, b| a.measurement_id.cmp(&b.measurement_id));
        canonical.axes.sort_by(|a, b| a.axis_id.cmp(&b.axis_id));
        canonical.planes.sort_by(|a, b| a.plane_id.cmp(&b.plane_id));
        canonical
            .profiles
            .sort_by(|a, b| a.profile_id.cmp(&b.profile_id));
        canonical
            .ignored_regions
            .sort_by(|a, b| a.region_id.cmp(&b.region_id));
        canonical
            .authored_constraints
            .sort_by(|a, b| a.constraint_id.cmp(&b.constraint_id));
        canonical
            .constraint_graph
            .dimensions
            .sort_by(|a, b| a.dimension_id.cmp(&b.dimension_id));
        canonical
            .constraint_graph
            .relations
            .sort_by(|a, b| a.constraint_id.cmp(&b.constraint_id));
        canonical
            .feature_plan_candidates
            .sort_by(|a, b| a.plan_id.cmp(&b.plan_id));
        canonical
            .remap_proposals
            .sort_by(|a, b| a.proposal_id.cmp(&b.proposal_id));
        canonical
            .evidence_views
            .sort_by(|a, b| a.view_id.cmp(&b.view_id));
        let mut canonical_value = serde_json::to_value(&canonical)
            .map_err(|error| format!("Capture guide canonical value failed: {error}"))?;
        canonicalize_json_numbers(&mut canonical_value);
        let bytes = serde_json::to_vec(&canonical_value)
            .map_err(|error| format!("Capture guide canonical serialization failed: {error}"))?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}

fn canonicalize_json_numbers(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                canonicalize_json_numbers(item);
            }
        }
        serde_json::Value::Object(fields) => {
            for item in fields.values_mut() {
                canonicalize_json_numbers(item);
            }
        }
        serde_json::Value::Number(number) => {
            let canonical = if let Some(value) = number.as_i64() {
                format!("i64:{value}")
            } else if let Some(value) = number.as_u64() {
                format!("u64:{value}")
            } else {
                let value = number.as_f64().unwrap_or(0.0);
                let value = if value == 0.0 { 0.0 } else { value };
                // serde_json may move a computed f64 by one ULP on persistence
                // round-trip. Twelve decimal digits exceed scan evidence precision
                // while keeping the canonical digest stable across that boundary.
                format!("f64:{value:.12e}")
            };
            *value = serde_json::Value::String(canonical);
        }
        _ => {}
    }
}

fn insert_unique(ids: &mut HashSet<String>, id: &str) -> Result<(), String> {
    if id.trim().is_empty() {
        return Err("Guide item ID must not be empty.".into());
    }
    if !ids.insert(id.to_string()) {
        return Err(format!("Duplicate guide item ID '{id}'."));
    }
    Ok(())
}

fn require_landmark(ids: &HashSet<&str>, owner: &str, id: &str) -> Result<(), String> {
    if !ids.contains(id) {
        return Err(format!("{owner} references missing landmark '{id}'."));
    }
    Ok(())
}

fn validate_refs(
    ids: &HashSet<&str>,
    kind: &str,
    owner_id: &str,
    refs: &[String],
) -> Result<(), String> {
    for id in refs {
        if !ids.contains(id.as_str()) {
            return Err(format!(
                "{kind} '{owner_id}' references missing landmark '{id}'."
            ));
        }
    }
    Ok(())
}

fn require_digest(label: &str, digest: &str) -> Result<(), String> {
    if !digest.starts_with("sha256:") || digest.len() <= "sha256:".len() {
        return Err(format!("Capture {label} digest is invalid."));
    }
    Ok(())
}

fn validate_anchor(anchor: &CaptureSurfaceAnchor, digest: &str) -> Result<(), String> {
    if anchor.source_mesh_content_digest != digest {
        return Err("Landmark source mesh content digest differs from guide source mesh.".into());
    }
    let sum: f64 = anchor.barycentric.iter().sum();
    if anchor
        .barycentric
        .iter()
        .any(|value| !value.is_finite() || *value < -1.0e-9 || *value > 1.0 + 1.0e-9)
        || (sum - 1.0).abs() > 1.0e-8
    {
        return Err("Capture surface anchor barycentric coordinates are invalid.".into());
    }
    validate_vec3("capture anchor source position", anchor.source_position)?;
    validate_unitish("capture anchor source normal", anchor.source_normal)
}

fn validate_vec3(label: &str, value: [f64; 3]) -> Result<(), String> {
    if value
        .iter()
        .any(|component| !component.is_finite() || component.abs() > MAX_ABS_COORDINATE)
    {
        return Err(format!("{label} must contain finite bounded coordinates."));
    }
    Ok(())
}

fn validate_unitish(label: &str, value: [f64; 3]) -> Result<(), String> {
    validate_vec3(label, value)?;
    let norm = value
        .iter()
        .map(|component| component * component)
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() || norm <= 1.0e-12 || (norm - 1.0).abs() > 1.0e-6 {
        return Err(format!("{label} must be a finite unit vector."));
    }
    Ok(())
}

fn validate_positive(label: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("{label} must be finite and positive."));
    }
    Ok(())
}

fn validate_nonnegative(label: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || value < 0.0 {
        return Err(format!("{label} must be finite and non-negative."));
    }
    Ok(())
}

fn validate_fit(fit: &CaptureFitResidual) -> Result<(), String> {
    validate_nonnegative("fit RMS residual", fit.rms_mm)?;
    validate_nonnegative("fit maximum residual", fit.max_mm)?;
    validate_nonnegative("fit tolerance", fit.tolerance_mm)?;
    if fit.rms_mm > fit.max_mm {
        return Err("Fit RMS residual cannot exceed maximum residual.".into());
    }
    Ok(())
}

fn validate_profile_segment_geometry(
    geometry: &CaptureProfileSegmentGeometry,
) -> Result<(), String> {
    match geometry {
        CaptureProfileSegmentGeometry::Line { start_mm, end_mm } => {
            validate_vec3("profile line start", *start_mm)?;
            validate_vec3("profile line end", *end_mm)?;
            if start_mm == end_mm {
                return Err("Profile line endpoints are coincident.".into());
            }
        }
        CaptureProfileSegmentGeometry::Arc {
            center_mm,
            normal,
            radius_mm,
            start_angle_deg,
            end_angle_deg,
        } => {
            validate_vec3("profile arc center", *center_mm)?;
            validate_unitish("profile arc normal", *normal)?;
            validate_positive("profile arc radius", *radius_mm)?;
            if !start_angle_deg.is_finite()
                || !end_angle_deg.is_finite()
                || (*end_angle_deg - *start_angle_deg).abs() <= 1.0e-12
            {
                return Err("Profile arc has invalid parameter domain.".into());
            }
        }
        CaptureProfileSegmentGeometry::Circle {
            center_mm,
            normal,
            radius_mm,
        } => {
            validate_vec3("profile circle center", *center_mm)?;
            validate_unitish("profile circle normal", *normal)?;
            validate_positive("profile circle radius", *radius_mm)?;
        }
        CaptureProfileSegmentGeometry::Spline {
            degree,
            control_points_mm,
            knots,
        } => {
            if !(2..=5).contains(degree)
                || control_points_mm.len() < *degree as usize + 1
                || control_points_mm.len() > 64
                || knots.len() != control_points_mm.len() + *degree as usize + 1
                || knots.iter().any(|value| !value.is_finite())
                || knots.windows(2).any(|pair| pair[0] > pair[1])
            {
                return Err("Profile spline has invalid degree, controls, or knot domain.".into());
            }
            for point in control_points_mm {
                validate_vec3("profile spline control point", *point)?;
            }
        }
    }
    Ok(())
}

fn validate_feature_operation(
    operation: &CaptureFeatureOperation,
    profile_ids: &HashSet<&str>,
    dimension_ids: &HashSet<&str>,
    axis_ids: &HashSet<&str>,
    plane_ids: &HashSet<&str>,
) -> Result<(), String> {
    match operation {
        CaptureFeatureOperation::Extrude {
            profile_candidate_id,
            distance_dimension_id,
        } => {
            if !profile_ids.contains(profile_candidate_id.as_str())
                || !dimension_ids.contains(distance_dimension_id.as_str())
            {
                return Err("Extrude feature plan has missing profile or named dimension.".into());
            }
        }
        CaptureFeatureOperation::Revolve {
            profile_candidate_id,
            axis_id,
            angle_deg,
        } => {
            if !profile_ids.contains(profile_candidate_id.as_str())
                || !axis_ids.contains(axis_id.as_str())
                || !angle_deg.is_finite()
                || *angle_deg <= 0.0
                || *angle_deg > 360.0
            {
                return Err("Revolve feature plan has invalid profile, axis, or angle.".into());
            }
        }
        CaptureFeatureOperation::Sweep {
            profile_candidate_id,
            path_id,
        } => {
            if !profile_ids.contains(profile_candidate_id.as_str())
                || !axis_ids.contains(path_id.as_str())
            {
                return Err("Sweep feature plan has missing profile or path.".into());
            }
        }
        CaptureFeatureOperation::Mirror { plane_id } => {
            if !plane_ids.contains(plane_id.as_str()) {
                return Err("Mirror feature plan references missing plane.".into());
            }
        }
        CaptureFeatureOperation::BooleanUnion { operand_plan_ids } => {
            if operand_plan_ids.len() < 2 || operand_plan_ids.iter().any(|id| id.trim().is_empty())
            {
                return Err("Boolean-union feature plan needs two named operands.".into());
            }
        }
        CaptureFeatureOperation::BooleanDifference {
            base_plan_id,
            cutter_plan_ids,
        } => {
            if base_plan_id.trim().is_empty()
                || cutter_plan_ids.is_empty()
                || cutter_plan_ids.iter().any(|id| id.trim().is_empty())
            {
                return Err("Boolean-difference feature plan needs named base and cutters.".into());
            }
        }
    }
    Ok(())
}

fn validate_expected_topology(expectation: &CaptureFeatureExpectation) -> Result<(), String> {
    let valid = match expectation.expected_geometry_kind {
        CaptureExpectedGeometryKind::Point => {
            expectation.required_brep_topology_kind == CaptureRequiredBrepTopologyKind::Vertex
        }
        CaptureExpectedGeometryKind::Curve => {
            expectation.required_brep_topology_kind == CaptureRequiredBrepTopologyKind::Edge
        }
        CaptureExpectedGeometryKind::Plane => {
            expectation.required_brep_topology_kind == CaptureRequiredBrepTopologyKind::Face
        }
        CaptureExpectedGeometryKind::Cylinder => matches!(
            expectation.required_brep_topology_kind,
            CaptureRequiredBrepTopologyKind::Face | CaptureRequiredBrepTopologyKind::Edge
        ),
        CaptureExpectedGeometryKind::Profile => {
            expectation.required_brep_topology_kind == CaptureRequiredBrepTopologyKind::OrderedEdges
        }
    };
    if !valid {
        return Err(format!(
            "Feature expectation '{}' has incompatible analytic geometry and BRep topology kinds.",
            expectation.expectation_id
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_contract_serializes_camel_case_and_round_trips() {
        let guide = CaptureReconstructionGuide::test_fixture();
        let json = serde_json::to_value(&guide).expect("serialize guide");

        assert_eq!(
            json["schemaVersion"],
            CAPTURE_RECONSTRUCTION_GUIDE_SCHEMA_VERSION
        );
        assert_eq!(json["guideId"], "guide-1");
        assert_eq!(json["sourceMesh"]["contentDigest"], "sha256:mesh");
        assert_eq!(json["landmarks"][0]["anchor"]["triangleIndex"], 0);
        assert_eq!(
            json["featureExpectations"][0]["expectedGeometryKind"],
            "plane"
        );
        assert_eq!(
            json["featureExpectations"][0]["requiredBrepTopologyKind"],
            "face"
        );

        let decoded: CaptureReconstructionGuide =
            serde_json::from_value(json).expect("deserialize guide");
        assert_eq!(decoded, guide);
    }

    #[test]
    fn capture_guide_edit_intent_serializes_tag_and_fields_as_camel_case() {
        let edit = CaptureGuideEditIntent::ConfigureProfile {
            profile_id: "profile-1".into(),
            label: "outline".into(),
            profile_kind: CaptureProfileKind::Closed,
            operation_hint: CaptureProfileOperationHint::Extrude,
            support_plane_id: "plane-1".into(),
            feature_label: Some("body".into()),
            fit_role: None,
        };

        let json = serde_json::to_value(&edit).expect("serialize capture guide edit");

        assert_eq!(json["kind"], "configureProfile");
        assert_eq!(json["profileId"], "profile-1");
        assert_eq!(json["profileKind"], "closed");
        assert_eq!(json["operationHint"], "extrude");
        assert!(json.get("profile_id").is_none());
    }

    #[test]
    fn feature_expectation_rejects_missing_evidence_item() {
        let mut guide = CaptureReconstructionGuide::test_fixture();
        guide.feature_expectations[0].guide_item_ids = vec!["missing-axis".into()];

        let error = guide.validate().expect_err("missing guide evidence");

        assert!(
            error.contains("references missing guide item 'missing-axis'"),
            "{error}"
        );
    }

    #[test]
    fn typed_geometry_refs_reject_cross_kind_fields() {
        let cross_kind = serde_json::json!({
            "kind": "captureAnchor",
            "sourceMeshContentDigest": "sha256:mesh",
            "triangleIndex": 0,
            "barycentric": [1.0, 0.0, 0.0],
            "renderArtifactDigest": "sha256:render",
            "vertexIndex": 1
        });

        let error = serde_json::from_value::<TypedGeometryRef>(cross_kind)
            .expect_err("cross-kind fields must fail")
            .to_string();
        assert!(error.contains("unknown field"), "{error}");
    }

    #[test]
    fn guided_commit_result_is_strict_camel_case_and_bounds_assumption_evidence() {
        let result: CaptureGuidedCommitResult = serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "requestId": "capture-guide:sha256:req",
            "guideCanonicalDigest": "sha256:guide",
            "unresolvedAssumptions": ["unknown hidden bore"],
            "inferredRegions": ["quarter completed by X/Y symmetry"]
        }))
        .expect("guided result");
        result.validate().expect("bounded result");
        assert_eq!(result.unresolved_assumptions.len(), 1);
        assert!(
            serde_json::from_value::<CaptureGuidedCommitResult>(serde_json::json!({
                "schemaVersion": 1,
                "requestId": "capture-guide:sha256:req",
                "guideCanonicalDigest": "sha256:guide",
                "unresolvedAssumptions": [],
                "inferredRegions": [],
                "unexpected": true
            }))
            .is_err()
        );

        let mut oversized = result;
        oversized.unresolved_assumptions = (0..65).map(|index| format!("a-{index}")).collect();
        assert!(oversized
            .validate()
            .expect_err("bounded assumptions")
            .contains("exceeds bounded evidence"));
    }

    #[test]
    fn guide_validation_rejects_unknown_role_duplicate_ids_and_bad_profile_refs() {
        let mut json = serde_json::to_value(CaptureReconstructionGuide::test_fixture()).unwrap();
        json["landmarks"][0]["role"] = serde_json::json!("mystery");
        let error = serde_json::from_value::<CaptureReconstructionGuide>(json)
            .expect_err("unknown role must fail")
            .to_string();
        assert!(error.contains("unknown variant `mystery`"), "{error}");

        let mut guide = CaptureReconstructionGuide::test_fixture();
        guide.landmarks.push(guide.landmarks[0].clone());
        let error = guide.validate().expect_err("duplicate ID must fail");
        assert_eq!(error, "Duplicate guide item ID 'landmark-1'.");

        let mut guide = CaptureReconstructionGuide::test_fixture();
        guide.profiles[0].landmark_ids.push("missing".into());
        let error = guide
            .validate()
            .expect_err("missing profile reference must fail");
        assert_eq!(
            error,
            "Profile 'profile-1' references missing landmark 'missing'."
        );
    }

    #[test]
    fn guide_rejects_surface_neighborhood_outside_landmark_and_mesh_identity() {
        let mut guide = CaptureReconstructionGuide::test_fixture();
        guide.surface_neighborhoods = vec![CaptureSurfaceNeighborhood {
            neighborhood_id: "neighborhood:missing".into(),
            landmark_id: "missing".into(),
            source_mesh_content_digest: "sha256:other-mesh".into(),
            seed_triangle_index: 0,
            triangle_indices: vec![0],
            adjacency_edges: vec![],
            vertex_indices: vec![0, 1, 2],
            sample_count: 3,
            radius_source_units: 1.0,
            sampled_area_source_units_squared: 0.5,
            radial_coverage_ratio: 1.0,
            centroid_source: [0.0, 0.0, 0.0],
            mean_normal: [0.0, 0.0, 1.0],
            normal_spread_deg: 0.0,
            normal_variation_rms_deg: 0.0,
            estimated_curvature_per_source_unit: 0.0,
            position_rms_source_units: 0.1,
            planarity_rms_source_units: 0.0,
            planarity_max_source_units: 0.0,
            position_uncertainty_source_units: 0.0,
            reached_mesh_boundary: false,
            truncated_by_budget: false,
        }];

        assert_eq!(
            guide.validate().unwrap_err(),
            "Surface neighborhood 'neighborhood:missing' references missing landmark 'missing'."
        );
        guide.surface_neighborhoods[0].landmark_id = "landmark-1".into();
        assert_eq!(
            guide.validate().unwrap_err(),
            "Surface neighborhood 'neighborhood:missing' mesh digest differs from guide source mesh."
        );
    }

    #[test]
    fn canonical_digest_ignores_unordered_landmark_storage_but_preserves_profile_order() {
        let mut a = CaptureReconstructionGuide::test_fixture();
        let mut b = a.clone();
        b.landmarks.reverse();
        assert_eq!(
            a.compute_canonical_digest().unwrap(),
            b.compute_canonical_digest().unwrap()
        );

        a.profiles[0].landmark_ids.reverse();
        assert_ne!(
            a.compute_canonical_digest().unwrap(),
            b.compute_canonical_digest().unwrap()
        );
    }

    impl CaptureReconstructionGuide {
        pub(crate) fn test_fixture() -> Self {
            let anchor = |triangle_index: u64, position: [f64; 3]| CaptureSurfaceAnchor {
                source_mesh_content_digest: "sha256:mesh".into(),
                triangle_index,
                barycentric: [1.0, 0.0, 0.0],
                source_position: position,
                source_normal: [0.0, 0.0, 1.0],
            };
            let landmark = |id: &str, triangle_index: u64, position: [f64; 3]| CaptureLandmark {
                landmark_id: id.into(),
                label: id.into(),
                role: CaptureLandmarkRole::ProfileVertex,
                anchor: anchor(triangle_index, position),
                local_position_mm: position,
                local_normal: [0.0, 0.0, 1.0],
                uncertainty_mm: None,
            };
            Self {
                schema_version: CAPTURE_RECONSTRUCTION_GUIDE_SCHEMA_VERSION,
                guide_id: "guide-1".into(),
                revision: 1,
                capture_run_id: "run-1".into(),
                target_thread_id: "thread-1".into(),
                target_message_id: Some("message-1".into()),
                target_source_digest: "sha256:target".into(),
                target_version_id: Some("message-1".into()),
                source_mesh: CaptureGuideSourceMesh {
                    artifact_digest: "sha256:artifact".into(),
                    content_digest: "sha256:mesh".into(),
                    selection: CaptureMeshSelection::Raw,
                    crop_digest: None,
                    triangle_count: 3,
                    source_bounds: CaptureSourceBounds {
                        min: [0.0, 0.0, 0.0],
                        max: [1.0, 1.0, 0.0],
                    },
                },
                calibration: CaptureCalibration {
                    source_units: "sourceUnit".into(),
                    millimetres_per_source_unit: 1.0,
                    method: CaptureCalibrationMethod::KnownDistance,
                    measurements: vec![],
                    residual_mm: 0.0,
                },
                reconstruction_frame: CaptureReconstructionFrame {
                    origin_mm: [0.0, 0.0, 0.0],
                    x_axis: [1.0, 0.0, 0.0],
                    y_axis: [0.0, 1.0, 0.0],
                    z_axis: [0.0, 0.0, 1.0],
                    source_landmark_ids: vec![
                        "landmark-1".into(),
                        "landmark-2".into(),
                        "landmark-3".into(),
                    ],
                },
                landmarks: vec![
                    landmark("landmark-1", 0, [0.0, 0.0, 0.0]),
                    landmark("landmark-2", 1, [1.0, 0.0, 0.0]),
                    landmark("landmark-3", 2, [0.0, 1.0, 0.0]),
                ],
                evidence_computation_policy: CaptureEvidenceComputationPolicy::default(),
                surface_neighborhoods: vec![],
                primitive_candidates: vec![],
                primitive_hypotheses: vec![],
                surface_regions: vec![],
                region_adjacency: vec![],
                reconstructed_profiles: vec![],
                feature_expectations: vec![CaptureFeatureExpectation {
                    expectation_id: "expectation-1".into(),
                    guide_item_ids: vec!["plane-1".into()],
                    label: "support face".into(),
                    expected_geometry_kind: CaptureExpectedGeometryKind::Plane,
                    required_brep_topology_kind: CaptureRequiredBrepTopologyKind::Face,
                    cardinality: CaptureSelectorCardinality::One,
                    part_id: "part-1".into(),
                    instance_path: None,
                    expected_authored_selector: CaptureAuthoredSelector::Tag {
                        name: "support-face".into(),
                    },
                    required_for_acceptance: true,
                    position_tolerance_mm: Some(0.1),
                    normal_tolerance_deg: Some(1.0),
                    radial_tolerance_mm: None,
                }],
                measurements: vec![],
                axes: vec![],
                planes: vec![CaptureNamedPlane {
                    plane_id: "plane-1".into(),
                    label: "support".into(),
                    role: CapturePlaneRole::Support,
                    landmark_ids: vec![
                        "landmark-1".into(),
                        "landmark-2".into(),
                        "landmark-3".into(),
                    ],
                    origin_mm: [0.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    fit: CaptureFitResidual {
                        rms_mm: 0.0,
                        max_mm: 0.0,
                        tolerance_mm: 0.1,
                    },
                }],
                profiles: vec![CaptureOrderedProfile {
                    profile_id: "profile-1".into(),
                    label: "outline".into(),
                    kind: CaptureProfileKind::Closed,
                    support_plane_id: "plane-1".into(),
                    landmark_ids: vec![
                        "landmark-1".into(),
                        "landmark-2".into(),
                        "landmark-3".into(),
                    ],
                    operation_hint: CaptureProfileOperationHint::Extrude,
                    feature_label: None,
                    fit_role: None,
                }],
                ignored_regions: vec![],
                authored_constraints: vec![],
                constraint_graph: CaptureConstraintGraph::default(),
                feature_plan_candidates: vec![],
                selected_feature_plan_id: None,
                stage_bypasses: vec![],
                reconstruction_readiness: CaptureReconstructionReadiness::default(),
                remap_proposals: vec![],
                symmetry_completion: CaptureSymmetryCompletion::None,
                instruction: "Build symmetric insert.".into(),
                evidence_views: vec![],
                canonical_digest: String::new(),
            }
        }
    }
}
