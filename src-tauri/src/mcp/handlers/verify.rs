use super::artifact_bundle_digest;
use crate::contracts::{
    AppError, AppErrorCode, AppResult, ArtifactBundle, ModelManifest, RenderSnapshot,
    VerificationRecord,
};
use crate::mcp::contracts::{StructuralVerificationSummaryResponse, VerifyGeneratedModelResponse};
use crate::models::{AppState, PathResolver};
use std::collections::BTreeMap;
use uuid::Uuid;

pub async fn handle_verify_generated_model(
    state: &AppState,
    app: &dyn PathResolver,
    thread_id: &str,
    message_id: &str,
    model_id: &str,
    _original_prompt: &str,
) -> AppResult<VerifyGeneratedModelResponse> {
    let snapshot = load_draft_render_snapshot(state, message_id, model_id).await?;
    let (bundle, manifest) = match &snapshot {
        Some(snapshot) => (
            snapshot.artifact_bundle.clone(),
            snapshot.model_manifest.clone(),
        ),
        None => (
            crate::model_runtime::read_artifact_bundle(app, model_id)?,
            crate::model_runtime::read_model_manifest(app, model_id)?,
        ),
    };
    let artifact_digest = artifact_bundle_digest(&bundle);
    let result = enrich_verify_result_with_diagnostic_context(
        crate::services::author_verification_foundation::verify_structure_with_author_verification(
            &bundle, &manifest,
        ),
        state,
        message_id,
        &bundle,
        &manifest,
    )
    .await?;
    if let Some(snapshot) = snapshot {
        persist_verification_record(state, message_id, &snapshot, &result).await?;
        attach_verification_outcome(state, message_id, &result).await?;
    }
    Ok(VerifyGeneratedModelResponse {
        thread_id: thread_id.to_string(),
        message_id: message_id.to_string(),
        model_id: model_id.to_string(),
        artifact_digest,
        result,
    })
}

async fn attach_verification_outcome(
    state: &AppState,
    preview_id: &str,
    result: &crate::contracts::StructuralVerificationResult,
) -> AppResult<()> {
    let conn = state.db.lock().await;
    let draft = crate::db::get_agent_draft_by_preview_id(&conn, preview_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| AppError::persistence("Verified preview draft disappeared."))?;
    let version_id = draft
        .base_message_id
        .as_deref()
        .ok_or_else(|| AppError::persistence("Verified preview has no durable version."))?;
    let version = crate::db::get_thread_message_version(&conn, &draft.thread_id, version_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| AppError::persistence("Verified durable version disappeared."))?;
    let status = if result.passed {
        crate::contracts::MessageStatus::Success
    } else {
        crate::contracts::MessageStatus::Error
    };
    let content = if result.passed {
        version.content
    } else {
        result.summary.clone()
    };
    crate::db::update_message_status_and_output(
        &conn,
        version_id,
        crate::db::MessageStatusUpdate {
            status: &status,
            output: version.output.as_ref(),
            usage: version.usage.as_ref(),
            artifact_bundle: version.artifact_bundle.as_ref(),
            model_manifest: version.model_manifest.as_ref(),
            structural_verification: Some(result),
            visual_kind: version.visual_kind.as_ref(),
            content: Some(&content),
        },
    )
    .map_err(|error| AppError::persistence(error.to_string()))
}

async fn load_draft_render_snapshot(
    state: &AppState,
    preview_id: &str,
    requested_model_id: &str,
) -> AppResult<Option<RenderSnapshot>> {
    let draft = {
        let conn = state.db.lock().await;
        crate::db::get_agent_draft_by_preview_id(&conn, preview_id)
            .map_err(|error| AppError::persistence(error.to_string()))?
    };
    let Some(draft) = draft else {
        return Ok(None);
    };
    if draft.artifact_bundle.model_id != requested_model_id {
        return Err(AppError::with_details(
            AppErrorCode::Conflict,
            "Verification preview does not match the requested artifact.",
            format!(
                "previewId={} previewModelId={} requestedModelId={requested_model_id}",
                draft.preview_id, draft.artifact_bundle.model_id
            ),
        )
        .with_operation("verify_generated_model"));
    }
    crate::services::render_snapshot::build_render_snapshot(
        crate::services::render_snapshot::RenderSnapshotInput {
            design: &draft.design_output,
            effective_params: &draft.design_output.initial_params,
            artifact_bundle: &draft.artifact_bundle,
            model_manifest: &draft.model_manifest,
        },
    )
    .map(Some)
}

async fn persist_verification_record(
    state: &AppState,
    preview_id: &str,
    snapshot: &RenderSnapshot,
    result: &crate::contracts::StructuralVerificationResult,
) -> AppResult<()> {
    let record = VerificationRecord {
        verification_id: Uuid::new_v4().to_string(),
        snapshot_id: snapshot.snapshot_id.clone(),
        artifact_digest: snapshot.artifact_digest.clone(),
        passed: result.passed,
        verifier_status: result.verifier_status.clone(),
        verifier_source: result.verifier_source.clone(),
    };
    let conn = state.db.lock().await;
    crate::db::upsert_verification_record(&conn, preview_id, &record, super::now_secs())
        .map_err(|error| AppError::persistence(error.to_string()))
}

