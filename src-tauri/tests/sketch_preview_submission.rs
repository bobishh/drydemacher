use ecky_cad_lib::contracts::{SketchDocument, SketchView};
use ecky_cad_lib::services::sketch_preview_submission::{
    SketchPreviewMode, SketchPreviewSubmissionRequest, SketchPreviewTarget,
};

#[test]
fn sketch_preview_submission_contract_serializes_camel_case() {
    let value = serde_json::to_value(SketchPreviewSubmissionRequest {
        target: SketchPreviewTarget {
            target_id: "sketch-workspace".to_string(),
            part_id: "sketch-preview".to_string(),
        },
        document: SketchDocument {
            document_id: "document-1".to_string(),
            active_sketch_id: None,
            units: Some("mm".to_string()),
            metadata: None,
            sketches: Vec::new(),
        },
        mode: SketchPreviewMode::Auto,
    })
    .expect("request json");

    assert_eq!(value["target"]["targetId"], "sketch-workspace");
    assert_eq!(value["target"]["partId"], "sketch-preview");
    assert_eq!(value["document"]["documentId"], "document-1");
    assert_eq!(value["mode"], "auto");
    assert!(value["target"].get("target_id").is_none());
    let _: SketchView = SketchView::Front;
}
