use tauri::AppHandle;

use crate::contracts::{
    AppResult, ArtifactBundle, AuthoringGraph, AuthoringGraphRequest, ModelManifest,
};

#[tauri::command]
#[specta::specta]
pub async fn get_authoring_graph(
    request: AuthoringGraphRequest,
    app: AppHandle,
) -> AppResult<AuthoringGraph> {
    let (manifest, artifact_bundle): (Option<ModelManifest>, Option<ArtifactBundle>) =
        match request.model_id.as_deref() {
            Some(model_id) => (
                Some(crate::model_runtime::read_model_manifest(&app, model_id)?),
                Some(crate::model_runtime::read_artifact_bundle(&app, model_id)?),
            ),
            None => (None, None),
        };

    crate::services::authoring_graph::build_authoring_graph(
        &request.source,
        manifest.as_ref(),
        artifact_bundle.as_ref(),
    )
}

#[cfg(test)]
mod tests {
    use crate::contracts::{AuthoringGraphAstNode, AuthoringGraphRequest};

    #[test]
    fn request_and_graph_nodes_serialize_with_camel_case_boundary_names() {
        let request = serde_json::to_value(AuthoringGraphRequest {
            source: "(model (part body (box 1 1 1)))".to_string(),
            model_id: Some("model-1".to_string()),
        })
        .expect("serialize request");
        let node = serde_json::to_value(AuthoringGraphAstNode {
            path: "/parts/body".to_string(),
            stable_node_key: "sha256:key".to_string(),
            kind: "Part".to_string(),
            value_kind: "Part".to_string(),
            operation: None,
            part_id: Some("body".to_string()),
            source_addressable: true,
            editable_ops: vec!["replace".to_string()],
            non_editable_reason: None,
            child_paths: Vec::new(),
            input_ports: Vec::new(),
        })
        .expect("serialize node");

        assert_eq!(request["modelId"], "model-1");
        assert!(request.get("model_id").is_none());
        assert_eq!(node["stableNodeKey"], "sha256:key");
        assert_eq!(node["partId"], "body");
        assert_eq!(node["sourceAddressable"], true);
        assert!(node.get("stable_node_key").is_none());
    }
}
