use crate::contracts::{
    AppError, AppResult, CaptureSurfaceAnchor, DesignOutput, GeometryBackend, Message,
    SourceLanguage,
};
use crate::external_shapes::{
    ApplyExternalShapePlaneCropRequest, ExternalShapeSource, RemoveExternalShapePlaneCropRequest,
};
use crate::models::{AppState, PathResolver};
use crate::services::manual_code::{ManualCodeApplyRequest, ManualCodeApplyResponse};
use crate::surface_trim_cap::SurfaceTrimCapMode;
use crate::surface_trim_external_shapes::SurfaceTrimPathMode;
use crate::surface_trim_source::{ApplySurfaceTrimRequest, RemoveSurfaceTrimRequest};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ExternalShapeEditIntent {
    #[specta(rename_all = "camelCase")]
    ApplyPlaneCrop {
        node_id: u64,
        expected_mesh_content_digest: String,
        anchors: Vec<CaptureSurfaceAnchor>,
        keep_positive: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        replace_crop_node_id: Option<u64>,
    },
    #[specta(rename_all = "camelCase")]
    RemovePlaneCrop { node_id: u64, crop_node_id: u64 },
    #[specta(rename_all = "camelCase")]
    ApplySurfaceTrim {
        schema_version: u32,
        node_id: u64,
        expected_mesh_content_digest: String,
        loop_anchors: Vec<CaptureSurfaceAnchor>,
        keep_seed: CaptureSurfaceAnchor,
        path_mode: SurfaceTrimPathMode,
        cap_mode: SurfaceTrimCapMode,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        replace_trim_node_id: Option<u64>,
    },
    #[specta(rename_all = "camelCase")]
    RemoveSurfaceTrim { node_id: u64, trim_node_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyExternalShapeEditInput {
    pub thread_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_message_id: Option<String>,
    pub expected_source_digest: String,
    pub edit: ExternalShapeEditIntent,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApplyExternalShapeEditResult {
    pub version: ManualCodeApplyResponse,
    pub source_digest: String,
    pub external_sources: Vec<ExternalShapeSource>,
}

struct ExternalShapeEditContext {
    source: String,
    source_path: PathBuf,
    source_folder: PathBuf,
    base_message: Message,
    base_design: DesignOutput,
}

pub async fn apply_external_shape_edit(
    input: ApplyExternalShapeEditInput,
    state: &AppState,
    app: &dyn PathResolver,
) -> AppResult<ApplyExternalShapeEditResult> {
    let thread_id = require_id(&input.thread_id, "External shape thread id")?;
    let context = load_context(&thread_id, input.base_message_id.as_deref(), state).await?;
    let next_source = patch_source(
        &thread_id,
        &context.source,
        &context.source_folder,
        &input.expected_source_digest,
        input.edit,
    )?;
    let request = manual_code_request(&thread_id, &context, next_source);
    let version = crate::services::manual_code::apply_manual_code(request, state, app).await?;

    let canonical_source = std::fs::read_to_string(&context.source_path).map_err(|error| {
        AppError::persistence(format!(
            "Failed to reread bound external shape source '{}': {error}",
            context.source_path.display()
        ))
    })?;
    let source_digest =
        crate::services::render_snapshot::canonical_source_digest(&canonical_source);
    let external_sources = crate::external_shapes::discover_bound_external_shapes(
        &canonical_source,
        &context.source_folder,
    )?;

    Ok(ApplyExternalShapeEditResult {
        version,
        source_digest,
        external_sources,
    })
}

async fn load_context(
    thread_id: &str,
    base_message_id: Option<&str>,
    state: &AppState,
) -> AppResult<ExternalShapeEditContext> {
    let conn = state.db.lock().await;
    let binding = crate::thread_source_binding::get_binding(&conn, thread_id)
        .map_err(|error| AppError::persistence(error.to_string()))?
        .ok_or_else(|| {
            AppError::not_found(format!(
                "Design thread '{}' has no bound model.ecky source.",
                thread_id
            ))
        })?;
    let base_message = match base_message_id {
        Some(message_id) => crate::db::get_thread_message_version(&conn, thread_id, message_id)
            .map_err(|error| AppError::persistence(error.to_string()))?
            .ok_or_else(|| {
                AppError::conflict(format!(
                    "External shape base message '{}' is missing or does not belong to thread '{}'.",
                    message_id, thread_id
                ))
            })?,
        None => crate::db::get_thread_latest_version(&conn, thread_id)
            .map_err(|error| AppError::persistence(error.to_string()))?
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "Design thread '{}' has no version for external shape edit.",
                    thread_id
                ))
            })?,
    };
    let base_design = base_message.output.clone().ok_or_else(|| {
        AppError::validation(format!(
            "External shape base message '{}' has no design output.",
            base_message.id
        ))
    })?;
    let source_path = PathBuf::from(binding.source_path);
    let source = std::fs::read_to_string(&source_path).map_err(|error| {
        AppError::persistence(format!(
            "Failed to read bound external shape source '{}': {error}",
            source_path.display()
        ))
    })?;
    Ok(ExternalShapeEditContext {
        source,
        source_path,
        source_folder: PathBuf::from(binding.folder_path),
        base_message,
        base_design,
    })
}

