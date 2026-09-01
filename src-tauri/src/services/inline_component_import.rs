use crate::component_import_runtime::{
    CopyInlineComponentImportRequest, InstalledLibraryComponentResolver,
};
use crate::contracts::{AppError, AppResult, DesignOutput, Message};
use crate::models::{AppState, PathResolver};
use crate::services::manual_code::{ManualCodeApplyRequest, ManualCodeApplyResponse};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyInlineComponentImportInput {
    pub thread_id: String,
    pub base_message_id: String,
    pub expected_source_digest: String,
    pub package_id: String,
    pub version: String,
    pub component_id: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApplyInlineComponentImportResult {
    pub version: ManualCodeApplyResponse,
    pub source_digest: String,
    pub entry_symbol: String,
    pub part_key: String,
}

struct InlineComponentImportContext {
    source: String,
    source_path: PathBuf,
    base_message: Message,
    base_design: DesignOutput,
}

pub async fn apply_inline_component_import(
    input: ApplyInlineComponentImportInput,
    state: &AppState,
    app: &dyn PathResolver,
) -> AppResult<ApplyInlineComponentImportResult> {
    let thread_id = require_id(&input.thread_id, "Component import thread id")?;
    let base_message_id = require_id(&input.base_message_id, "Component import base message id")?;
    let expected_source_digest = require_id(
        &input.expected_source_digest,
        "Component import expected source digest",
    )?;
    let package_id = require_id(&input.package_id, "Component import package id")?;
    let version = require_id(&input.version, "Component import package version")?;
    let component_id = require_id(&input.component_id, "Component import component id")?;
    let context = load_context(&thread_id, &base_message_id, state).await?;
    let actual_source_digest =
        crate::services::render_snapshot::canonical_source_digest(&context.source);
    if actual_source_digest != expected_source_digest {
        return Err(AppError::conflict(format!(
            "Bound source changed before component import: expected digest '{}', actual digest '{}'.",
            expected_source_digest, actual_source_digest
        )));
    }

    let imported = crate::component_import_runtime::copy_inline_component_import(
        CopyInlineComponentImportRequest {
            package_id,
            version,
            component_id,
            authored_source: context.source.clone(),
        },
        &InstalledLibraryComponentResolver { app },
    )?;
    let request = manual_code_request(&thread_id, &context, imported.authored_source);
    let version = crate::services::manual_code::apply_manual_code(request, state, app).await?;
    let canonical_source = std::fs::read_to_string(&context.source_path).map_err(|error| {
        AppError::persistence(format!(
            "Failed to reread bound component import source '{}': {error}",
            context.source_path.display()
        ))
    })?;

    Ok(ApplyInlineComponentImportResult {
        version,
        source_digest: crate::services::render_snapshot::canonical_source_digest(&canonical_source),
        entry_symbol: imported.entry_symbol,
        part_key: imported.part_key,
    })
}

async fn load_context(
    thread_id: &str,
    base_message_id: &str,
    state: &AppState,
) -> AppResult<InlineComponentImportContext> {
    let conn = state.db.lock().await;
    let base_message = crate::db::get_thread_message_version(&conn, thread_id, base_message_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| {
            AppError::conflict(format!(
                "Component import base message '{}' is missing or does not belong to thread '{}'.",
                base_message_id, thread_id
            ))
        })?;
    let base_design = base_message.output.clone().ok_or_else(|| {
        AppError::validation(format!(
            "Component import base message '{}' has no design output.",
            base_message_id
        ))
    })?;
    let binding = crate::thread_source_binding::get_binding(&conn, thread_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| {
            AppError::not_found(format!(
                "Design thread '{}' has no bound model.ecky source.",
                thread_id
            ))
        })?;
    let source_path = PathBuf::from(binding.source_path);
    let source = std::fs::read_to_string(&source_path).map_err(|error| {
        AppError::persistence(format!(
            "Failed to read bound component import source '{}': {error}",
            source_path.display()
        ))
    })?;

    Ok(InlineComponentImportContext {
        source,
        source_path,
        base_message,
        base_design,
    })
}

fn manual_code_request(
    thread_id: &str,
    context: &InlineComponentImportContext,
    source: String,
) -> ManualCodeApplyRequest {
    ManualCodeApplyRequest {
        thread_id: thread_id.to_string(),
        base_message_id: Some(context.base_message.id.clone()),
        source,
        persist: true,
        title: Some(context.base_design.title.clone()),
        version_name: Some("Inline component import".to_string()),
        ui_spec: context.base_design.ui_spec.clone(),
        parameters: context.base_design.initial_params.clone(),
        post_processing: context.base_design.post_processing.clone(),
        source_language: Some(context.base_design.source_language),
        geometry_backend: Some(context.base_design.geometry_backend),
    }
}

fn require_id(value: &str, label: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::validation(format!("{} cannot be empty.", label)));
    }
    Ok(value.to_string())
}
