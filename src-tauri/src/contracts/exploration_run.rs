//! Boundary contracts for the Rust-owned generation/exploration runner.
//!
//! Callers submit intent and renderable context. They never submit lifecycle
//! facts such as "version appended" or "verification passed".

use serde::{Deserialize, Serialize};
use specta::Type;

use super::{
    ArtifactBundle, Attachment, DesignOutput, GenerateDesignOptions, Message, ModelManifest,
    StructuralVerificationResult, UsageSummary,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExplorationRunKind {
    Interactive,
    Controller,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExplorationRunPhase {
    Queued,
    Planning,
    Building,
    Verifying,
    Deciding,
    AwaitingInput,
    Completed,
    Stopped,
    Interrupted,
    Superseded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StartExplorationRunInput {
    pub request_id: String,
    pub thread_id: String,
    pub prompt: String,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub image_data: Option<String>,
    #[serde(default)]
    pub parent_macro_code: Option<String>,
    #[serde(default)]
    pub working_design: Option<DesignOutput>,
    #[serde(default)]
    pub base_version_id: Option<String>,
    pub kind: ExplorationRunKind,
    #[serde(default)]
    pub options: GenerateDesignOptions,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    #[serde(default)]
    pub hard_constraints: Vec<String>,
    #[serde(default)]
    pub soft_preferences: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExplorationRunProgress {
    pub request_id: String,
    pub thread_id: String,
    #[serde(default)]
    pub cycle_id: Option<String>,
    pub phase: ExplorationRunPhase,
    pub attempt: u32,
    pub max_attempts: u32,
    pub running_builds: u32,
    pub pending_builds: u32,
    #[serde(default)]
    pub current_version_id: Option<String>,
    pub summary: String,
    #[serde(default)]
    pub raw_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExplorationRunOutput {
    pub request_id: String,
    pub thread_id: String,
    #[serde(default)]
    pub cycle_id: Option<String>,
    pub phase: ExplorationRunPhase,
    pub message_id: String,
    #[serde(default)]
    pub design: Option<DesignOutput>,
    #[serde(default)]
    pub artifact_bundle: Option<ArtifactBundle>,
    #[serde(default)]
    pub model_manifest: Option<ModelManifest>,
    #[serde(default)]
    pub structural_verification: Option<StructuralVerificationResult>,
    #[serde(default)]
    pub usage: Option<UsageSummary>,
    #[serde(default)]
    pub response_text: Option<String>,
    #[serde(default)]
    pub raw_error: Option<String>,
    pub publication_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExplorationRunProjection {
    pub run: ExplorationRunOutput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StopExplorationRunInput {
    pub request_id: String,
    pub thread_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExplorationVisualEvidenceInput {
    pub request_id: String,
    pub thread_id: String,
    pub version_id: String,
    pub render_snapshot_id: String,
    pub artifact_digest: String,
    #[serde(default)]
    pub screenshots: Vec<String>,
}
