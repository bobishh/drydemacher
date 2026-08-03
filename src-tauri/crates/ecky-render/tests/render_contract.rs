use std::collections::{BTreeMap, BTreeSet};

use ecky_render::{
    ArtifactFormat, ArtifactRole, KernelArg, KernelCommand, KernelPart, KernelPlan, RenderArtifact,
    RenderAsset, RenderProduct, RenderRequest, RENDER_SCHEMA_VERSION,
};
use serde_json::json;

#[test]
fn render_contract_is_path_free_and_uses_camel_case_json() {
    let request = RenderRequest {
        schema_version: RENDER_SCHEMA_VERSION,
        source: "(result (box 10mm 20mm 30mm))".into(),
        parameters: BTreeMap::from([("wall-thickness".into(), json!(1.6))]),
        geometry_backend: "eckyRust".into(),
        requested_artifacts: BTreeSet::from([ArtifactFormat::Stl, ArtifactFormat::Step]),
        assets: BTreeMap::from([(
            "turtle".into(),
            RenderAsset {
                media_type: "model/stl".into(),
                digest: "sha256:turtle".into(),
                bytes: vec![4, 5, 6],
            },
        )]),
    };

    let request_json = serde_json::to_value(&request).expect("request serializes");
    assert_eq!(request_json["schemaVersion"], RENDER_SCHEMA_VERSION);
    assert_eq!(request_json["geometryBackend"], "eckyRust");
    assert_eq!(
        request_json["requestedArtifacts"],
        json!(["step", "stl"]),
        "ordered collections keep cache keys deterministic"
    );
    assert_eq!(request_json["assets"]["turtle"]["bytes"], json!([4, 5, 6]));

    let product = RenderProduct {
        schema_version: RENDER_SCHEMA_VERSION,
        source_digest: "sha256:source".into(),
        manifest: json!({"title": "box"}),
        artifacts: vec![RenderArtifact {
            role: ArtifactRole::Preview,
            format: ArtifactFormat::Stl,
            media_type: "model/stl".into(),
            digest: "sha256:artifact".into(),
            bytes: vec![0, 1, 2, 3],
        }],
        diagnostics: Vec::new(),
    };

    let product_json = serde_json::to_value(&product).expect("product serializes");
    let encoded = product_json.to_string();
    assert!(!encoded.contains("\"path\""));
    assert!(!encoded.contains("Path"));
    assert_eq!(product_json["artifacts"][0]["bytes"], json!([0, 1, 2, 3]));
}

#[test]
fn kernel_plan_is_a_public_versioned_protocol() {
    let plan = KernelPlan {
        schema_version: 1,
        plan_id: "plan-1".into(),
        parts: vec![KernelPart {
            key: "body".into(),
            label: "Body".into(),
            root: 2,
            representation: ecky_render::KernelRepresentation::AnalyticBrep,
            commands: vec![KernelCommand {
                output: 2,
                op: "box".into(),
                args: vec![KernelArg {
                    kind: "number".into(),
                    value: json!(10.0),
                }],
                keywords: Vec::new(),
            }],
        }],
        partial_boolean_groups: Vec::new(),
    };

    let encoded = serde_json::to_value(&plan).expect("plan serializes");
    assert_eq!(encoded["schemaVersion"], 1);
    assert_eq!(encoded["planId"], "plan-1");
    assert_eq!(encoded["parts"][0]["commands"][0]["output"], 2);

    let decoded: KernelPlan = serde_json::from_value(encoded).expect("plan deserializes");
    assert_eq!(decoded, plan);
}
