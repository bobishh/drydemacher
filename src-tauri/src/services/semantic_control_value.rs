use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::contracts::{
    AppError, AppResult, ControlPrimitive, ControlRelationMode, DesignParams, ModelManifest,
    ModelSourceKind, ParamValue, SourceLanguage, UiField, UiSpec,
};
use crate::models::{AppState, PathResolver};

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApplySemanticControlValueInput {
    pub thread_id: String,
    pub target_message_id: String,
    pub primitive_id: String,
    pub value: ParamValue,
}

#[derive(Debug, Clone, Serialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApplySemanticControlValueResult {
    pub parameter_patch: DesignParams,
    pub changed_parameter_keys: Vec<String>,
    pub applied_primitive_ids: Vec<String>,
}

pub fn resolve_semantic_control_value(
    manifest: &ModelManifest,
    ui_spec: &UiSpec,
    primitive_id: &str,
    value: ParamValue,
) -> AppResult<ApplySemanticControlValueResult> {
    let primitive_id = primitive_id.trim();
    if primitive_id.is_empty() {
        return Err(AppError::validation(
            "Semantic primitive id cannot be empty.",
        ));
    }

    if let Some(parameter_key) = primitive_id.strip_prefix("ast-param:") {
        return resolve_ast_parameter(manifest, ui_spec, primitive_id, parameter_key, value);
    }

    let root = match find_primitive(manifest, primitive_id) {
        Ok(root) => root,
        Err(_) => {
            return resolve_derived_field_primitive(manifest, ui_spec, primitive_id, value);
        }
    };
    if !root.editable {
        return Err(AppError::validation(format!(
            "Semantic primitive '{}' is not editable.",
            primitive_id
        )));
    }

    let mut parameter_patch = DesignParams::new();
    let mut applied_primitive_ids = Vec::new();
    let mut visited = std::collections::HashSet::from([primitive_id.to_string()]);
    let mut queue = VecDeque::from([(primitive_id.to_string(), value)]);

    while let Some((current_id, current_value)) = queue.pop_front() {
        let primitive = find_primitive(manifest, &current_id)?;
        apply_primitive_bindings(primitive, ui_spec, &current_value, &mut parameter_patch)?;
        applied_primitive_ids.push(current_id.clone());

        for relation in manifest
            .control_relations
            .iter()
            .filter(|relation| relation.enabled && relation.source_primitive_id == current_id)
        {
            if visited.contains(&relation.target_primitive_id) {
                continue;
            }
            let target_value = apply_relation_value(
                &current_value,
                relation.mode.clone(),
                relation.scale,
                relation.offset,
            )?;
            visited.insert(relation.target_primitive_id.clone());
            queue.push_back((relation.target_primitive_id.clone(), target_value));
        }
    }

    if parameter_patch.is_empty() {
        return Err(AppError::validation(format!(
            "Semantic primitive '{}' has no bindings to declared parameters.",
            primitive_id
        )));
    }
    let changed_parameter_keys = parameter_patch.keys().cloned().collect();
    Ok(ApplySemanticControlValueResult {
        parameter_patch,
        changed_parameter_keys,
        applied_primitive_ids,
    })
}

pub async fn apply_semantic_control_value(
    input: ApplySemanticControlValueInput,
    state: &AppState,
    app: &dyn PathResolver,
) -> AppResult<ApplySemanticControlValueResult> {
    let target = {
        let conn = state.db.lock().await;
        crate::services::target::resolve_target(
            &conn,
            app,
            Some(input.thread_id),
            Some(input.target_message_id),
        )?
    };
    let design = target
        .design
        .ok_or_else(|| AppError::validation("Semantic control target has no design output."))?;
    let manifest = target
        .model_manifest
        .ok_or_else(|| AppError::validation("Semantic control target has no model manifest."))?;
    resolve_semantic_control_value(&manifest, &design.ui_spec, &input.primitive_id, input.value)
}

