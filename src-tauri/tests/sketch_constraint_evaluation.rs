use ecky_cad_lib::commands::sketch::evaluate_sketch_document_constraints;
use ecky_cad_lib::contracts::{
    SketchConstraint, SketchConstraintKind, SketchDefinition, SketchDocument, SketchPrimitive,
    SketchPrimitiveKind, SketchView,
};
use ecky_cad_lib::services::sketch_constraint_validation::{
    auto_repair_dimension_geometry, repair_dimension_constraint_values, validate_constraints,
    SketchConstraintEvaluationMode, SketchConstraintEvaluationRequest,
    SketchConstraintEvaluationResponse,
};

fn document_with_constraints(constraints: Vec<SketchConstraint>) -> SketchDocument {
    SketchDocument {
        document_id: "doc-test".to_string(),
        sketches: vec![SketchDefinition {
            sketch_id: "sketch-front".to_string(),
            view: SketchView::Front,
            plane: None,
            primitives: vec![SketchPrimitive {
                primitive_id: "primitive-front-1".to_string(),
                kind: SketchPrimitiveKind::Polyline,
                points: vec![
                    [10.0, 20.0],
                    [60.0, 20.0],
                    [60.0, 45.0],
                    [10.0, 45.0],
                    [10.0, 20.0],
                ],
                closed: true,
                radius: None,
                topology: None,
                provenance: None,
            }],
            constraints,
        }],
        active_sketch_id: None,
        units: Some("mm".to_string()),
        metadata: None,
    }
}

fn dimension(constraint_id: &str, value: Option<f64>) -> SketchConstraint {
    SketchConstraint {
        constraint_id: constraint_id.to_string(),
        kind: SketchConstraintKind::Dimension,
        target_ids: vec!["primitive-front-1".to_string()],
        value,
    }
}

#[test]
fn validates_and_repairs_dimension_constraints_without_mutating_input() {
    let source = document_with_constraints(vec![
        dimension("primitive-front-1-width-dimension", Some(99.0)),
        dimension("primitive-front-1-height-dimension", Some(25.0)),
    ]);

    let validation = validate_constraints(&source);
    assert!(!validation.passed);
    assert_eq!(
        validation.issues,
        ["sketch 'sketch-front' primitive 'primitive-front-1' width dimension expected 99mm but measured 50mm."]
    );

    let repaired = repair_dimension_constraint_values(&source).expect("repair stale value");
    assert_eq!(source.sketches[0].constraints[0].value, Some(99.0));
    assert_eq!(
        repaired.document.sketches[0].constraints[0].value,
        Some(50.0)
    );
    assert_eq!(
        repaired.evidence,
        ["sketch 'sketch-front' primitive 'primitive-front-1' width dimension repaired 99mm -> 50mm."]
    );
}

#[test]
fn auto_repairs_only_small_geometry_delta_and_preserves_closed_point() {
    let source = document_with_constraints(vec![dimension(
        "primitive-front-1-width-dimension",
        Some(50.4),
    )]);

    let repaired = auto_repair_dimension_geometry(&source, 1.0);
    assert_eq!(source.sketches[0].primitives[0].points[0], [10.0, 20.0]);
    assert_eq!(
        repaired.document.sketches[0].primitives[0].points,
        vec![
            [9.8, 20.0],
            [60.2, 20.0],
            [60.2, 45.0],
            [9.8, 45.0],
            [9.8, 20.0],
        ]
    );
    assert_eq!(repaired.evidence[0].primitive_id, "primitive-front-1");
    assert_eq!(
        repaired.evidence[0].detail,
        "width dimension 50mm -> 50.4mm"
    );
    assert!(validate_constraints(&repaired.document).passed);
}

#[test]
fn preserves_raw_structural_issues_and_rejects_noop_value_repair() {
    let mut missing_target = dimension("primitive-front-1-height-dimension", Some(25.0));
    missing_target.target_ids = vec!["primitive-front-missing".to_string()];
    let source = document_with_constraints(vec![
        dimension("primitive-front-1-width-dimension", None),
        missing_target,
        dimension("primitive-front-1-depth-dimension", Some(12.0)),
    ]);

    let validation = validate_constraints(&source);
    assert_eq!(
        validation.issues,
        [
            "sketch 'sketch-front' dimension constraint 'primitive-front-1-width-dimension' has missing or non-finite value.",
            "sketch 'sketch-front' dimension constraint 'primitive-front-1-height-dimension' targets missing primitive 'primitive-front-missing'.",
            "sketch 'sketch-front' dimension constraint 'primitive-front-1-depth-dimension' is neither width nor height.",
        ]
    );
    assert_eq!(
        repair_dimension_constraint_values(&source),
        Err(validation.issues.join(" "))
    );

    let matching = document_with_constraints(vec![dimension(
        "primitive-front-1-width-dimension",
        Some(50.0),
    )]);
    assert_eq!(
        repair_dimension_constraint_values(&matching),
        Err("No repairable dimension constraint mismatch.".to_string())
    );
}

#[test]
fn leaves_large_geometry_delta_for_explicit_repair() {
    let source = document_with_constraints(vec![dimension(
        "primitive-front-1-width-dimension",
        Some(99.0),
    )]);
    let repaired = auto_repair_dimension_geometry(&source, 1.0);
    assert!(repaired.evidence.is_empty());
    assert_eq!(repaired.document, source);
}

#[tokio::test]
async fn tauri_command_projects_validation_through_one_rust_boundary() {
    let response = evaluate_sketch_document_constraints(SketchConstraintEvaluationRequest {
        document: document_with_constraints(vec![dimension(
            "primitive-front-1-width-dimension",
            Some(50.0),
        )]),
        mode: SketchConstraintEvaluationMode::Validate,
        max_delta: None,
    })
    .await
    .expect("command validation");

    assert!(matches!(
        response,
        SketchConstraintEvaluationResponse::Validation {
            passed: true,
            evidence,
            issues,
        } if evidence == ["sketch 'sketch-front' primitive 'primitive-front-1' width dimension matched 50mm."]
            && issues.is_empty()
    ));
}

#[test]
fn tauri_payload_serializes_only_camel_case_boundary_fields() {
    let request = SketchConstraintEvaluationRequest {
        document: document_with_constraints(Vec::new()),
        mode: SketchConstraintEvaluationMode::AutoRepairGeometry,
        max_delta: Some(0.5),
    };
    let value = serde_json::to_value(request).expect("serialize request");
    assert_eq!(value["mode"], "autoRepairGeometry");
    assert_eq!(value["maxDelta"], 0.5);
    assert!(value.get("max_delta").is_none());

    let response = SketchConstraintEvaluationResponse::GeometryRepaired {
        document: document_with_constraints(Vec::new()),
        evidence: vec![ecky_cad_lib::services::sketch_constraint_validation::SketchConstraintGeometryRepairEvidence {
            primitive_id: "primitive-front-1".to_string(),
            detail: "width dimension 50mm -> 50.4mm".to_string(),
        }],
    };
    let value = serde_json::to_value(response).expect("serialize response");
    assert_eq!(value["kind"], "geometryRepaired");
    assert_eq!(value["evidence"][0]["primitiveId"], "primitive-front-1");
    assert!(value["evidence"][0].get("primitive_id").is_none());
}
