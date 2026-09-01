use tauri::{AppHandle, State};

use crate::contracts::AppResult;
use crate::models::AppState;
use crate::services::inline_component_import::{
    ApplyInlineComponentImportInput, ApplyInlineComponentImportResult,
};

#[tauri::command]
#[specta::specta]
pub async fn apply_inline_component_import(
    input: ApplyInlineComponentImportInput,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<ApplyInlineComponentImportResult> {
    crate::services::inline_component_import::apply_inline_component_import(input, &state, &app)
        .await
}