fn resolve_ast_parameter(
    manifest: &ModelManifest,
    ui_spec: &UiSpec,
    primitive_id: &str,
    parameter_key: &str,
    value: ParamValue,
) -> AppResult<ApplySemanticControlValueResult> {
    if manifest.source_kind != ModelSourceKind::Generated
        || manifest.source_language != SourceLanguage::EckyIrV0
    {
        return Err(AppError::validation(format!(
            "AST semantic primitive '{}' is only valid for generated Ecky models.",
            primitive_id
        )));
    }
    let parameter_key = parameter_key.trim();
    if parameter_key.is_empty() {
        return Err(AppError::validation(
            "AST semantic primitive must include a parameter key.",
        ));
    }
    let field = find_field(ui_spec, parameter_key).ok_or_else(|| {
        AppError::validation(format!(
            "AST semantic primitive '{}' references undeclared parameter '{}'.",
            primitive_id, parameter_key
        ))
    })?;
    if field.frozen() {
        return Err(AppError::validation(format!(
            "AST semantic primitive '{}' is not editable.",
            primitive_id
        )));
    }
    field.validate_value(&value)?;
    let parameter_patch = DesignParams::from([(parameter_key.to_string(), value)]);
    Ok(ApplySemanticControlValueResult {
        changed_parameter_keys: vec![parameter_key.to_string()],
        applied_primitive_ids: vec![primitive_id.to_string()],
        parameter_patch,
    })
}

fn resolve_derived_field_primitive(
    manifest: &ModelManifest,
    ui_spec: &UiSpec,
    primitive_id: &str,
    value: ParamValue,
) -> AppResult<ApplySemanticControlValueResult> {
    if manifest.source_language == SourceLanguage::EckyIrV0 {
        return Err(AppError::validation(format!(
            "Semantic primitive '{}' was not found.",
            primitive_id
        )));
    }
    let matches = ui_spec
        .fields
        .iter()
        .filter(|field| format!("primitive-{}", slugify(field.key())) == primitive_id)
        .collect::<Vec<_>>();
    let field = match matches.as_slice() {
        [field] => *field,
        [] => {
            return Err(AppError::validation(format!(
                "Semantic primitive '{}' was not found.",
                primitive_id
            )))
        }
        _ => {
            return Err(AppError::validation(format!(
                "Semantic primitive '{}' is ambiguous across declared parameters.",
                primitive_id
            )))
        }
    };
    if field.frozen() {
        return Err(AppError::validation(format!(
            "Semantic primitive '{}' is not editable.",
            primitive_id
        )));
    }
    let raw_value = coerce_field_value(field, &value, primitive_id)?;
    field.validate_value(&raw_value)?;
    let parameter_key = field.key().to_string();
    Ok(ApplySemanticControlValueResult {
        parameter_patch: DesignParams::from([(parameter_key.clone(), raw_value)]),
        changed_parameter_keys: vec![parameter_key],
        applied_primitive_ids: vec![primitive_id.to_string()],
    })
}

fn find_primitive<'a>(
    manifest: &'a ModelManifest,
    primitive_id: &str,
) -> AppResult<&'a ControlPrimitive> {
    manifest
        .control_primitives
        .iter()
        .find(|primitive| primitive.primitive_id == primitive_id)
        .ok_or_else(|| {
            AppError::validation(format!(
                "Semantic primitive '{}' was not found.",
                primitive_id
            ))
        })
}

fn find_field<'a>(ui_spec: &'a UiSpec, parameter_key: &str) -> Option<&'a UiField> {
    ui_spec
        .fields
        .iter()
        .find(|field| field.key() == parameter_key)
}

fn apply_primitive_bindings(
    primitive: &ControlPrimitive,
    ui_spec: &UiSpec,
    value: &ParamValue,
    patch: &mut DesignParams,
) -> AppResult<()> {
    for binding in &primitive.bindings {
        let Some(field) = find_field(ui_spec, &binding.parameter_key) else {
            continue;
        };
        let raw_value = match field {
            UiField::Checkbox { .. } | UiField::Select { .. } | UiField::Image { .. } => {
                coerce_field_value(field, value, &primitive.primitive_id)?
            }
            UiField::Range { .. } | UiField::Number { .. } => {
                let semantic_value = coerce_number(value, &primitive.primitive_id)?;
                let scale = if binding.scale == 0.0 {
                    1.0
                } else {
                    binding.scale
                };
                let mut raw = semantic_value * scale + binding.offset;
                if let Some(min) = binding.min {
                    raw = raw.max(min);
                }
                if let Some(max) = binding.max {
                    raw = raw.min(max);
                }
                if !raw.is_finite() {
                    return Err(AppError::validation(format!(
                        "Semantic primitive '{}' produced a non-finite value for parameter '{}'.",
                        primitive.primitive_id, binding.parameter_key
                    )));
                }
                ParamValue::Number(raw)
            }
        };
        field.validate_value(&raw_value)?;
        patch.insert(binding.parameter_key.clone(), raw_value);
    }
    Ok(())
}

