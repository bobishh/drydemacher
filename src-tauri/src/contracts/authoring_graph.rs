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
pub struct AuthoringGraphInputPort {
    pub role: String,
    pub value_kind: String,
    pub cardinality: String,
    pub child_path: String,
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
    pub source_addressable: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub editable_ops: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub non_editable_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub child_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_ports: Vec<AuthoringGraphInputPort>,
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

impl AuthoringGraph {
    pub fn validate(&self) -> Result<(), String> {
        require_text(&self.source_digest, "Authoring graph sourceDigest")?;
        require_text(&self.core_digest, "Authoring graph coreDigest")?;

        for node in &self.ast_nodes {
            let owner = format!("Authoring graph AST node '{}'", node.path);
            require_text(&node.path, "Authoring graph AST node path")?;
            require_text(&node.stable_node_key, &format!("{owner} stableNodeKey"))?;
            require_text(&node.kind, &format!("{owner} kind"))?;
            require_text(&node.value_kind, &format!("{owner} valueKind"))?;
            match (
                node.source_addressable,
                normalized_reason(&node.non_editable_reason),
            ) {
                (true, Some(_)) => {
                    return Err(format!(
                        "{owner} cannot include nonEditableReason when source-addressable"
                    ));
                }
                (false, None) => {
                    return Err(format!(
                        "{owner} requires raw non-editable reason when non-addressable"
                    ));
                }
                _ => {}
            }
            for port in &node.input_ports {
                require_text(&port.role, &format!("{owner} input port role"))?;
                require_text(
                    &port.value_kind,
                    &format!("{owner} input port '{}' valueKind", port.role),
                )?;
                require_text(
                    &port.child_path,
                    &format!("{owner} input port '{}' childPath", port.role),
                )?;
                if !matches!(port.cardinality.as_str(), "one" | "many") {
                    return Err(format!(
                        "{owner} input port '{}' cardinality must be one or many",
                        port.role
                    ));
                }
            }
        }

        for target in &self.targets {
            let owner = format!("Authoring graph target '{}'", target.target_id);
            require_text(&target.target_id, "Authoring graph targetId")?;
            require_text(&target.part_id, &format!("{owner} partId"))?;
            require_text(&target.viewer_node_id, &format!("{owner} viewerNodeId"))?;
            require_text(&target.label, &format!("{owner} label"))?;
            let reason = normalized_reason(&target.non_editable_reason);
            if target.editable {
                if target.feature_ids.is_empty() || target.source_stable_node_keys.is_empty() {
                    return Err(format!(
                        "{owner} is editable but lacks exact feature and source bindings"
                    ));
                }
                if reason.is_some() {
                    return Err(format!(
                        "{owner} cannot include nonEditableReason when editable"
                    ));
                }
            } else if reason.is_none() {
                return Err(format!("{owner} requires raw non-editable reason"));
            }
        }

        Ok(())
    }
}

fn require_text(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{field} must be non-empty"))
    } else {
        Ok(())
    }
}

fn normalized_reason(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph_with_target(target: AuthoringGraphTarget) -> AuthoringGraph {
        AuthoringGraph {
            source_digest: "source".to_string(),
            core_digest: "core".to_string(),
            artifact_digest: None,
            ast_nodes: Vec::new(),
            features: Vec::new(),
            dependencies: Vec::new(),
            constraints: Vec::new(),
            targets: vec![target],
            handles: Vec::new(),
        }
    }

    fn target(editable: bool) -> AuthoringGraphTarget {
        AuthoringGraphTarget {
            target_id: "part:body".to_string(),
            durable_target_id: None,
            canonical_target_id: None,
            alias_ids: Vec::new(),
            part_id: "body".to_string(),
            viewer_node_id: "body".to_string(),
            label: "Body".to_string(),
            kind: SelectionTargetKind::Part,
            parameter_keys: Vec::new(),
            primitive_ids: Vec::new(),
            feature_ids: Vec::new(),
            source_stable_node_keys: Vec::new(),
            editable,
            non_editable_reason: None,
        }
    }

    #[test]
    fn editable_target_requires_exact_feature_and_source_bindings() {
        let error = graph_with_target(target(true))
            .validate()
            .expect_err("editable target without exact bindings must fail");

        assert!(error.contains("part:body"));
        assert!(error.contains("feature"));
        assert!(error.contains("source"));
    }

    #[test]
    fn non_editable_target_requires_raw_reason() {
        let error = graph_with_target(target(false))
            .validate()
            .expect_err("non-editable target without reason must fail");

        assert!(error.contains("part:body"));
        assert!(error.contains("reason"));
    }
}