pub async fn handle_structural_verification_summary(
    state: &AppState,
    app: &dyn PathResolver,
    thread_id: &str,
    message_id: &str,
    model_id: &str,
) -> AppResult<StructuralVerificationSummaryResponse> {
    let bundle = crate::model_runtime::read_artifact_bundle(app, model_id)?;
    let manifest = crate::model_runtime::read_model_manifest(app, model_id)?;
    let artifact_digest = artifact_bundle_digest(&bundle);
    let result = enrich_verify_result_with_diagnostic_context(
        crate::services::author_verification_foundation::verify_structure_with_author_verification(
            &bundle, &manifest,
        ),
        state,
        message_id,
        &bundle,
        &manifest,
    )
    .await?;
    Ok(StructuralVerificationSummaryResponse {
        thread_id: thread_id.to_string(),
        message_id: message_id.to_string(),
        model_id: model_id.to_string(),
        artifact_digest,
        passed: result.passed,
        summary: result.summary,
        issue_count: result.issues.len(),
        verifier_status: result.verifier_status,
        verifier_source: result.verifier_source,
    })
}

fn core_param_value_to_param_value(
    value: &crate::ecky_core_ir::CoreParameterValue,
) -> crate::contracts::ParamValue {
    match value {
        crate::ecky_core_ir::CoreParameterValue::Number(value) => {
            crate::contracts::ParamValue::Number(*value)
        }
        crate::ecky_core_ir::CoreParameterValue::Boolean(value) => {
            crate::contracts::ParamValue::Boolean(*value)
        }
        crate::ecky_core_ir::CoreParameterValue::Text(value)
        | crate::ecky_core_ir::CoreParameterValue::Choice(value)
        | crate::ecky_core_ir::CoreParameterValue::Image(value) => {
            crate::contracts::ParamValue::String(value.clone())
        }
    }
}

async fn resolved_verify_diagnostic_params(
    state: &AppState,
    message_id: &str,
    bundle: &ArtifactBundle,
) -> AppResult<Vec<crate::contracts::DiagnosticParamValue>> {
    let mut resolved = BTreeMap::new();
    let Some(source_path) = bundle
        .macro_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    else {
        return Ok(resolved
            .into_iter()
            .map(|(key, value)| crate::contracts::DiagnosticParamValue { key, value })
            .collect());
    };
    let Ok(source) = std::fs::read_to_string(source_path) else {
        return Ok(resolved
            .into_iter()
            .map(|(key, value)| crate::contracts::DiagnosticParamValue { key, value })
            .collect());
    };
    let Ok(program) = crate::ecky_scheme::compile_to_core_program(&source) else {
        return Ok(resolved
            .into_iter()
            .map(|(key, value)| crate::contracts::DiagnosticParamValue { key, value })
            .collect());
    };

    for param in &program.parameters {
        resolved.insert(
            param.key.clone(),
            core_param_value_to_param_value(&param.default_value),
        );
    }

    let persisted_params = {
        let conn = state.db.lock().await;
        match crate::db::get_agent_draft_by_preview_id(&conn, message_id)
            .map_err(|error| crate::contracts::AppError::persistence(error.to_string()))?
        {
            Some(draft) if draft.artifact_bundle.model_id == bundle.model_id => {
                Some(draft.design_output.initial_params)
            }
            Some(draft) => {
                return Err(crate::contracts::AppError::with_details(
                    crate::contracts::AppErrorCode::Conflict,
                    "Verification preview does not match the requested artifact.",
                    format!(
                        "previewId={} previewModelId={} requestedModelId={}",
                        draft.preview_id, draft.artifact_bundle.model_id, bundle.model_id
                    ),
                )
                .with_operation("verify_generated_model"));
            }
            None => crate::db::get_message_output_and_thread(&conn, message_id)
                .map_err(|error| crate::contracts::AppError::persistence(error.to_string()))?
                .map(|(output, _thread_id)| output.initial_params),
        }
    };
    if let Some(params) = persisted_params {
        for (key, value) in params {
            resolved.insert(key, value);
        }
    }

    Ok(resolved
        .into_iter()
        .map(|(key, value)| crate::contracts::DiagnosticParamValue { key, value })
        .collect())
}

fn verify_check_op_name(check: &crate::contracts::AuthoredVerifyCheck) -> Option<String> {
    match (check.metric_source.as_deref(), check.metric_key.as_deref()) {
        (Some(source), Some(key)) => Some(format!("verify:{source}/{key}")),
        (Some(source), None) => Some(format!("verify:{source}")),
        (None, Some(key)) => Some(format!("verify:{key}")),
        (None, None) => Some(format!("verify:{}", check.tag)),
    }
}

async fn enrich_verify_result_with_diagnostic_context(
    mut result: crate::contracts::StructuralVerificationResult,
    state: &AppState,
    message_id: &str,
    bundle: &ArtifactBundle,
    manifest: &ModelManifest,
) -> AppResult<crate::contracts::StructuralVerificationResult> {
    let part_key = (manifest.parts.len() == 1).then(|| manifest.parts[0].part_id.clone());
    let resolved_params = resolved_verify_diagnostic_params(state, message_id, bundle).await?;

    let mut failing_contexts = Vec::new();
    for check in &mut result.authored_verify_checks {
        if check.status == crate::contracts::AuthoredVerifyCheckStatus::Passed {
            continue;
        }
        let context = crate::contracts::DiagnosticContext {
            part_key: part_key.clone(),
            op_name: verify_check_op_name(check),
            start_line: None,
            end_line: None,
            resolved_params: resolved_params.clone(),
        };
        check.diagnostic_context = Some(context.clone());
        failing_contexts.push(context);
    }

    let mut failing_index = 0usize;
    for issue in &mut result.issues {
        if !matches!(
            issue.code.as_str(),
            "AUTHORED_VERIFY_FAILED" | "AUTHORED_VERIFY_ERROR"
        ) {
            continue;
        }
        let Some(context) = failing_contexts.get(failing_index).cloned() else {
            break;
        };
        if issue.part_id.is_none() {
            issue.part_id = context.part_key.clone();
        }
        issue.diagnostic_context = Some(context);
        failing_index += 1;
    }

    Ok(result)
}
