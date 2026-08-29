use serde::{Deserialize, Serialize};
use specta::Type;

use super::{ArtifactBundle, DesignOutput, ModelManifest};

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentOrigin {
    pub host_label: String,
    pub client_kind: String,
    pub agent_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_model_label: Option<String>,
    pub session_id: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    pub session_id: String,
    pub client_kind: String,
    pub host_label: String,
    pub agent_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_model_label: Option<String>,
    pub thread_id: Option<String>,
    pub message_id: Option<String>,
    pub model_id: Option<String>,
    pub phase: String,
    pub status_text: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AgentActivityActorKind {
    Agent,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentActivityActor {
    pub kind: AgentActivityActorKind,
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AgentActivityKind {
    Trace,
    Runtime,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AgentActivitySeverity {
    Info,
    Success,
    Warning,
    Error,
    Question,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AgentActivityState {
    Active,
    Resolved,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentActivityEventInput {
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    pub actor_kind: AgentActivityActorKind,
    pub actor_id: String,
    pub actor_label: String,
    pub kind: AgentActivityKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub severity: AgentActivitySeverity,
    pub state: AgentActivityState,
    pub requires_attention: bool,
    pub occurred_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentActivityEvent {
    pub event_id: String,
    pub cursor: u64,
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    pub actor: AgentActivityActor,
    pub kind: AgentActivityKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub severity: AgentActivitySeverity,
    pub state: AgentActivityState,
    pub requires_attention: bool,
    pub occurred_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentActivityCatchUp {
    pub events: Vec<AgentActivityEvent>,
    pub latest_cursor: u64,
    pub oldest_cursor: u64,
    pub has_more: bool,
    pub dropped_count: u64,
    pub retained_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentDraft {
    pub preview_id: String,
    pub session_id: String,
    pub thread_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_message_id: Option<String>,
    pub design_output: DesignOutput,
    pub artifact_bundle: ArtifactBundle,
    pub model_manifest: ModelManifest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_feedback: Option<AgentDraftFeedback>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentDraftProjection {
    pub preview_id: String,
    pub session_id: String,
    pub thread_id: String,
    pub base_message_id: Option<String>,
    pub design_output: DesignOutput,
    pub artifact_bundle: ArtifactBundle,
    pub model_manifest: ModelManifest,
    pub draft_feedback: Option<AgentDraftFeedback>,
    pub updated_at: u64,
    pub dense_topology_ref: Option<String>,
    pub edge_count: usize,
    pub face_count: usize,
    pub selection_target_count: usize,
    pub observed_bytes: usize,
    pub truncated_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AgentDraftFeedbackStatus {
    Checking,
    Passed,
    Failed,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AgentDraftFeedbackSource {
    StructuralVerification,
    RenderError,
    ToolError,
    VisualRepair,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentDraftFeedback {
    pub session_id: String,
    pub thread_id: String,
    pub preview_id: String,
    pub status: AgentDraftFeedbackStatus,
    pub summary: String,
    pub items: Vec<AgentDraftFeedbackItem>,
    #[serde(default)]
    pub authoring_lints: Vec<AgentDraftFeedbackAuthoringLint>,
    pub source: AgentDraftFeedbackSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentDraftFeedbackItem {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentDraftFeedbackAuthoringLint {
    pub kind: String,
    pub part_key: String,
    pub param_key: String,
    pub delta: f64,
    pub occurrence_count: usize,
    pub suggested_param_key: String,
    pub message: String,
    #[serde(default)]
    pub source_stable_node_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TargetLeaseInfo {
    pub session_id: String,
    pub thread_id: String,
    pub message_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    pub host_label: String,
    pub agent_label: String,
    pub acquired_at: u64,
    pub expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThreadAgentState {
    /// "none" | "sleeping" | "waking" | "waiting" | "active" | "disconnected" | "error"
    pub connection_state: String,
    pub agent_label: Option<String>,
    pub llm_model_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub phase: Option<String>,
    pub status_text: Option<String>,
    #[serde(default)]
    pub busy: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity_started_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attention_kind: Option<String>,
    #[serde(default)]
    pub waiting_on_prompt: bool,
    pub updated_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkingVersionEvent {
    pub session_id: String,
    pub thread_id: String,
    pub message_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentDraftPreviewUpdatedEvent {
    pub session_id: String,
    pub thread_id: String,
    pub preview_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_message_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    pub design: DesignOutput,
    pub artifact_bundle: ArtifactBundle,
    pub model_manifest: ModelManifest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback: Option<AgentDraftFeedback>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentDraftPreviewChangedEvent {
    pub session_id: String,
    pub thread_id: String,
    pub preview_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_message_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    pub revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback_status: Option<AgentDraftFeedbackStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback_summary: Option<String>,
}
