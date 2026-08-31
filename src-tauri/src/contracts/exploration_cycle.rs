//! Public data contract for the exploration controller.
//!
//! The controller owns orchestration state. Version identity remains owned by
//! the immutable history model; this contract only carries references to it.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CyclePhase {
    Idle,
    Planning,
    Building,
    Verifying,
    Deciding,
    AwaitingInput,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CycleStatus {
    Active,
    Interrupted,
    Completed,
    Stopped,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PlanAction {
    Build,
    Ask,
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Decision {
    Complete,
    Replan,
    #[specta(rename_all = "camelCase")]
    Ask {
        question: String,
        blocked_decision: String,
    },
    Stop,
    Compare,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VerificationVerdict {
    Green,
    Red,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlanProposal {
    pub action: PlanAction,
    pub source_version_id: String,
    pub hypothesis: String,
    pub change_scope: String,
    pub expected_evidence: String,
    pub budget_cost: u32,
    #[serde(default)]
    pub question: Option<String>,
    #[serde(default)]
    pub blocked_decision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Verification {
    pub version_id: String,
    pub input_digest: String,
    pub evidence_ref: String,
    pub deterministic: VerificationVerdict,
    #[serde(default)]
    pub vision: Option<VerificationVerdict>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CycleState {
    pub cycle_id: String,
    pub thread_id: String,
    pub phase: CyclePhase,
    pub status: CycleStatus,
    pub current_version_id: String,
    #[serde(default)]
    pub chosen_version_id: Option<String>,
    pub pending_question: Option<String>,
    #[serde(default)]
    pub pending_blocked_decision: Option<String>,
    #[serde(default)]
    pub last_answer: Option<String>,
    #[serde(default)]
    pub last_evidence_ref: Option<String>,
    pub budget: u32,
    pub budget_used: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CycleEvent {
    pub event_id: String,
    pub cycle_id: String,
    pub sequence: u64,
    pub event_type: CycleEventType,
    pub phase: CyclePhase,
    #[serde(default)]
    pub source_version_id: Option<String>,
    #[serde(default)]
    pub result_version_id: Option<String>,
    #[serde(default)]
    pub evidence_ref: Option<String>,
    #[serde(default)]
    pub raw_error: Option<String>,
    #[serde(default)]
    pub render_snapshot_id: Option<String>,
    #[serde(default)]
    pub artifact_digest: Option<String>,
    #[serde(default)]
    pub route: Option<CycleRouteMetadata>,
    #[serde(default)]
    pub plan: Option<PlanProposal>,
    #[serde(default)]
    pub question: Option<String>,
    #[serde(default)]
    pub blocked_decision: Option<String>,
    #[serde(default)]
    pub answer: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CycleEventType {
    Started,
    PlanAccepted,
    BuildStarted,
    BuildUnchanged,
    VersionAppended,
    VerificationRecorded,
    DecisionRecorded,
    ProviderFailed,
    ModelCallRecorded,
    Answered,
    Stopped,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CycleRouteMetadata {
    pub prompt_version: String,
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub latency_ms: Option<u64>,
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub estimated_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CycleDefinition {
    pub objective: String,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    #[serde(default)]
    pub hard_constraints: Vec<String>,
    #[serde(default)]
    pub soft_preferences: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CyclePacket {
    pub state: CycleState,
    pub base_version_id: String,
    pub definition: CycleDefinition,
    #[serde(default)]
    pub hypothesis: Option<String>,
    #[serde(default)]
    pub last_verification: Option<Verification>,
    #[serde(default)]
    pub last_route: Option<CycleRouteMetadata>,
    pub event_count: u64,
    pub prompt_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartCycleInput {
    pub thread_id: String,
    pub base_version_id: String,
    pub objective: String,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    #[serde(default)]
    pub hard_constraints: Vec<String>,
    #[serde(default)]
    pub soft_preferences: Vec<String>,
    pub budget: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CycleNextAction {
    #[specta(rename_all = "camelCase")]
    Plan { proposal: PlanProposal },
    #[specta(rename_all = "camelCase")]
    BuildStarted { source_version_id: String },
    #[specta(rename_all = "camelCase")]
    VersionAppended { result_version_id: String },
    #[specta(rename_all = "camelCase")]
    Verify {
        verification: Verification,
        #[serde(default)]
        raw_error: Option<String>,
        #[serde(default)]
        render_snapshot_id: Option<String>,
        #[serde(default)]
        artifact_digest: Option<String>,
    },
    #[specta(rename_all = "camelCase")]
    Decide { decision: Decision },
    #[specta(rename_all = "camelCase")]
    ProviderFailed { raw_error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CycleNextInput {
    pub cycle_id: String,
    pub action: CycleNextAction,
    #[serde(default)]
    pub route: Option<CycleRouteMetadata>,
}

#[cfg(test)]
mod tests {
    use super::{CycleNextAction, Decision, Verification, VerificationVerdict};
    use serde_json::json;

    #[test]
    fn cycle_action_payload_serializes_variant_fields_as_camel_case() {
        let build = serde_json::to_value(CycleNextAction::BuildStarted {
            source_version_id: "version-a".into(),
        })
        .expect("serialize build action");
        assert_eq!(
            build,
            json!({ "kind": "buildStarted", "sourceVersionId": "version-a" })
        );

        let verify = serde_json::to_value(CycleNextAction::Verify {
            verification: Verification {
                version_id: "version-b".into(),
                input_digest: "digest-b".into(),
                evidence_ref: "evidence-b".into(),
                deterministic: VerificationVerdict::Green,
                vision: None,
            },
            raw_error: Some("provider body".into()),
            render_snapshot_id: Some("snapshot-b".into()),
            artifact_digest: Some("digest-b".into()),
        })
        .expect("serialize verify action");
        assert_eq!(verify["rawError"], "provider body");
        assert_eq!(verify["renderSnapshotId"], "snapshot-b");
        assert_eq!(verify["artifactDigest"], "digest-b");
        assert!(verify.get("raw_error").is_none());
    }

    #[test]
    fn decision_payload_serializes_variant_fields_as_camel_case() {
        let decision = serde_json::to_value(Decision::Ask {
            question: "Keep face fixed?".into(),
            blocked_decision: "mounting orientation".into(),
        })
        .expect("serialize decision");

        assert_eq!(
            decision,
            json!({
                "ask": {
                    "question": "Keep face fixed?",
                    "blockedDecision": "mounting orientation"
                }
            })
        );
    }
}