fn patch_source(
    thread_id: &str,
    source: &str,
    source_folder: &Path,
    expected_source_digest: &str,
    edit: ExternalShapeEditIntent,
) -> AppResult<String> {
    match edit {
        ExternalShapeEditIntent::ApplyPlaneCrop {
            node_id,
            expected_mesh_content_digest,
            anchors,
            keep_positive,
            replace_crop_node_id,
        } => Ok(crate::external_shapes::apply_plane_crop_to_source(
            source,
            source_folder,
            &ApplyExternalShapePlaneCropRequest {
                thread_id: thread_id.to_string(),
                node_id,
                expected_source_digest: expected_source_digest.to_string(),
                expected_mesh_content_digest,
                anchors,
                keep_positive,
                replace_crop_node_id,
            },
        )?
        .source),
        ExternalShapeEditIntent::RemovePlaneCrop {
            node_id,
            crop_node_id,
        } => Ok(crate::external_shapes::remove_plane_crop_from_source(
            source,
            source_folder,
            &RemoveExternalShapePlaneCropRequest {
                thread_id: thread_id.to_string(),
                node_id,
                crop_node_id,
                expected_source_digest: expected_source_digest.to_string(),
            },
        )?
        .source),
        ExternalShapeEditIntent::ApplySurfaceTrim {
            schema_version,
            node_id,
            expected_mesh_content_digest,
            loop_anchors,
            keep_seed,
            path_mode,
            cap_mode,
            replace_trim_node_id,
        } => Ok(crate::surface_trim_source::apply_surface_trim_to_source(
            source,
            source_folder,
            &ApplySurfaceTrimRequest {
                schema_version,
                thread_id: thread_id.to_string(),
                target_message_id: None,
                node_id,
                expected_source_digest: expected_source_digest.to_string(),
                expected_mesh_content_digest,
                loop_anchors,
                keep_seed,
                path_mode,
                cap_mode,
                replace_trim_node_id,
            },
        )?
        .source),
        ExternalShapeEditIntent::RemoveSurfaceTrim {
            node_id,
            trim_node_id,
        } => Ok(crate::surface_trim_source::remove_surface_trim_from_source(
            source,
            &RemoveSurfaceTrimRequest {
                thread_id: thread_id.to_string(),
                target_message_id: None,
                node_id,
                trim_node_id,
                expected_source_digest: expected_source_digest.to_string(),
            },
        )?
        .source),
    }
}

fn manual_code_request(
    thread_id: &str,
    context: &ExternalShapeEditContext,
    source: String,
) -> ManualCodeApplyRequest {
    ManualCodeApplyRequest {
        thread_id: thread_id.to_string(),
        base_message_id: Some(context.base_message.id.clone()),
        source,
        persist: true,
        title: Some(context.base_design.title.clone()),
        version_name: Some("External shape edit".to_string()),
        ui_spec: context.base_design.ui_spec.clone(),
        parameters: context.base_design.initial_params.clone(),
        post_processing: context.base_design.post_processing.clone(),
        source_language: Some(SourceLanguage::EckyIrV0),
        geometry_backend: Some(GeometryBackend::EckyRust),
    }
}

fn require_id(value: &str, label: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::validation(format!("{} cannot be empty.", label)));
    }
    Ok(value.to_string())
}
