use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use crate::db;
use crate::models::{
    AgentDraft, AgentSession, AppError, AppResult, AppState, LastDesignSnapshot, McpServerStatus,
};

#[tauri::command]
#[specta::specta]
pub async fn get_active_agent_sessions(state: State<'_, AppState>) -> AppResult<Vec<AgentSession>> {
    let conn = state.db.lock().await;
    db::get_active_agent_sessions(&conn, 30)
        .map_err(|e| crate::models::AppError::persistence(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn get_mcp_server_status(state: State<'_, AppState>) -> AppResult<McpServerStatus> {
    Ok(state.mcp_status())
}

#[tauri::command]
#[specta::specta]
pub async fn get_agent_draft(
    state: State<'_, AppState>,
    thread_id: String,
    base_message_id: String,
) -> AppResult<Option<AgentDraft>> {
    let conn = state.db.lock().await;
    db::get_agent_draft(&conn, &thread_id, &base_message_id)
        .map_err(|e| crate::models::AppError::persistence(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn delete_agent_draft(
    state: State<'_, AppState>,
    thread_id: String,
    base_message_id: String,
) -> AppResult<()> {
    let conn = state.db.lock().await;
    db::delete_agent_draft(&conn, &thread_id, &base_message_id)
        .map_err(|e| crate::models::AppError::persistence(e.to_string()))
}

fn last_snapshot_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .unwrap()
        .join("last_design.json")
}

pub(crate) fn write_last_snapshot(app: &AppHandle, snapshot: Option<&LastDesignSnapshot>) {
    let path = last_snapshot_path(app);
    match snapshot {
        Some(snapshot) => {
            if let Ok(serialized) = serde_json::to_string_pretty(snapshot) {
                let _ = fs::write(path, serialized);
            }
        }
        None => {
            let _ = fs::remove_file(path);
        }
    }
}

pub(crate) fn build_runtime_snapshot(
    design: Option<crate::models::DesignOutput>,
    thread_id: Option<String>,
    message_id: Option<String>,
    artifact_bundle: Option<crate::models::ArtifactBundle>,
    model_manifest: Option<crate::models::ModelManifest>,
    selected_part_id: Option<String>,
) -> LastDesignSnapshot {
    LastDesignSnapshot {
        design,
        thread_id,
        message_id,
        artifact_bundle,
        model_manifest,
        selected_part_id,
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_last_design(state: State<'_, AppState>) -> AppResult<Option<LastDesignSnapshot>> {
    Ok(state.last_snapshot.lock().unwrap().clone())
}

#[tauri::command]
#[specta::specta]
pub async fn save_last_design(
    snapshot: Option<LastDesignSnapshot>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<()> {
    {
        let mut last = state.last_snapshot.lock().unwrap();
        *last = snapshot.clone();
    }
    write_last_snapshot(&app, snapshot.as_ref());
    Ok(())
}

/// Called by the frontend when the user clicks a confirmation button.
/// Resolves the pending oneshot channel so the MCP handler can return.
#[tauri::command]
#[specta::specta]
pub async fn resolve_agent_confirm(
    request_id: String,
    choice: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let mut channels = state.confirm_channels.lock().await;
    if let Some(tx) = channels.remove(&request_id) {
        let _ = tx.send(choice);
    } else {
        return Err(AppError::not_found(format!(
            "No pending confirmation with id: {}",
            request_id
        )));
    }
    Ok(())
}
