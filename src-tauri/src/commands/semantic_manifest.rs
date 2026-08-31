use tauri::{AppHandle, State};

use crate::contracts::{AppResult, ApplySemanticManifestEditInput, SemanticManifestEditResult};
use crate::models::AppState;

#[tauri::command]
#[specta::specta]
pub async fn apply_semantic_control_value(
    input: crate::services::semantic_control_value::ApplySemanticControlValueInput,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<crate::services::semantic_control_value::ApplySemanticControlValueResult> {
    crate::services::semantic_control_value::apply_semantic_control_value(input, &state, &app).await
}

#[tauri::command]
#[specta::specta]
pub async fn apply_semantic_manifest_edit(
    input: ApplySemanticManifestEditInput,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<SemanticManifestEditResult> {
    let manifest = crate::commands::render::load_control_view_target_manifest(
        &input.model_id,
        input.message_id.as_deref(),
        &state,
        &app,
    )
    .await?;
    let result = crate::services::semantic_manifest::apply_semantic_manifest_edit(
        &manifest,
        input.edit,
        crate::services::semantic_manifest::SemanticEditActor::Manual,
    )?;
    let manifest = crate::commands::render::persist_model_manifest(
        &input.model_id,
        result.manifest,
        input.message_id,
        &state,
        &app,
    )
    .await?;
    Ok(SemanticManifestEditResult {
        manifest,
        edited_id: result.edited_id,
        selected_view_id: result.selected_view_id,
    })
}
