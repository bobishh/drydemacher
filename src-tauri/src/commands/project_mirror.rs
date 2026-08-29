//! Tauri commands surfacing the filesystem project mirror to the app shell
//! (filesystem-project-mirror T5.2). These are thin wrappers over the
//! existing `project_mirror` core and the MCP `project_folder_*` handlers:
//! they resolve the active thread's slug, then delegate so that all version
//! writes still flow through the shared preview -> verify -> commit pipeline
//! (no direct database writes; see the "Mirror Stays Out of the Database"
//! requirement).

use tauri::{AppHandle, State};

use crate::contracts::AppResult;
use crate::mcp::contracts::AgentIdentityOverride;
use crate::mcp::handlers::{
    handle_project_folder_apply, handle_project_folder_export, handle_project_folder_status,
    AgentContext, ProjectFolderApplyRequest, ProjectFolderExportRequest,
    ProjectFolderStatusRequest,
};
use crate::models::AppState;
use crate::project_mirror::{
    self, ProjectFolderRenderActivity, ProjectFolderStatus, ProjectManifest, ProjectSyncState,
};
use crate::services::target::resolve_editable_target;

/// Context label for user-driven project-folder actions (as opposed to agent
/// or watcher driven ones). Provenance only; does not change behavior.
fn ui_agent_context() -> AgentContext {
    AgentContext {
        session_id: "tauri-ui".to_string(),
        client_kind: "ui".to_string(),
        host_label: "Ecky".to_string(),
        agent_label: "project-folder".to_string(),
        llm_model_id: None,
        llm_model_label: None,
    }
}

/// Resolves the active editable target and its deterministic project slug.
/// Mirrors the slug rule used by `open_project_in_editor` so the UI, the
/// system-editor action, and the mirror all address one folder per thread.
async fn resolve_target_and_slug(
    state: &AppState,
    app: &AppHandle,
    thread_id: Option<String>,
    message_id: Option<String>,
) -> AppResult<(crate::services::target::EditableTarget, String)> {
    let (target, binding) = {
        let conn = state.db.lock().await;
        let target = resolve_editable_target(&conn, app, thread_id, message_id)?;
        let binding = crate::thread_source_binding::get_binding(&conn, &target.thread_id)
            .map_err(|err| crate::contracts::AppError::persistence(err.to_string()))?;
        (target, binding)
    };
    let slug = match binding {
        Some(binding) => std::path::Path::new(&binding.folder_path)
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                crate::contracts::AppError::persistence(format!(
                    "Bound source folder '{}' has no valid folder name.",
                    binding.folder_path
                ))
            })?
            .to_string(),
        None => project_mirror::project_slug(&target.design_output.title, &target.thread_id),
    };
    Ok((target, slug))
}

/// Export the active version's macro source to its project folder, writing
/// `model.ecky` and refreshing the `ecky-project.edn` manifest. Re-export
/// preserves the existing project id.
#[tauri::command]
#[specta::specta]
pub async fn project_folder_export(
    thread_id: Option<String>,
    message_id: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<ProjectFolderExportResult> {
    let ctx = ui_agent_context();
    let response = handle_project_folder_export(
        state.inner(),
        &app,
        ProjectFolderExportRequest {
            identity: AgentIdentityOverride::default(),
            thread_id,
            message_id,
            slug: None,
        },
        &ctx,
    )
    .await?;
    Ok(ProjectFolderExportResult {
        slug: response.slug,
        folder: response.folder,
        manifest: response.manifest,
    })
}

#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFolderExportResult {
    pub slug: String,
    pub folder: String,
    pub manifest: ProjectManifest,
}

/// Read-only sync classification (`clean` / `fileChanged` / `threadAdvanced`
/// / `conflict` / `missing`) for the active thread's folder. Does not mutate
/// the folder, the thread, or any history.
#[tauri::command]
#[specta::specta]
pub async fn project_folder_status(
    thread_id: Option<String>,
    message_id: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<ProjectFolderStatus> {
    let (_target, slug) = resolve_target_and_slug(&state, &app, thread_id, message_id).await?;
    handle_project_folder_status(state.inner(), &app, ProjectFolderStatusRequest { slug }).await
}

/// Snapshot active watcher-owned renders. Independent from the database and
/// render lock, so startup can show truthful progress while rendering blocks.
#[tauri::command]
#[specta::specta]
pub async fn project_folder_render_activity(
    state: State<'_, AppState>,
) -> AppResult<Vec<ProjectFolderRenderActivity>> {
    Ok(state
        .project_folder_render_activity
        .lock()
        .await
        .values()
        .cloned()
        .collect())
}

/// Apply the folder's source file onto the active thread head.
/// `force=true` allows apply when both file and thread diverged.
#[tauri::command]
#[specta::specta]
pub async fn project_folder_apply(
    thread_id: Option<String>,
    message_id: Option<String>,
    force: Option<bool>,
    title: Option<String>,
    version_name: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<ProjectFolderApplyResult> {
    let (_target, slug) = resolve_target_and_slug(&state, &app, thread_id, message_id).await?;
    let response = handle_project_folder_apply(
        state.inner(),
        &app,
        ProjectFolderApplyRequest {
            identity: AgentIdentityOverride::default(),
            slug,
            force: force.unwrap_or(false),
            title,
            version_name,
        },
        &ui_agent_context(),
    )
    .await?;
    Ok(ProjectFolderApplyResult {
        state_before: response.state_before,
        no_op: response.no_op,
        thread_id: response.thread_id,
        message_id: response.message_id,
        manifest: response.manifest,
    })
}

#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFolderApplyResult {
    pub state_before: ProjectSyncState,
    pub no_op: bool,
    pub thread_id: String,
    pub message_id: String,
    pub manifest: ProjectManifest,
}