fn coerce_field_value(field: &UiField, value: &ParamValue, owner: &str) -> AppResult<ParamValue> {
    match field {
        UiField::Checkbox { .. } => Ok(ParamValue::Boolean(coerce_boolean(value))),
        UiField::Select { .. } | UiField::Image { .. } => Ok(value.clone()),
        UiField::Range { .. } | UiField::Number { .. } => {
            Ok(ParamValue::Number(coerce_number(value, owner)?))
        }
    }
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for character in value.trim().to_ascii_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(character);
            pending_dash = false;
        } else if !slug.is_empty() {
            pending_dash = true;
        }
    }
    slug
}

fn apply_relation_value(
    source: &ParamValue,
    mode: ControlRelationMode,
    scale: f64,
    offset: f64,
) -> AppResult<ParamValue> {
    match mode {
        ControlRelationMode::Mirror => Ok(source.clone()),
        ControlRelationMode::Scale => Ok(ParamValue::Number(
            coerce_number(source, "relation source")? * scale,
        )),
        ControlRelationMode::Offset => Ok(ParamValue::Number(
            coerce_number(source, "relation source")? + offset,
        )),
    }
}

fn coerce_boolean(value: &ParamValue) -> bool {
    match value {
        ParamValue::Boolean(value) => *value,
        ParamValue::Number(value) => *value != 0.0 && !value.is_nan(),
        ParamValue::String(value) => !value.is_empty(),
        ParamValue::Null => false,
    }
}

