use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::contracts::{
    validate_sketch_definition, AppError, AppResult, ArtifactBundle,
    BrepHiddenLineProjectionRequest, BrepHiddenLineProjectionResponse,
    BrepHiddenLineProjectionView, SketchBrepCandidateRequest, SketchBrepCandidateResponse,
    SketchDefinition, SketchDocument, SketchDraftOperationKind, SketchDraftRequest,
    SketchDraftSource, SketchPreviewHullRequest, SketchPrimitive, SketchPrimitiveKind,
    SketchValidationIssue, SketchValidationIssueKind, SketchView,
};
use crate::models::{AppState, PathResolver};

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SketchPreviewTarget {
    pub target_id: String,
    pub part_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SketchPreviewMode {
    Manual,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SketchPreviewSubmissionRequest {
    pub target: SketchPreviewTarget,
    pub document: SketchDocument,
    pub mode: SketchPreviewMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SketchPreviewSubmissionStatus {
    Completed,
    Failed,
    Superseded,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SketchPreviewRenderer {
    Draft,
    PreviewHull,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SketchPreviewFailureStage {
    Validation,
    Render,
    CandidateAnalysis,
    HiddenLineProjection,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SketchBrepAutoRepairEvidence {
    pub primitive_id: String,
    pub view: crate::contracts::SketchView,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SketchPreviewSubmissionPacket {
    pub preview_id: String,
    pub target: SketchPreviewTarget,
    pub mode: SketchPreviewMode,
    pub status: SketchPreviewSubmissionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renderer: Option<SketchPreviewRenderer>,
    pub document: SketchDocument,
    pub repair_attempts: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repair_evidence: Vec<SketchBrepAutoRepairEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_source: Option<SketchDraftSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_bundle: Option<ArtifactBundle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_response: Option<SketchBrepCandidateResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden_line_response: Option<BrepHiddenLineProjectionResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_stage: Option<SketchPreviewFailureStage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<AppError>,
}

#[derive(Default)]
pub struct SketchPreviewRunRegistry {
    active_by_target: Mutex<HashMap<String, String>>,
}

impl SketchPreviewRunRegistry {
    pub fn begin(&self, target_id: &str) -> String {
        let preview_id = uuid::Uuid::new_v4().to_string();
        self.active_by_target
            .lock()
            .expect("sketch preview run registry lock")
            .insert(target_id.to_string(), preview_id.clone());
        preview_id
    }

    pub fn is_current(&self, target_id: &str, preview_id: &str) -> bool {
        self.active_by_target
            .lock()
            .expect("sketch preview run registry lock")
            .get(target_id)
            .is_some_and(|active| active == preview_id)
    }
}

trait SketchPreviewPipeline {
    fn render(
        &self,
        renderer: SketchPreviewRenderer,
        target: &SketchPreviewTarget,
        document: &SketchDocument,
    ) -> AppResult<(SketchDraftSource, ArtifactBundle)>;

    fn analyze_candidates(
        &self,
        request: SketchBrepCandidateRequest,
    ) -> AppResult<SketchBrepCandidateResponse>;

    fn hidden_lines(
        &self,
        request: BrepHiddenLineProjectionRequest,
    ) -> AppResult<BrepHiddenLineProjectionResponse>;
}

struct RuntimeSketchPreviewPipeline<'a> {
    state: &'a AppState,
    app: &'a dyn PathResolver,
}

impl SketchPreviewPipeline for RuntimeSketchPreviewPipeline<'_> {
    fn render(
        &self,
        renderer: SketchPreviewRenderer,
        target: &SketchPreviewTarget,
        document: &SketchDocument,
    ) -> AppResult<(SketchDraftSource, ArtifactBundle)> {
        match renderer {
            SketchPreviewRenderer::Draft => {
                let sketch = draft_sketch(document)?;
                crate::sketch_draft_runtime::generate_sketch_draft_preview(
                    SketchDraftRequest {
                        part_id: target.part_id.clone(),
                        sketch,
                        operation: SketchDraftOperationKind::Extrude,
                        amount: 12.0,
                        symmetric: false,
                    },
                    self.app,
                )
            }
            SketchPreviewRenderer::PreviewHull => {
                crate::sketch_draft_runtime::generate_sketch_preview_hull(
                    SketchPreviewHullRequest {
                        part_id: target.part_id.clone(),
                        document: document.clone(),
                        fallback_depth: 12.0,
                    },
                    self.app,
                )
            }
        }
    }

    fn analyze_candidates(
        &self,
        request: SketchBrepCandidateRequest,
    ) -> AppResult<SketchBrepCandidateResponse> {
        crate::sketch_draft_runtime::analyze_sketch_brep_candidates(request)
    }

    fn hidden_lines(
        &self,
        request: BrepHiddenLineProjectionRequest,
    ) -> AppResult<BrepHiddenLineProjectionResponse> {
        crate::freecad::extract_brep_hidden_line_projections(
            self.app,
            crate::services::render::configured_freecad_cmd(self.state).as_deref(),
            request,
        )
    }
}

pub async fn submit_sketch_preview(
    request: SketchPreviewSubmissionRequest,
    state: &AppState,
    app: &dyn PathResolver,
) -> AppResult<SketchPreviewSubmissionPacket> {
    let preview_id = state
        .sketch_preview_run_registry
        .begin(&request.target.target_id);
    if request.mode == SketchPreviewMode::Auto {
        tokio::time::sleep(Duration::from_millis(650)).await;
    }
    if !state
        .sketch_preview_run_registry
        .is_current(&request.target.target_id, &preview_id)
    {
        return Ok(superseded_packet(request, preview_id));
    }

    let _render_guard = state.acquire_geometry_render().await;
    if !state
        .sketch_preview_run_registry
        .is_current(&request.target.target_id, &preview_id)
    {
        return Ok(superseded_packet(request, preview_id));
    }

    let pipeline = RuntimeSketchPreviewPipeline { state, app };
    Ok(run_preview_pipeline_with_current(
        request,
        preview_id,
        &pipeline,
        |target_id, preview_id| {
            state
                .sketch_preview_run_registry
                .is_current(target_id, preview_id)
        },
    ))
}

#[cfg(test)]
fn run_preview_pipeline(
    request: SketchPreviewSubmissionRequest,
    preview_id: String,
    pipeline: &dyn SketchPreviewPipeline,
) -> SketchPreviewSubmissionPacket {
    run_preview_pipeline_with_current(request, preview_id, pipeline, |_, _| true)
}

fn run_preview_pipeline_with_current(
    request: SketchPreviewSubmissionRequest,
    preview_id: String,
    pipeline: &dyn SketchPreviewPipeline,
    is_current: impl Fn(&str, &str) -> bool,
) -> SketchPreviewSubmissionPacket {
    let mut packet = base_packet(request, preview_id);
    let orthographic_repair = auto_repair_orthographic_document(&packet.document);
    packet.document = orthographic_repair.document;
    packet.repair_evidence = orthographic_repair.evidence;
    if let Err(error) = validate_submission(&packet.target, &packet.document) {
        return fail_packet(packet, SketchPreviewFailureStage::Validation, error);
    }

    let renderer = select_renderer(&packet.document);
    packet.renderer = Some(renderer);
    for attempt in 0..=1 {
        if !is_current(&packet.target.target_id, &packet.preview_id) {
            packet.status = SketchPreviewSubmissionStatus::Superseded;
            return packet;
        }

        let (draft_source, artifact_bundle) =
            match pipeline.render(renderer, &packet.target, &packet.document) {
                Ok(result) => result,
                Err(error) => return fail_packet(packet, SketchPreviewFailureStage::Render, error),
            };
        packet.draft_source = Some(draft_source);
        packet.artifact_bundle = Some(artifact_bundle.clone());
        if !is_current(&packet.target.target_id, &packet.preview_id) {
            packet.status = SketchPreviewSubmissionStatus::Superseded;
            return packet;
        }

        if renderer == SketchPreviewRenderer::Draft {
            return packet;
        }

        let candidate_response = match pipeline.analyze_candidates(SketchBrepCandidateRequest {
            document: packet.document.clone(),
        }) {
            Ok(response) => response,
            Err(error) => {
                return fail_packet(packet, SketchPreviewFailureStage::CandidateAnalysis, error)
            }
        };
        packet.candidate_response = Some(candidate_response);
        if !is_current(&packet.target.target_id, &packet.preview_id) {
            packet.status = SketchPreviewSubmissionStatus::Superseded;
            return packet;
        }

        if !has_projection_artifact(&artifact_bundle) {
            return packet;
        }

        let hidden_line_response = match pipeline.hidden_lines(BrepHiddenLineProjectionRequest {
            artifact_bundle,
            views: vec![SketchView::Front, SketchView::Top, SketchView::Side],
            tolerance: Some(0.1),
            sketch_document: Some(packet.document.clone()),
        }) {
            Ok(response) => response,
            Err(error) => {
                return fail_packet(
                    packet,
                    SketchPreviewFailureStage::HiddenLineProjection,
                    error,
                )
            }
        };
        if !is_current(&packet.target.target_id, &packet.preview_id) {
            packet.status = SketchPreviewSubmissionStatus::Superseded;
            return packet;
        }

        if attempt == 0 {
            let repair = auto_repair_projection(&packet.document, &hidden_line_response);
            if !repair.evidence.is_empty() {
                packet.document = repair.document;
                packet.repair_attempts = 1;
                packet.repair_evidence.extend(repair.evidence);
                packet.draft_source = None;
                packet.artifact_bundle = None;
                packet.candidate_response = None;
                packet.hidden_line_response = None;
                continue;
            }
        }

        packet.hidden_line_response = Some(hidden_line_response);
        return packet;
    }

    packet
}

fn base_packet(
    request: SketchPreviewSubmissionRequest,
    preview_id: String,
) -> SketchPreviewSubmissionPacket {
    SketchPreviewSubmissionPacket {
        preview_id,
        target: request.target,
        mode: request.mode,
        status: SketchPreviewSubmissionStatus::Completed,
        renderer: None,
        document: request.document,
        repair_attempts: 0,
        repair_evidence: Vec::new(),
        draft_source: None,
        artifact_bundle: None,
        candidate_response: None,
        hidden_line_response: None,
        failure_stage: None,
        error: None,
    }
}

fn superseded_packet(
    request: SketchPreviewSubmissionRequest,
    preview_id: String,
) -> SketchPreviewSubmissionPacket {
    let mut packet = base_packet(request, preview_id);
    packet.status = SketchPreviewSubmissionStatus::Superseded;
    packet
}

fn fail_packet(
    mut packet: SketchPreviewSubmissionPacket,
    stage: SketchPreviewFailureStage,
    error: AppError,
) -> SketchPreviewSubmissionPacket {
    packet.status = SketchPreviewSubmissionStatus::Failed;
    packet.failure_stage = Some(stage);
    packet.error = Some(error);
    packet
}

fn validate_submission(target: &SketchPreviewTarget, document: &SketchDocument) -> AppResult<()> {
    if target.target_id.trim().is_empty() {
        return Err(AppError::validation(
            "sketch preview targetId must be non-empty.",
        ));
    }
    if target.part_id.trim().is_empty() {
        return Err(AppError::validation(
            "sketch preview partId must be non-empty.",
        ));
    }
    if document.document_id.trim().is_empty() {
        return Err(AppError::validation(
            "sketch preview documentId must be non-empty.",
        ));
    }
    if document.sketches.is_empty() {
        return Err(AppError::validation(
            "sketch preview document must include at least one sketch.",
        ));
    }
    for sketch in &document.sketches {
        validate_sketch_definition("sketch preview", sketch)?;
    }
    let constraints = crate::services::sketch_constraint_validation::validate_constraints(document);
    if !constraints.passed {
        return Err(AppError::validation(format!(
            "Sketch preview constraint validation failed: {}",
            constraints.issues.join(" ")
        )));
    }
    Ok(())
}

fn select_renderer(document: &SketchDocument) -> SketchPreviewRenderer {
    let front = has_closed_profile(document, SketchView::Front);
    let secondary = has_closed_profile(document, SketchView::Top)
        || has_closed_profile(document, SketchView::Side);
    if front && secondary {
        SketchPreviewRenderer::PreviewHull
    } else {
        SketchPreviewRenderer::Draft
    }
}

fn has_closed_profile(document: &SketchDocument, view: SketchView) -> bool {
    document.sketches.iter().any(|sketch| {
        sketch.view == view && sketch.primitives.iter().any(|primitive| primitive.closed)
    })
}

fn draft_sketch(document: &SketchDocument) -> AppResult<SketchDefinition> {
    document
        .sketches
        .iter()
        .find(|sketch| {
            sketch.view == SketchView::Front
                && sketch.primitives.iter().any(|primitive| primitive.closed)
        })
        .or_else(|| {
            document
                .sketches
                .iter()
                .find(|sketch| sketch.primitives.iter().any(|primitive| primitive.closed))
        })
        .cloned()
        .ok_or_else(|| AppError::validation("Sketch preview requires a closed profile."))
}

fn has_projection_artifact(bundle: &ArtifactBundle) -> bool {
    !bundle.fcstd_path.trim().is_empty()
        || bundle.export_artifacts.iter().any(|artifact| {
            artifact.format.eq_ignore_ascii_case("step") && !artifact.path.trim().is_empty()
        })
}

#[derive(Clone, Copy)]
struct Bounds2d {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl Bounds2d {
    fn width(self) -> f64 {
        self.max_x - self.min_x
    }

    fn height(self) -> f64 {
        self.max_y - self.min_y
    }
}

struct AutoRepairResult {
    document: SketchDocument,
    evidence: Vec<SketchBrepAutoRepairEvidence>,
}

const MAX_ORTHOGRAPHIC_REPAIR_PASSES: usize = 8;
const ORTHOGRAPHIC_TOLERANCE: f64 = 1e-6;

#[derive(Clone, Copy)]
enum OrthographicAxis {
    X,
    Y,
}

impl OrthographicAxis {
    fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Y => "Y",
        }
    }

    fn min(self, bounds: Bounds2d) -> f64 {
        match self {
            Self::X => bounds.min_x,
            Self::Y => bounds.min_y,
        }
    }

    fn max(self, bounds: Bounds2d) -> f64 {
        match self {
            Self::X => bounds.max_x,
            Self::Y => bounds.max_y,
        }
    }

    fn dimension(self, bounds: Bounds2d) -> f64 {
        self.max(bounds) - self.min(bounds)
    }
}

#[derive(Clone, Copy)]
struct OrthographicProfile {
    sketch_index: usize,
    primitive_index: usize,
    bounds: Bounds2d,
}

#[derive(Clone, Copy)]
enum OrthographicEdit {
    Scale {
        current: f64,
        target: f64,
    },
    Translate {
        current_min: f64,
        current_max: f64,
        target_min: f64,
        target_max: f64,
    },
}

#[derive(Clone)]
struct OrthographicRepairAction {
    profile: OrthographicProfile,
    view: SketchView,
    axis: OrthographicAxis,
    edit: OrthographicEdit,
}

fn auto_repair_orthographic_document(document: &SketchDocument) -> AutoRepairResult {
    let mut document = document.clone();
    let mut evidence = Vec::new();

    for _ in 0..MAX_ORTHOGRAPHIC_REPAIR_PASSES {
        let Some(action) = next_orthographic_repair(&document) else {
            break;
        };
        let primitive_id = document.sketches[action.profile.sketch_index].primitives
            [action.profile.primitive_index]
            .primitive_id
            .clone();
        let Some(detail) = apply_orthographic_repair(&mut document, &action) else {
            break;
        };
        evidence.push(SketchBrepAutoRepairEvidence {
            primitive_id,
            view: action.view.clone(),
            detail,
        });
    }

    if !evidence.is_empty() {
        if let Ok(repaired) =
            crate::services::sketch_constraint_validation::repair_dimension_constraint_values(
                &document,
            )
        {
            document = repaired.document;
        }
    }

    AutoRepairResult { document, evidence }
}

fn next_orthographic_repair(document: &SketchDocument) -> Option<OrthographicRepairAction> {
    let front = orthographic_profile(document, SketchView::Front)?;
    let top = orthographic_profile(document, SketchView::Top);
    let side = orthographic_profile(document, SketchView::Side);

    if let Some(top) = top {
        if let Some(action) = scale_repair(
            top,
            SketchView::Top,
            OrthographicAxis::X,
            front.bounds.width(),
        ) {
            return Some(action);
        }
        if let Some(action) = translate_repair(
            top,
            SketchView::Top,
            OrthographicAxis::X,
            front.bounds.min_x,
            front.bounds.max_x,
        ) {
            return Some(action);
        }
    }

    if let Some(side) = side {
        if let Some(action) = scale_repair(
            side,
            SketchView::Side,
            OrthographicAxis::Y,
            front.bounds.height(),
        ) {
            return Some(action);
        }
        if let Some(action) = translate_repair(
            side,
            SketchView::Side,
            OrthographicAxis::Y,
            front.bounds.min_y,
            front.bounds.max_y,
        ) {
            return Some(action);
        }
        if let Some(top) = top {
            if let Some(action) = scale_repair(
                side,
                SketchView::Side,
                OrthographicAxis::X,
                top.bounds.height(),
            ) {
                return Some(action);
            }
            if let Some(action) = translate_repair(
                side,
                SketchView::Side,
                OrthographicAxis::X,
                top.bounds.min_y,
                top.bounds.max_y,
            ) {
                return Some(action);
            }
        }
    }
    None
}

fn orthographic_profile(
    document: &SketchDocument,
    view: SketchView,
) -> Option<OrthographicProfile> {
    document
        .sketches
        .iter()
        .enumerate()
        .find_map(|(sketch_index, sketch)| {
            if sketch.view != view {
                return None;
            }
            sketch
                .primitives
                .iter()
                .enumerate()
                .find_map(|(primitive_index, primitive)| {
                    if !primitive.closed {
                        return None;
                    }
                    primitive_bounds(primitive).map(|bounds| OrthographicProfile {
                        sketch_index,
                        primitive_index,
                        bounds,
                    })
                })
        })
}

fn primitive_bounds(primitive: &SketchPrimitive) -> Option<Bounds2d> {
    match primitive.kind {
        SketchPrimitiveKind::Circle => {
            let center = *primitive.points.first()?;
            let radius = primitive
                .radius
                .filter(|radius| radius.is_finite() && *radius > 0.0)?;
            Some(Bounds2d {
                min_x: center[0] - radius,
                min_y: center[1] - radius,
                max_x: center[0] + radius,
                max_y: center[1] + radius,
            })
        }
        SketchPrimitiveKind::Polyline => bounds_from_points(&logical_points(&primitive.points)),
        _ => None,
    }
}

fn scale_repair(
    profile: OrthographicProfile,
    view: SketchView,
    axis: OrthographicAxis,
    target: f64,
) -> Option<OrthographicRepairAction> {
    let current = axis.dimension(profile.bounds);
    (current > 0.0 && target > 0.0 && (current - target).abs() > ORTHOGRAPHIC_TOLERANCE).then_some(
        OrthographicRepairAction {
            profile,
            view,
            axis,
            edit: OrthographicEdit::Scale { current, target },
        },
    )
}

fn translate_repair(
    profile: OrthographicProfile,
    view: SketchView,
    axis: OrthographicAxis,
    target_min: f64,
    target_max: f64,
) -> Option<OrthographicRepairAction> {
    let current_min = axis.min(profile.bounds);
    let current_max = axis.max(profile.bounds);
    let current_size = current_max - current_min;
    let target_size = target_max - target_min;
    (current_size.is_finite()
        && target_size.is_finite()
        && (current_size - target_size).abs() <= ORTHOGRAPHIC_TOLERANCE
        && (current_min - target_min).abs() > ORTHOGRAPHIC_TOLERANCE)
        .then_some(OrthographicRepairAction {
            profile,
            view,
            axis,
            edit: OrthographicEdit::Translate {
                current_min,
                current_max,
                target_min,
                target_max,
            },
        })
}

fn apply_orthographic_repair(
    document: &mut SketchDocument,
    action: &OrthographicRepairAction,
) -> Option<String> {
    let primitive = document
        .sketches
        .get_mut(action.profile.sketch_index)?
        .primitives
        .get_mut(action.profile.primitive_index)?;
    if primitive.kind != SketchPrimitiveKind::Polyline || !primitive.closed {
        return None;
    }
    let mut points = logical_points(&primitive.points);
    let bounds = bounds_from_points(&points)?;
    let axis_index = action.axis.index();
    let detail = match action.edit {
        OrthographicEdit::Scale { current, target } => {
            let current_dimension = action.axis.dimension(bounds);
            if current_dimension <= 0.0 {
                return None;
            }
            let center = (action.axis.min(bounds) + action.axis.max(bounds)) / 2.0;
            let scale = target / current_dimension;
            for point in &mut points {
                point[axis_index] = round_coordinate(center + (point[axis_index] - center) * scale);
            }
            format!(
                "{} {} {}MM -> {}MM",
                sketch_view_label(&action.view),
                action.axis.label(),
                format_orthographic_number(current),
                format_orthographic_number(target)
            )
        }
        OrthographicEdit::Translate {
            current_min,
            current_max,
            target_min,
            target_max,
        } => {
            let delta = target_min - action.axis.min(bounds);
            for point in &mut points {
                point[axis_index] = round_coordinate(point[axis_index] + delta);
            }
            format!(
                "{} {} RANGE {}..{}MM -> {}..{}MM",
                sketch_view_label(&action.view),
                action.axis.label(),
                format_orthographic_number(current_min),
                format_orthographic_number(current_max),
                format_orthographic_number(target_min),
                format_orthographic_number(target_max)
            )
        }
    };
    points.push(*points.first()?);
    primitive.points = points;
    Some(detail)
}

fn sketch_view_label(view: &SketchView) -> &'static str {
    match view {
        SketchView::Front => "FRONT",
        SketchView::Top => "TOP",
        SketchView::Side => "SIDE",
        _ => "SKETCH",
    }
}

fn format_orthographic_number(value: f64) -> String {
    let value = round_coordinate(value);
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn auto_repair_projection(
    document: &SketchDocument,
    projection: &BrepHiddenLineProjectionResponse,
) -> AutoRepairResult {
    let mut document = document.clone();
    let mut evidence = Vec::new();
    let mut repaired = HashSet::new();
    let Some(validation) = &projection.validation else {
        return AutoRepairResult { document, evidence };
    };
    if validation.passed {
        return AutoRepairResult { document, evidence };
    }

    for issue in &validation.issues {
        let repair_kind = match issue.kind {
            SketchValidationIssueKind::BoundsMismatch => "bounds",
            SketchValidationIssueKind::ContainmentMismatch => "containment",
            _ => continue,
        };
        let Some((sketch_index, primitive_index)) = find_issue_primitive(&document, issue) else {
            continue;
        };
        let primitive_id = document.sketches[sketch_index].primitives[primitive_index]
            .primitive_id
            .clone();
        if repaired.contains(&primitive_id) {
            continue;
        }
        let view = document.sketches[sketch_index].view.clone();
        let primitive = &document.sketches[sketch_index].primitives[primitive_index];
        if primitive.kind != SketchPrimitiveKind::Polyline || !primitive.closed {
            continue;
        }
        let Some(projection_view) = projection.views.iter().find(|entry| entry.view == view) else {
            continue;
        };
        let source_points = logical_points(&primitive.points);
        let Some(source_bounds) = bounds_from_points(&source_points) else {
            continue;
        };
        let localized = targeted_projection_bounds(projection_view, issue, primitive);
        let projection_bounds = if repair_kind == "containment" {
            localized.or_else(|| bounds_from_projection_view(projection_view))
        } else {
            localized
                .filter(|bounds| bounds_are_scalable(*bounds))
                .or_else(|| bounds_from_projection_view(projection_view))
        };
        let Some(projection_bounds) = projection_bounds else {
            continue;
        };
        let target_bounds = if repair_kind == "bounds" {
            projection_bounds
        } else {
            union_bounds(source_bounds, projection_bounds)
        };
        if !bounds_are_scalable(source_bounds)
            || !bounds_are_scalable(target_bounds)
            || bounds_equal(source_bounds, target_bounds)
            || (repair_kind == "containment"
                && !containment_expansion_allowed(source_bounds, target_bounds))
        {
            continue;
        }

        let mut points = source_points
            .iter()
            .map(|point| remap_point(*point, source_bounds, target_bounds))
            .collect::<Vec<_>>();
        if let Some(first) = points.first().copied() {
            points.push(first);
        }
        document.sketches[sketch_index].primitives[primitive_index].points = points;
        evidence.push(SketchBrepAutoRepairEvidence {
            primitive_id: primitive_id.clone(),
            view: view.clone(),
            detail: format!(
                "{} {} {} bounds {}x{} -> {}x{}",
                if repair_kind == "bounds" {
                    "BREP AUTO SNAP"
                } else {
                    "BREP AUTO CONTAIN"
                },
                format!("{:?}", view).to_uppercase(),
                primitive_id,
                format_number(source_bounds.width()),
                format_number(source_bounds.height()),
                format_number(target_bounds.width()),
                format_number(target_bounds.height())
            ),
        });
        repaired.insert(primitive_id);
    }

    AutoRepairResult { document, evidence }
}

fn find_issue_primitive(
    document: &SketchDocument,
    issue: &SketchValidationIssue,
) -> Option<(usize, usize)> {
    if !issue.sketch_id.is_empty() {
        if let Some((sketch_index, sketch)) = document
            .sketches
            .iter()
            .enumerate()
            .find(|(_, sketch)| sketch.sketch_id == issue.sketch_id)
        {
            if let Some(primitive_index) = direct_issue_primitive(sketch, issue) {
                return Some((sketch_index, primitive_index));
            }
        }
    }

    for (sketch_index, sketch) in document.sketches.iter().enumerate() {
        if let Some(index) = sketch
            .primitives
            .iter()
            .position(|primitive| topology_matches(primitive, issue))
        {
            return Some((sketch_index, index));
        }
    }
    for (sketch_index, sketch) in document.sketches.iter().enumerate() {
        let matches = sketch
            .primitives
            .iter()
            .enumerate()
            .filter(|(_, primitive)| edge_matches(primitive, issue.edge_id.as_deref()))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            return Some((sketch_index, matches[0]));
        }
    }
    None
}

fn direct_issue_primitive(
    sketch: &SketchDefinition,
    issue: &SketchValidationIssue,
) -> Option<usize> {
    let primitive_id = issue.primitive_id.as_deref()?;
    let index = sketch
        .primitives
        .iter()
        .position(|primitive| primitive.primitive_id == primitive_id)?;
    let primitive = &sketch.primitives[index];
    if issue.topology.is_some()
        && primitive.topology.is_some()
        && !topology_matches(primitive, issue)
    {
        return None;
    }
    if issue.edge_id.is_some()
        && primitive
            .topology
            .as_ref()
            .is_some_and(|topology| !topology.edge_ids.is_empty())
        && !edge_matches(primitive, issue.edge_id.as_deref())
    {
        return None;
    }
    Some(index)
}

fn topology_matches(primitive: &SketchPrimitive, issue: &SketchValidationIssue) -> bool {
    let (Some(expected), Some(actual)) = (&issue.topology, &primitive.topology) else {
        return false;
    };
    if expected
        .loop_id
        .as_ref()
        .is_some_and(|loop_id| actual.loop_id.as_ref() == Some(loop_id))
    {
        return true;
    }
    same_edge_ids(&expected.edge_ids, &actual.edge_ids)
}

fn edge_matches(primitive: &SketchPrimitive, edge_id: Option<&str>) -> bool {
    let Some(edge_id) = edge_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    primitive.topology.as_ref().is_some_and(|topology| {
        topology
            .edge_ids
            .iter()
            .any(|candidate| candidate.trim() == edge_id)
    })
}

fn targeted_projection_bounds(
    view: &BrepHiddenLineProjectionView,
    issue: &SketchValidationIssue,
    primitive: &SketchPrimitive,
) -> Option<Bounds2d> {
    let topology = issue.topology.as_ref().or(primitive.topology.as_ref());
    if let Some(topology) = topology {
        let matched_loop = topology
            .loop_id
            .as_ref()
            .and_then(|loop_id| view.loops.iter().find(|entry| &entry.loop_id == loop_id))
            .or_else(|| {
                if topology.edge_ids.is_empty() {
                    None
                } else {
                    view.loops
                        .iter()
                        .find(|entry| same_edge_ids(&entry.edge_ids, &topology.edge_ids))
                }
            });
        if let Some(bounds) = matched_loop.and_then(|entry| bounds_from_points(&entry.points)) {
            return Some(bounds);
        }
        if !topology.edge_ids.is_empty() {
            let target_ids = normalized_edge_ids(&topology.edge_ids);
            let points = view
                .visible_edges
                .iter()
                .chain(&view.hidden_edges)
                .filter(|edge| target_ids.binary_search(&edge.edge_id.trim()).is_ok())
                .flat_map(|edge| edge.points.iter().copied())
                .collect::<Vec<_>>();
            if let Some(bounds) = bounds_from_points(&points) {
                return Some(bounds);
            }
        }
    }
    if let Some(edge_id) = issue.edge_id.as_deref().map(str::trim) {
        let points = view
            .visible_edges
            .iter()
            .chain(&view.hidden_edges)
            .filter(|edge| edge.edge_id.trim() == edge_id)
            .flat_map(|edge| edge.points.iter().copied())
            .collect::<Vec<_>>();
        return bounds_from_points(&points);
    }
    None
}

fn bounds_from_projection_view(view: &BrepHiddenLineProjectionView) -> Option<Bounds2d> {
    let points = view
        .visible_edges
        .iter()
        .chain(&view.hidden_edges)
        .flat_map(|edge| edge.points.iter().copied())
        .collect::<Vec<_>>();
    bounds_from_points(&points)
}

fn bounds_from_points(points: &[[f64; 2]]) -> Option<Bounds2d> {
    let first = *points.first()?;
    if !first[0].is_finite() || !first[1].is_finite() {
        return None;
    }
    let mut bounds = Bounds2d {
        min_x: first[0],
        min_y: first[1],
        max_x: first[0],
        max_y: first[1],
    };
    for point in points.iter().skip(1) {
        if !point[0].is_finite() || !point[1].is_finite() {
            return None;
        }
        bounds.min_x = bounds.min_x.min(point[0]);
        bounds.min_y = bounds.min_y.min(point[1]);
        bounds.max_x = bounds.max_x.max(point[0]);
        bounds.max_y = bounds.max_y.max(point[1]);
    }
    Some(bounds)
}

fn logical_points(points: &[[f64; 2]]) -> Vec<[f64; 2]> {
    if points.len() >= 2 && points.first() == points.last() {
        points[..points.len() - 1].to_vec()
    } else {
        points.to_vec()
    }
}

fn union_bounds(left: Bounds2d, right: Bounds2d) -> Bounds2d {
    Bounds2d {
        min_x: left.min_x.min(right.min_x),
        min_y: left.min_y.min(right.min_y),
        max_x: left.max_x.max(right.max_x),
        max_y: left.max_y.max(right.max_y),
    }
}

fn bounds_are_scalable(bounds: Bounds2d) -> bool {
    bounds.width() > 1e-6 && bounds.height() > 1e-6
}

fn bounds_equal(left: Bounds2d, right: Bounds2d) -> bool {
    (left.min_x - right.min_x).abs() <= 1e-6
        && (left.min_y - right.min_y).abs() <= 1e-6
        && (left.max_x - right.max_x).abs() <= 1e-6
        && (left.max_y - right.max_y).abs() <= 1e-6
}

fn containment_expansion_allowed(source: Bounds2d, target: Bounds2d) -> bool {
    target.width() / source.width() <= 2.0 && target.height() / source.height() <= 2.0
}

fn remap_point(point: [f64; 2], source: Bounds2d, target: Bounds2d) -> [f64; 2] {
    let x_ratio = (point[0] - source.min_x) / source.width();
    let y_ratio = (point[1] - source.min_y) / source.height();
    [
        round_coordinate(target.min_x + x_ratio * target.width()),
        round_coordinate(target.min_y + y_ratio * target.height()),
    ]
}

fn normalized_edge_ids(edge_ids: &[String]) -> Vec<&str> {
    let mut normalized = edge_ids
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    normalized.sort_unstable();
    normalized
}

fn same_edge_ids(left: &[String], right: &[String]) -> bool {
    let left = normalized_edge_ids(left);
    let right = normalized_edge_ids(right);
    !left.is_empty() && left == right
}

fn round_coordinate(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn format_number(value: f64) -> String {
    let value = round_coordinate(value);
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.4}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{
        AppErrorCode, BrepHiddenLineProjectionRequest, SketchBrepCandidateRequest,
        SketchDefinition, SketchPrimitive, SketchPrimitiveKind, SketchView,
    };
    use std::cell::Cell;

    struct FakePipeline {
        render_count: Cell<u8>,
        hidden_line_count: Cell<u8>,
        repair_first_projection: bool,
        render_error: Option<AppError>,
    }

    impl FakePipeline {
        fn happy() -> Self {
            Self {
                render_count: Cell::new(0),
                hidden_line_count: Cell::new(0),
                repair_first_projection: false,
                render_error: None,
            }
        }

        fn repair() -> Self {
            Self {
                repair_first_projection: true,
                ..Self::happy()
            }
        }
    }

    impl SketchPreviewPipeline for FakePipeline {
        fn render(
            &self,
            _renderer: SketchPreviewRenderer,
            _target: &SketchPreviewTarget,
            _document: &SketchDocument,
        ) -> crate::contracts::AppResult<(SketchDraftSource, ArtifactBundle)> {
            self.render_count.set(self.render_count.get() + 1);
            if let Some(error) = self.render_error.clone() {
                return Err(error);
            }
            Ok((draft_source(), artifact_bundle()))
        }

        fn analyze_candidates(
            &self,
            _request: SketchBrepCandidateRequest,
        ) -> crate::contracts::AppResult<SketchBrepCandidateResponse> {
            Ok(candidate_response())
        }

        fn hidden_lines(
            &self,
            _request: BrepHiddenLineProjectionRequest,
        ) -> crate::contracts::AppResult<BrepHiddenLineProjectionResponse> {
            let call = self.hidden_line_count.get() + 1;
            self.hidden_line_count.set(call);
            if self.repair_first_projection && call == 1 {
                Ok(mismatched_projection())
            } else {
                Ok(passing_projection())
            }
        }
    }

    fn rectangle(sketch_id: &str, view: SketchView, primitive_id: &str) -> SketchDefinition {
        SketchDefinition {
            sketch_id: sketch_id.to_string(),
            view,
            plane: None,
            primitives: vec![SketchPrimitive {
                primitive_id: primitive_id.to_string(),
                kind: SketchPrimitiveKind::Polyline,
                points: vec![
                    [10.0, 20.0],
                    [60.0, 20.0],
                    [60.0, 50.0],
                    [10.0, 50.0],
                    [10.0, 20.0],
                ],
                closed: true,
                radius: None,
                topology: None,
                provenance: None,
            }],
            constraints: Vec::new(),
        }
    }

    fn document() -> SketchDocument {
        SketchDocument {
            document_id: "document-preview".to_string(),
            active_sketch_id: Some("front".to_string()),
            units: Some("mm".to_string()),
            metadata: None,
            sketches: vec![
                rectangle("front", SketchView::Front, "front-profile"),
                rectangle("top", SketchView::Top, "top-profile"),
            ],
        }
    }

    fn request() -> SketchPreviewSubmissionRequest {
        SketchPreviewSubmissionRequest {
            target: SketchPreviewTarget {
                target_id: "workspace-sketch".to_string(),
                part_id: "sketch-preview".to_string(),
            },
            document: document(),
            mode: SketchPreviewMode::Manual,
        }
    }

    #[test]
    fn orthographic_repair_matches_front_width_and_range_before_preview() {
        let mut document = document();
        document.sketches[1].primitives[0].points = vec![
            [10.0, 10.0],
            [50.0, 10.0],
            [50.0, 32.0],
            [10.0, 32.0],
            [10.0, 10.0],
        ];

        let repaired = auto_repair_orthographic_document(&document);

        assert_eq!(
            repaired
                .evidence
                .iter()
                .map(|entry| entry.detail.as_str())
                .collect::<Vec<_>>(),
            vec!["TOP X 40MM -> 50MM", "TOP X RANGE 5..55MM -> 10..60MM"]
        );
        assert_eq!(
            repaired.document.sketches[1].primitives[0].points,
            vec![
                [10.0, 10.0],
                [60.0, 10.0],
                [60.0, 32.0],
                [10.0, 32.0],
                [10.0, 10.0]
            ]
        );
        assert_eq!(document.sketches[1].primitives[0].points[1], [50.0, 10.0]);
    }

    #[test]
    fn orthographic_repair_chains_side_height_and_depth_alignment() {
        let mut document = document();
        document.sketches[1].primitives[0].points = vec![
            [10.0, 10.0],
            [60.0, 10.0],
            [60.0, 32.0],
            [10.0, 32.0],
            [10.0, 10.0],
        ];
        document.sketches.push(SketchDefinition {
            sketch_id: "side".to_string(),
            view: SketchView::Side,
            plane: None,
            primitives: vec![SketchPrimitive {
                primitive_id: "side-profile".to_string(),
                kind: SketchPrimitiveKind::Polyline,
                points: vec![
                    [10.0, 10.0],
                    [40.0, 10.0],
                    [40.0, 35.0],
                    [10.0, 35.0],
                    [10.0, 10.0],
                ],
                closed: true,
                radius: None,
                topology: None,
                provenance: None,
            }],
            constraints: Vec::new(),
        });

        let repaired = auto_repair_orthographic_document(&document);

        assert_eq!(
            repaired
                .evidence
                .iter()
                .map(|entry| entry.detail.as_str())
                .collect::<Vec<_>>(),
            vec![
                "SIDE Y 25MM -> 30MM",
                "SIDE Y RANGE 7.5..37.5MM -> 20..50MM",
                "SIDE X 30MM -> 22MM",
                "SIDE X RANGE 14..36MM -> 10..32MM",
            ]
        );
        assert_eq!(
            repaired.document.sketches[2].primitives[0].points,
            vec![
                [10.0, 20.0],
                [32.0, 20.0],
                [32.0, 50.0],
                [10.0, 50.0],
                [10.0, 20.0]
            ]
        );
    }

    #[test]
    fn raw_orthographic_document_is_repaired_inside_preview_pipeline() {
        let pipeline = FakePipeline::happy();
        let mut request = request();
        request.document.sketches[1].primitives[0].points = vec![
            [10.0, 10.0],
            [50.0, 10.0],
            [50.0, 32.0],
            [10.0, 32.0],
            [10.0, 10.0],
        ];

        let packet = run_preview_pipeline(request, "preview-orthographic".to_string(), &pipeline);

        assert_eq!(packet.status, SketchPreviewSubmissionStatus::Completed);
        assert_eq!(packet.repair_attempts, 0);
        assert_eq!(packet.repair_evidence.len(), 2);
        assert_eq!(packet.repair_evidence[0].detail, "TOP X 40MM -> 50MM");
        assert_eq!(
            packet.document.sketches[1].primitives[0].points,
            vec![
                [10.0, 10.0],
                [60.0, 10.0],
                [60.0, 32.0],
                [10.0, 32.0],
                [10.0, 10.0]
            ]
        );
        assert_eq!(pipeline.render_count.get(), 1);
        assert_eq!(pipeline.hidden_line_count.get(), 1);
    }

    fn draft_source() -> SketchDraftSource {
        serde_json::from_value(serde_json::json!({
            "sourceLanguage": "ecky",
            "geometryBackend": "eckyRust",
            "macroDialect": "ecky",
            "source": "(model (part sketch-preview (box 1 1 1)))",
            "warnings": []
        }))
        .expect("draft source")
    }

    fn artifact_bundle() -> ArtifactBundle {
        serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "modelId": "preview-model",
            "sourceKind": "generated",
            "engineKind": "eckyIrV0",
            "sourceLanguage": "ecky",
            "geometryBackend": "eckyRust",
            "contentHash": "preview-hash",
            "artifactVersion": 1,
            "fcstdPath": "",
            "manifestPath": "/tmp/manifest.json",
            "modelStlPath": "/tmp/model.stl",
            "viewerAssets": [],
            "edgeTargets": [],
            "faceTargets": [],
            "calloutAnchors": [],
            "measurementGuides": [],
            "exportArtifacts": [{"label": "STEP", "format": "step", "path": "/tmp/model.step", "role": "primary"}]
        }))
        .expect("artifact bundle")
    }

    fn candidate_response() -> SketchBrepCandidateResponse {
        serde_json::from_value(serde_json::json!({
            "graph": {"vertices": [], "edges": []},
            "search": {"cells": [], "rejectedCellCount": 0, "solutions": [], "evidence": []},
            "validation": {"passed": true, "issues": [], "evidence": ["candidate pass"]}
        }))
        .expect("candidate response")
    }

    fn passing_projection() -> BrepHiddenLineProjectionResponse {
        serde_json::from_value(serde_json::json!({
            "modelId": "preview-model",
            "sourceArtifactPath": "/tmp/model.step",
            "views": [],
            "warningEntries": [],
            "validation": {"passed": true, "issues": [], "evidence": ["projection pass"]}
        }))
        .expect("passing projection")
    }

    fn mismatched_projection() -> BrepHiddenLineProjectionResponse {
        serde_json::from_value(serde_json::json!({
            "modelId": "preview-model",
            "sourceArtifactPath": "/tmp/model.step",
            "views": [{
                "view": "front",
                "direction": [0, -1, 0],
                "visibleEdges": [
                    {"edgeId": "bottom", "points": [[0, 0], [80, 0]], "sourceClass": "visible"},
                    {"edgeId": "right", "points": [[80, 0], [80, 40]], "sourceClass": "visible"}
                ],
                "hiddenEdges": [
                    {"edgeId": "top", "points": [[80, 40], [0, 40]], "sourceClass": "hidden"}
                ],
                "loops": []
            }],
            "warningEntries": [],
            "validation": {
                "passed": false,
                "issues": [{
                    "sketchId": "front",
                    "kind": "boundsMismatch",
                    "view": "front",
                    "primitiveId": "front-profile",
                    "severity": "error",
                    "message": "Front bounds mismatch."
                }],
                "evidence": []
            }
        }))
        .expect("mismatched projection")
    }

    #[test]
    fn happy_preview_returns_one_canonical_packet() {
        let pipeline = FakePipeline::happy();

        let packet = run_preview_pipeline(request(), "preview-1".to_string(), &pipeline);

        assert_eq!(packet.status, SketchPreviewSubmissionStatus::Completed);
        assert_eq!(packet.renderer, Some(SketchPreviewRenderer::PreviewHull));
        assert_eq!(packet.preview_id, "preview-1");
        assert!(packet.draft_source.is_some());
        assert!(packet.artifact_bundle.is_some());
        assert!(packet.candidate_response.is_some());
        assert!(packet.hidden_line_response.is_some());
        assert!(packet.error.is_none());
        assert_eq!(pipeline.render_count.get(), 1);
        assert_eq!(pipeline.hidden_line_count.get(), 1);
    }

    #[test]
    fn projection_mismatch_repairs_and_rebuilds_at_most_once() {
        let pipeline = FakePipeline::repair();

        let packet = run_preview_pipeline(request(), "preview-repair".to_string(), &pipeline);

        assert_eq!(packet.status, SketchPreviewSubmissionStatus::Completed);
        assert_eq!(packet.repair_attempts, 1);
        assert_eq!(packet.repair_evidence.len(), 1);
        assert_eq!(pipeline.render_count.get(), 2);
        assert_eq!(pipeline.hidden_line_count.get(), 2);
        assert_eq!(
            packet.document.sketches[0].primitives[0].points,
            vec![
                [0.0, 0.0],
                [80.0, 0.0],
                [80.0, 40.0],
                [0.0, 40.0],
                [0.0, 0.0]
            ]
        );
    }

    #[test]
    fn render_failure_returns_raw_error_packet() {
        let pipeline = FakePipeline {
            render_error: Some(AppError::with_details(
                AppErrorCode::Render,
                "Sketch preview render failed.",
                "raw OCCT failure: null shape",
            )),
            ..FakePipeline::happy()
        };

        let packet = run_preview_pipeline(request(), "preview-error".to_string(), &pipeline);

        assert_eq!(packet.status, SketchPreviewSubmissionStatus::Failed);
        assert_eq!(
            packet.failure_stage,
            Some(SketchPreviewFailureStage::Render)
        );
        let error = packet.error.expect("raw error");
        assert_eq!(error.code, AppErrorCode::Render);
        assert_eq!(
            error.details.as_deref(),
            Some("raw OCCT failure: null shape")
        );
        assert_eq!(pipeline.render_count.get(), 1);
    }

    #[test]
    fn run_registry_makes_newest_preview_authoritative_per_target() {
        let registry = SketchPreviewRunRegistry::default();
        let first = registry.begin("workspace-sketch");
        let other_target = registry.begin("other-sketch");
        let newest = registry.begin("workspace-sketch");

        assert!(!registry.is_current("workspace-sketch", &first));
        assert!(registry.is_current("workspace-sketch", &newest));
        assert!(registry.is_current("other-sketch", &other_target));
    }
}
