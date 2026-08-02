use serde::{Deserialize, Serialize};
use specta::Type;

use super::SelectionTargetKind;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringGraphRequest {
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringGraphAstNode {
    pub path: String,
    pub stable_node_key: String,
    pub kind: String,
    pub value_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringGraphFeature {
    pub feature_id: String,
    pub kind: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_stable_node_key: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependency_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringGraphDependency {
    pub parameter_key: String,
    pub parameter_stable_node_key: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependent_source_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affected_stable_node_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub impacted_part_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feature_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringGraphConstraint {
    pub constraint_id: String,
    pub label: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameter_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affected_stable_node_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringGraphTarget {
    pub target_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub durable_target_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_target_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alias_ids: Vec<String>,
    pub part_id: String,
    pub viewer_node_id: String,
    pub label: String,
    pub kind: SelectionTargetKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameter_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub primitive_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feature_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_stable_node_keys: Vec<String>,
    pub editable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub non_editable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringGraphHandle {
    pub handle_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameter_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringGraph {
    pub source_digest: String,
    pub core_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_digest: Option<String>,
    pub ast_nodes: Vec<AuthoringGraphAstNode>,
    pub features: Vec<AuthoringGraphFeature>,
    pub dependencies: Vec<AuthoringGraphDependency>,
    pub constraints: Vec<AuthoringGraphConstraint>,
    pub targets: Vec<AuthoringGraphTarget>,
    pub handles: Vec<AuthoringGraphHandle>,
}
