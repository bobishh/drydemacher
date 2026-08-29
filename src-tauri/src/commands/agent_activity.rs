use tauri::State;

use crate::contracts::{AgentActivityCatchUp, AppResult};
use crate::models::AppState;

#[tauri::command]
#[specta::specta]
pub async fn get_agent_activity(
    after_cursor: Option<u64>,
    state: State<'_, AppState>,
) -> AppResult<AgentActivityCatchUp> {
    Ok(state.get_agent_activity(after_cursor))
}
