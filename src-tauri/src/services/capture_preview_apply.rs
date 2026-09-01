use serde::{Deserialize, Serialize};
use specta::Type;

use crate::contracts::{
    AppError, AppResult, DesignParams, GeometryBackend, MessageStatus, SourceLanguage, UiSpec,
};
use crate::models::{AppState, PathResolver};
use crate::services::manual_code::{ManualCodeApplyRequest, ManualCodeApplyResponse};

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyCapturePreviewInput {
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApplyCapturePreviewResult {
    pub source: String,
    pub draft: ManualCodeApplyResponse,
}

pub async fn apply_capture_preview(
    input: ApplyCapturePreviewInput,
    state: &AppState,
    app: &dyn PathResolver,
) -> AppResult<ApplyCapturePreviewResult> {
    let (run, head) = {
        let db = state.db.lock().await;
        let run = crate::capture_runs::get(&db, &input.run_id)
            .map_err(|error| AppError::persistence(error.to_string()))?
            .ok_or_else(|| AppError::not_found("Capture run not found."))?;
        let head = crate::services::history::get_thread_latest_version(&db, &run.target_thread_id)?;
        (run, head)
    };
    let current_output = head.as_ref().and_then(|message| message.output.as_ref());
    let current_source = current_output
        .map(|output| output.macro_code.clone())
        .unwrap_or_default();
    let source_language = current_output
        .map(|output| output.source_language)
        .unwrap_or(SourceLanguage::EckyIrV0);
    validate_capture_target(&run.target_source, &current_source, source_language)?;
    let stl_path = run
        .derived_stl_path
        .clone()
        .or_else(|| {
            run.mesh_preview
                .as_ref()
                .map(|preview| preview.stl_path.clone())
        })
        .ok_or_else(|| AppError::not_found("Capture preview mesh is missing."))?;
    if !std::path::Path::new(&stl_path).is_file() {
        return Err(AppError::not_found(format!(
            "Capture preview STL is missing: {stl_path}"
        )));
    }
    let source =
        build_capture_solidify_source(&current_source, &stl_path, &run.id, run.preview_scale)?;
    let request = ManualCodeApplyRequest {
        thread_id: run.target_thread_id,
        base_message_id: head.as_ref().map(|message| message.id.clone()),
        source: source.clone(),
        persist: false,
        title: Some(run.title),
        version_name: Some("Capture Draft".into()),
        ui_spec: current_output
            .map(|output| output.ui_spec.clone())
            .unwrap_or_else(|| UiSpec { fields: Vec::new() }),
        parameters: current_output
            .map(|output| output.initial_params.clone())
            .unwrap_or_else(DesignParams::new),
        post_processing: current_output.and_then(|output| output.post_processing.clone()),
        source_language: Some(SourceLanguage::EckyIrV0),
        geometry_backend: Some(
            current_output
                .map(|output| output.geometry_backend)
                .unwrap_or(GeometryBackend::EckyRust),
        ),
    };
    let draft = crate::services::manual_code::apply_manual_code(request, state, app).await?;
    if draft.status == MessageStatus::Error {
        return Err(draft.error.clone().unwrap_or_else(|| {
            AppError::validation("Capture preview returned no renderable runtime.")
        }));
    }
    if draft.artifact_bundle.is_none()
        || draft.model_manifest.is_none()
        || draft.snapshot_id.is_none()
    {
        return Err(AppError::validation(
            "Capture preview returned no renderable runtime.",
        ));
    }
    Ok(ApplyCapturePreviewResult { source, draft })
}

fn validate_capture_target(
    expected_source: &str,
    current_source: &str,
    source_language: SourceLanguage,
) -> AppResult<()> {
    if current_source != expected_source {
        return Err(AppError::validation(format!(
            "Capture target source diverged: expected {}, found {}.",
            crate::services::render_snapshot::canonical_source_digest(expected_source),
            crate::services::render_snapshot::canonical_source_digest(current_source),
        )));
    }
    if !current_source.trim().is_empty() && source_language != SourceLanguage::EckyIrV0 {
        return Err(AppError::validation(format!(
            "Capture target source language {} cannot accept Ecky AST insertion.",
            source_language.as_str(),
        )));
    }
    Ok(())
}

fn build_capture_solidify_source(
    source: &str,
    stl_path: &str,
    capture_id: &str,
    scale: f64,
) -> AppResult<String> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(AppError::validation(
            "Capture scale must be greater than zero.",
        ));
    }
    let suffix = capture_id
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    let part_id = format!(
        "capture_{}",
        if suffix.is_empty() { "scan" } else { &suffix }
    );
    let scale_id = part_id.replacen("capture_", "capture_scale_", 1);
    let scale_literal = format!("{scale:.8}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string();
    let parameter = format!(
        "(params (number {scale_id} {scale_literal} :label \"Capture scale\" :min 0.001 :max 2 :step 0.001))"
    );
    let quoted_path =
        serde_json::to_string(stl_path).map_err(|error| AppError::validation(error.to_string()))?;
    let part = format!(
        "(part {part_id} (scale {scale_id} {scale_id} {scale_id} (solidify (import-stl {quoted_path}))))"
    );
    if source.trim().is_empty() {
        return Ok(format!("(model\n  {parameter}\n  {part})"));
    }
    let model = crate::commands::macro_ast::macro_ast_source_map_impl(source)?
        .into_iter()
        .find(|node| node.id == "model" && node.kind == "model")
        .ok_or_else(|| AppError::parse("Capture target has no parser-derived model AST range."))?;
    let end = model.end_byte as usize;
    let insert_at = end.checked_sub(1).ok_or_else(|| {
        AppError::parse("Capture model AST range does not end at model closing parenthesis.")
    })?;
    if source.as_bytes().get(insert_at) != Some(&b')') {
        return Err(AppError::parse(
            "Capture model AST range does not end at model closing parenthesis.",
        ));
    }
    Ok(format!(
        "{}\n  {parameter}\n  {part}{}",
        &source[..insert_at],
        &source[insert_at..]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_capture_preview_input_accepts_only_camel_case_boundary() {
        let parsed: ApplyCapturePreviewInput =
            serde_json::from_value(serde_json::json!({ "runId": "run-1" })).unwrap();
        assert_eq!(parsed.run_id, "run-1");
        assert!(serde_json::from_value::<ApplyCapturePreviewInput>(
            serde_json::json!({ "run_id": "run-1" })
        )
        .is_err());
    }

    #[test]
    fn empty_capture_source_builds_named_scaled_solid() {
        let source = build_capture_solidify_source("", "/tmp/scan.stl", "abc-123", 0.05).unwrap();
        assert!(source.contains("capture_scale_abc_123 0.05"));
        assert!(source.contains("(part capture_abc_123"));
        assert!(source.contains("(solidify (import-stl \"/tmp/scan.stl\"))"));
    }

    #[test]
    fn existing_capture_source_inserts_inside_parser_model_range() {
        let source = build_capture_solidify_source(
            "(model (part base (box 10 20 30)))",
            "/tmp/scan.stl",
            "run",
            1.0,
        )
        .unwrap();
        assert!(source.starts_with("(model (part base"));
        assert!(source.contains("(part capture_run"));
        assert!(source.ends_with(')'));
    }

    #[test]
    fn stale_capture_source_returns_exact_digest_evidence() {
        let error =
            validate_capture_target("(model)", "(model (part newer))", SourceLanguage::EckyIrV0)
                .expect_err("stale source must fail");
        assert!(error
            .message
            .contains("Capture target source diverged: expected sha256:"));
        assert!(error.message.contains(", found sha256:"));
    }

    #[test]
    fn non_ecky_capture_target_rejects_ast_insertion() {
        let error = validate_capture_target("box = 1", "box = 1", SourceLanguage::Build123d)
            .expect_err("foreign source must fail");
        assert_eq!(
            error.message,
            "Capture target source language build123d cannot accept Ecky AST insertion."
        );
    }
}
