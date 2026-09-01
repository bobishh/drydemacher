use tauri::{AppHandle, State};

use crate::contracts::AppResult;
use crate::models::AppState;
use crate::services::external_shape_edit::{
    ApplyExternalShapeEditInput, ApplyExternalShapeEditResult,
};

#[tauri::command]
#[specta::specta]
pub async fn apply_external_shape_edit(
    input: ApplyExternalShapeEditInput,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<ApplyExternalShapeEditResult> {
    crate::services::external_shape_edit::apply_external_shape_edit(input, &state, &app).await
}