fn coerce_number(value: &ParamValue, owner: &str) -> AppResult<f64> {
    let numeric = match value {
        ParamValue::Number(value) => *value,
        ParamValue::Boolean(value) => u8::from(*value) as f64,
        ParamValue::String(value) if value.trim().is_empty() => 0.0,
        ParamValue::String(value) => value.trim().parse::<f64>().map_err(|_| {
            AppError::validation(format!(
                "Semantic primitive '{}' requires a numeric value.",
                owner
            ))
        })?,
        ParamValue::Null => 0.0,
    };
    if !numeric.is_finite() {
        return Err(AppError::validation(format!(
            "Semantic primitive '{}' requires a finite numeric value.",
            owner
        )));
    }
    Ok(numeric)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn imported_manifest() -> ModelManifest {
        serde_json::from_value(serde_json::json!({
            "modelId": "model-1",
            "sourceKind": "importedFcstd",
            "sourceLanguage": "legacyPython",
            "document": {
                "documentName": "Model",
                "documentLabel": "Model",
                "objectCount": 1,
                "warnings": []
            },
            "controlPrimitives": [
                {
                    "primitiveId": "width-control",
                    "label": "Width",
                    "kind": "number",
                    "source": "generated",
                    "bindings": [{
                        "parameterKey": "width",
                        "scale": 2.0,
                        "offset": 1.0,
                        "min": 0.0,
                        "max": 10.0
                    }],
                    "editable": true,
                    "order": 1
                },
                {
                    "primitiveId": "depth-control",
                    "label": "Depth",
                    "kind": "number",
                    "source": "generated",
                    "bindings": [{
                        "parameterKey": "depth",
                        "scale": 1.0,
                        "offset": 0.0,
                        "min": 1.0,
                        "max": 8.0
                    }],
                    "editable": true,
                    "order": 2
                },
                {
                    "primitiveId": "height-control",
                    "label": "Height",
                    "kind": "number",
                    "source": "generated",
                    "bindings": [{
                        "parameterKey": "height",
                        "scale": 1.0,
                        "offset": 0.0
                    }],
                    "editable": true,
                    "order": 3
                }
            ],
            "controlRelations": [
                {
                    "relationId": "width-depth",
                    "sourcePrimitiveId": "width-control",
                    "targetPrimitiveId": "depth-control",
                    "mode": "scale",
                    "scale": 0.5,
                    "offset": 0.0,
                    "enabled": true
                },
                {
                    "relationId": "depth-height",
                    "sourcePrimitiveId": "depth-control",
                    "targetPrimitiveId": "height-control",
                    "mode": "offset",
                    "scale": 1.0,
                    "offset": 3.0,
                    "enabled": true
                }
            ],
            "taggedAnchors": {},
            "analysisDeclarations": []
        }))
        .expect("manifest")
    }

    fn numeric_ui_spec() -> UiSpec {
        serde_json::from_value(serde_json::json!({
            "fields": [
                { "type": "number", "key": "width", "label": "Width" },
                { "type": "number", "key": "depth", "label": "Depth" },
                { "type": "number", "key": "height", "label": "Height" }
            ]
        }))
        .expect("ui spec")
    }

    #[test]
    fn semantic_value_clamps_bindings_and_propagates_relations_in_rust() {
        let result = resolve_semantic_control_value(
            &imported_manifest(),
            &numeric_ui_spec(),
            "width-control",
            ParamValue::Number(8.0),
        )
        .expect("semantic patch");

        assert_eq!(
            result.parameter_patch.get("width"),
            Some(&ParamValue::Number(10.0))
        );
        assert_eq!(
            result.parameter_patch.get("depth"),
            Some(&ParamValue::Number(4.0))
        );
        assert_eq!(
            result.parameter_patch.get("height"),
            Some(&ParamValue::Number(7.0))
        );
        assert_eq!(
            result.changed_parameter_keys,
            vec!["depth", "height", "width"]
        );
        assert_eq!(
            result.applied_primitive_ids,
            vec!["width-control", "depth-control", "height-control"]
        );
    }

    #[test]
    fn generated_ecky_ast_control_resolves_exact_ui_parameter_without_manifest_primitive() {
        let mut manifest = imported_manifest();
        manifest.source_kind = crate::contracts::ModelSourceKind::Generated;
        manifest.source_language = crate::contracts::SourceLanguage::EckyIrV0;
        manifest.control_primitives.clear();
        manifest.control_relations.clear();

        let result = resolve_semantic_control_value(
            &manifest,
            &numeric_ui_spec(),
            "ast-param:width",
            ParamValue::Number(12.0),
        )
        .expect("AST parameter patch");

        assert_eq!(
            result.parameter_patch,
            DesignParams::from([("width".to_string(), ParamValue::Number(12.0))])
        );
        assert_eq!(result.applied_primitive_ids, vec!["ast-param:width"]);
    }

    #[test]
    fn legacy_derived_primitive_id_is_rebuilt_and_validated_in_rust() {
        let mut manifest = imported_manifest();
        manifest.control_primitives.clear();
        manifest.control_relations.clear();

        let result = resolve_semantic_control_value(
            &manifest,
            &numeric_ui_spec(),
            "primitive-width",
            ParamValue::Number(9.0),
        )
        .expect("derived primitive patch");

        assert_eq!(
            result.parameter_patch,
            DesignParams::from([("width".to_string(), ParamValue::Number(9.0))])
        );
        assert_eq!(result.applied_primitive_ids, vec!["primitive-width"]);
    }

    #[test]
    fn unknown_or_non_editable_control_is_rejected_without_patch() {
        let mut manifest = imported_manifest();
        manifest.control_primitives[0].editable = false;

        let locked = resolve_semantic_control_value(
            &manifest,
            &numeric_ui_spec(),
            "width-control",
            ParamValue::Number(2.0),
        )
        .expect_err("locked control");
        assert!(locked.message.contains("not editable"));

        let unknown = resolve_semantic_control_value(
            &manifest,
            &numeric_ui_spec(),
            "missing-control",
            ParamValue::Number(2.0),
        )
        .expect_err("unknown control");
        assert!(unknown.message.contains("was not found"));
    }

    #[test]
    fn semantic_value_boundary_is_camel_case() {
        let input = ApplySemanticControlValueInput {
            thread_id: "thread-1".to_string(),
            target_message_id: "message-1".to_string(),
            primitive_id: "width-control".to_string(),
            value: ParamValue::Number(4.0),
        };
        let value = serde_json::to_value(input).expect("serialize input");

        assert_eq!(value["threadId"], "thread-1");
        assert_eq!(value["targetMessageId"], "message-1");
        assert_eq!(value["primitiveId"], "width-control");
        assert!(value.get("thread_id").is_none());
    }
}
