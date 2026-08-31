use serde::{Deserialize, Serialize};
use specta::Type;

use super::{
    AdvisoryCondition, AdvisorySeverity, ControlPrimitiveKind, ControlRelationMode,
    ControlViewScope, ControlViewSection, EnrichmentStatus, ModelManifest, PrimitiveBinding,
};

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProposalStatusEdit {
    pub proposal_id: String,
    pub status: EnrichmentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SemanticManifestEditIntent {
    #[specta(rename_all = "camelCase")]
    SaveView {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        view_id: Option<String>,
        label: String,
        scope: ControlViewScope,
        #[serde(default)]
        part_ids: Vec<String>,
        #[serde(default)]
        primitive_ids: Vec<String>,
        #[serde(default)]
        sections: Vec<ControlViewSection>,
        #[serde(default, rename = "default")]
        #[specta(rename = "default")]
        is_default: bool,
    },
    #[specta(rename_all = "camelCase")]
    DeleteView { view_id: String },
    #[specta(rename_all = "camelCase")]
    SavePrimitive {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        primitive_id: Option<String>,
        label: String,
        primitive_kind: ControlPrimitiveKind,
        scope: ControlViewScope,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        part_id: Option<String>,
        bindings: Vec<PrimitiveBinding>,
        #[serde(default)]
        attach_to_view: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base_view_id: Option<String>,
    },
    #[specta(rename_all = "camelCase")]
    DeletePrimitive { primitive_id: String },
    #[specta(rename_all = "camelCase")]
    SaveAdvisory {
        label: String,
        severity: AdvisorySeverity,
        primitive_ids: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        view_id: Option<String>,
        message: String,
        condition: AdvisoryCondition,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        threshold: Option<f64>,
    },
    #[specta(rename_all = "camelCase")]
    DeleteAdvisory { advisory_id: String },
    #[specta(rename_all = "camelCase")]
    SaveRelation {
        source_primitive_id: String,
        target_primitive_id: String,
        mode: ControlRelationMode,
        #[serde(default = "default_scale")]
        scale: f64,
        #[serde(default)]
        offset: f64,
    },
    #[specta(rename_all = "camelCase")]
    DeleteRelation { relation_id: String },
    #[specta(rename_all = "camelCase")]
    SetProposalStatus {
        proposal_id: String,
        status: EnrichmentStatus,
    },
    #[specta(rename_all = "camelCase")]
    SetProposalStatuses { entries: Vec<ProposalStatusEdit> },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApplySemanticManifestEditInput {
    pub model_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    pub edit: SemanticManifestEditIntent,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SemanticManifestEditResult {
    pub manifest: ModelManifest,
    pub edited_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_view_id: Option<String>,
}

fn default_scale() -> f64 {
    1.0
}
