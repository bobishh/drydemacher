use crate::contracts::{
    AppError, AppResult, ArtifactBundle, DenseTopologyKind, DenseTopologyPage, Message,
    SourceWindow, Thread, ThreadMessagesPage, VersionDetail, VersionPreviewRuntime,
};
use crate::db;
use crate::models::AppState;
use crate::services::history as history_service;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::{AppHandle, State};

#[tauri::command]
#[specta::specta]
pub async fn get_history(state: State<'_, AppState>) -> AppResult<Vec<Thread>> {
    let started = Instant::now();
    let conn = if let Some(read_conn) = state.db_read.as_ref() {
        read_conn.lock().await
    } else {
        state.db.lock().await
    };
    let acquired = Instant::now();
    let result = history_service::get_history(&conn);
    let finished = Instant::now();
    if let Ok(value) = &result {
        crate::transport_budget::observe_projection(
            "get_history",
            "thread_summary",
            value,
            value.len(),
            0,
            0,
            acquired.duration_since(started),
            finished.duration_since(acquired),
        );
    }
    result
}

#[tauri::command]
#[specta::specta]
pub async fn get_thread(state: State<'_, AppState>, id: String) -> AppResult<Thread> {
    // Compatibility name, bounded response. Production IPC must never serialize
    // a full thread aggregate; timeline and selected detail have separate reads.
    if let Some(read_conn) = state.db_read.as_ref() {
        let conn = read_conn.lock().await;
        history_service::get_thread_summary(&conn, &id)
    } else {
        let conn = state.db.lock().await;
        history_service::get_thread_summary(&conn, &id)
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_thread_latest_version(
    state: State<'_, AppState>,
    thread_id: String,
) -> AppResult<Option<Message>> {
    if let Some(read_conn) = state.db_read.as_ref() {
        let conn = read_conn.lock().await;
        history_service::get_thread_latest_version(&conn, &thread_id)
    } else {
        let conn = state.db.lock().await;
        history_service::get_thread_latest_version(&conn, &thread_id)
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_thread_head_version_id(
    state: State<'_, AppState>,
    thread_id: String,
) -> AppResult<Option<String>> {
    let conn = if let Some(read_conn) = state.db_read.as_ref() {
        read_conn.lock().await
    } else {
        state.db.lock().await
    };
    db::get_thread_head_version_id(&conn, &thread_id)
        .map_err(|error| AppError::persistence(error.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn get_thread_preview(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Option<String>> {
    if let Some(read_conn) = state.db_read.as_ref() {
        let conn = read_conn.lock().await;
        history_service::get_thread_preview(&conn, &id)
    } else {
        let conn = state.db.lock().await;
        history_service::get_thread_preview(&conn, &id)
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_thread_message_version(
    state: State<'_, AppState>,
    thread_id: String,
    message_id: String,
) -> AppResult<Option<Message>> {
    if let Some(read_conn) = state.db_read.as_ref() {
        let conn = read_conn.lock().await;
        history_service::get_thread_message_version(&conn, &thread_id, &message_id)
    } else {
        let conn = state.db.lock().await;
        history_service::get_thread_message_version(&conn, &thread_id, &message_id)
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_version_detail(
    state: State<'_, AppState>,
    thread_id: String,
    message_id: String,
) -> AppResult<VersionDetail> {
    let started = Instant::now();
    let conn = if let Some(read_conn) = state.db_read.as_ref() {
        read_conn.lock().await
    } else {
        state.db.lock().await
    };
    let acquired = Instant::now();
    let result = history_service::get_version_detail(&conn, &thread_id, &message_id);
    let finished = Instant::now();
    if let Ok(value) = &result {
        crate::transport_budget::observe_projection(
            "get_version_detail",
            "version_core",
            value,
            1,
            value.truncated_fields.len(),
            2,
            acquired.duration_since(started),
            finished.duration_since(acquired),
        );
    }
    result
}

#[tauri::command]
#[specta::specta]
pub async fn get_version_source_window(
    state: State<'_, AppState>,
    thread_id: String,
    message_id: String,
    start_byte: usize,
    max_bytes: usize,
) -> AppResult<SourceWindow> {
    let conn = state.db.lock().await;
    history_service::get_version_source_window(
        &conn,
        &thread_id,
        &message_id,
        start_byte,
        max_bytes,
    )
}

#[tauri::command]
#[specta::specta]
pub async fn get_dense_topology_page(
    state: State<'_, AppState>,
    thread_id: String,
    message_id: String,
    kind: DenseTopologyKind,
    cursor: Option<String>,
    limit: Option<usize>,
) -> AppResult<DenseTopologyPage> {
    let conn = state.db.lock().await;
    history_service::get_dense_topology_page(&conn, &thread_id, &message_id, kind, cursor, limit)
}

fn preview_runtime_root(app: &dyn crate::models::PathResolver) -> AppResult<PathBuf> {
    Ok(crate::freecad::runtime_cache_dir(app)?.join("history-preview"))
}

fn copy_preview_file(source: &Path, target: &Path) -> AppResult<()> {
    let parent = target
        .parent()
        .ok_or_else(|| AppError::internal("History preview target has no parent."))?;
    fs::create_dir_all(parent).map_err(|error| {
        AppError::persistence(format!(
            "Failed to create history preview directory: {error}"
        ))
    })?;
    fs::copy(source, target).map_err(|error| {
        AppError::persistence(format!(
            "Failed to copy history preview STL '{}': {error}",
            source.display()
        ))
    })?;
    Ok(())
}

fn bundle_stl_paths(bundle: &ArtifactBundle) -> impl Iterator<Item = &str> {
    std::iter::once(bundle.model_stl_path.as_str())
        .chain(bundle.viewer_assets.iter().map(|asset| asset.path.as_str()))
}

fn remove_unprotected_old_stls(
    runtime_root: &Path,
    bundle: &ArtifactBundle,
    protected_paths: &HashSet<PathBuf>,
) {
    for raw_path in bundle_stl_paths(bundle) {
        let path = PathBuf::from(raw_path);
        if path.starts_with(runtime_root) && !protected_paths.contains(&path) {
            let _ = fs::remove_file(path);
        }
    }
}

fn ephemeral_preview_bundle(
    runtime_root: &Path,
    lease_id: &str,
    source: &ArtifactBundle,
) -> AppResult<ArtifactBundle> {
    let lease_root = runtime_root.join("history-preview").join(lease_id);
    let mut preview = source.clone();
    let model_target = lease_root.join("model.stl");
    copy_preview_file(Path::new(&source.model_stl_path), &model_target)?;
    preview.model_stl_path = model_target.to_string_lossy().to_string();

    let mut preview_assets = Vec::new();
    for (index, source_asset) in source.viewer_assets.iter().enumerate() {
        if !Path::new(&source_asset.path).is_file() {
            continue;
        }
        let target = lease_root.join("parts").join(format!("{index:04}.stl"));
        copy_preview_file(Path::new(&source_asset.path), &target)?;
        let mut asset = source_asset.clone();
        asset.path = target.to_string_lossy().to_string();
        preview_assets.push(asset);
    }
    preview.viewer_assets = preview_assets;
    Ok(preview)
}

#[tauri::command]
#[specta::specta]
pub async fn materialize_version_preview(
    thread_id: String,
    message_id: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<VersionPreviewRuntime> {
    let (message, is_latest, protected_paths) = {
        let db = state.db.lock().await;
        let message = db::get_thread_message_version(&db, &thread_id, &message_id)
            .map_err(|error| AppError::persistence(error.to_string()))?
            .ok_or_else(|| AppError::not_found("Version not found for history preview."))?;
        let latest = db::get_thread_latest_version(&db, &thread_id)
            .map_err(|error| AppError::persistence(error.to_string()))?;
        let protected_paths = db::get_latest_version_artifact_bundles(&db)
            .map_err(|error| AppError::persistence(error.to_string()))?
            .into_iter()
            .flat_map(|bundle| {
                bundle_stl_paths(&bundle)
                    .map(PathBuf::from)
                    .collect::<Vec<_>>()
            })
            .collect::<HashSet<_>>();
        (
            message,
            latest.is_some_and(|latest| latest.id == message_id),
            protected_paths,
        )
    };

    let stored_bundle = message
        .artifact_bundle
        .clone()
        .ok_or_else(|| AppError::not_found("Version artifact bundle not found."))?;
    let stored_model_exists = Path::new(&stored_bundle.model_stl_path).is_file();
    let (rendered_bundle, manifest) = if stored_model_exists {
        let manifest = match message.model_manifest.clone() {
            Some(manifest) => manifest,
            None => crate::model_runtime::read_model_manifest(&app, &stored_bundle.model_id)?,
        };
        (stored_bundle, manifest)
    } else {
        let output = message
            .output
            .clone()
            .ok_or_else(|| AppError::not_found("Version source not found for history preview."))?;
        let bundle = crate::services::render::render_model_with_previous_manifest(
            &output.macro_code,
            &output.initial_params,
            Some(output.macro_dialect),
            Some(output.geometry_backend),
            output.post_processing.as_ref(),
            message.model_manifest.as_ref(),
            &state,
            &app,
        )
        .await?;
        let manifest = crate::model_runtime::read_model_manifest(&app, &bundle.model_id)?;
        (bundle, manifest)
    };

    if is_latest {
        let db = state.db.lock().await;
        db::update_message_artifact_bundle(&db, &message_id, &rendered_bundle)
            .map_err(|error| AppError::persistence(error.to_string()))?;
        db::update_message_model_manifest(&db, &message_id, &manifest)
            .map_err(|error| AppError::persistence(error.to_string()))?;
        return Ok(VersionPreviewRuntime {
            artifact_bundle: rendered_bundle,
            model_manifest: manifest,
            lease_id: None,
            ephemeral: false,
        });
    }

    let runtime_root = crate::freecad::runtime_cache_dir(&app)?;
    let lease_id = uuid::Uuid::new_v4().to_string();
    let preview_bundle = match ephemeral_preview_bundle(&runtime_root, &lease_id, &rendered_bundle)
    {
        Ok(bundle) => bundle,
        Err(error) => {
            let _ = release_preview_directory(&runtime_root.join("history-preview"), &lease_id);
            return Err(error);
        }
    };
    remove_unprotected_old_stls(&runtime_root, &rendered_bundle, &protected_paths);
    Ok(VersionPreviewRuntime {
        artifact_bundle: preview_bundle,
        model_manifest: manifest,
        lease_id: Some(lease_id),
        ephemeral: true,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn release_version_preview(lease_id: String, app: AppHandle) -> AppResult<()> {
    release_preview_directory(&preview_runtime_root(&app)?, &lease_id)
}

fn release_preview_directory(preview_root: &Path, lease_id: &str) -> AppResult<()> {
    uuid::Uuid::parse_str(&lease_id)
        .map_err(|_| AppError::validation("Invalid history preview lease id."))?;
    let lease_root = preview_root.join(lease_id);
    if lease_root.is_dir() {
        fs::remove_dir_all(&lease_root).map_err(|error| {
            AppError::persistence(format!("Failed to release history preview: {error}"))
        })?;
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_thread_messages_page(
    state: State<'_, AppState>,
    thread_id: String,
    before: Option<String>,
    limit: Option<usize>,
    include_visual_payloads: bool,
) -> AppResult<ThreadMessagesPage> {
    let started = Instant::now();
    let conn = if let Some(read_conn) = state.db_read.as_ref() {
        read_conn.lock().await
    } else {
        state.db.lock().await
    };
    let acquired = Instant::now();
    let result = history_service::get_thread_messages_page(
        &conn,
        &thread_id,
        before,
        limit,
        include_visual_payloads,
    );
    let finished = Instant::now();
    if let Ok(value) = &result {
        crate::transport_budget::observe_projection(
            "get_thread_messages_page",
            "timeline_page",
            value,
            value.messages.len(),
            value.truncated_fields.len(),
            0,
            acquired.duration_since(started),
            finished.duration_since(acquired),
        );
    }
    result
}

#[tauri::command]
#[specta::specta]
pub async fn clear_history(state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.lock().await;
    crate::db::clear_history(&conn).map_err(|err| AppError::persistence(err.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn delete_thread(id: String, state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.lock().await;
    let changed = crate::db::delete_thread(&conn, &id)
        .map_err(|err| AppError::persistence(err.to_string()))?;
    if changed {
        Ok(())
    } else {
        Err(AppError::not_found("Thread not found."))
    }
}

#[tauri::command]
#[specta::specta]
pub async fn rename_thread(id: String, title: String, state: State<'_, AppState>) -> AppResult<()> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("Thread title cannot be empty."));
    }

    let conn = state.db.lock().await;
    let changed = crate::db::update_thread_title(&conn, &id, trimmed)
        .map_err(|err: rusqlite::Error| AppError::persistence(err.to_string()))?;
    if changed {
        Ok(())
    } else {
        Err(AppError::not_found("Thread not found."))
    }
}

#[tauri::command]
#[specta::specta]
pub async fn delete_version(message_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.lock().await;
    history_service::delete_version(&conn, &message_id)
}

#[tauri::command]
#[specta::specta]
pub async fn restore_version(message_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.lock().await;
    history_service::restore_version(&conn, &message_id)
}

#[tauri::command]
#[specta::specta]
pub async fn get_deleted_messages(
    state: State<'_, AppState>,
) -> AppResult<Vec<crate::contracts::DeletedMessage>> {
    if let Some(read_conn) = state.db_read.as_ref() {
        let conn = read_conn.lock().await;
        crate::db::get_deleted_messages(&conn)
            .map_err(|err: rusqlite::Error| AppError::persistence(err.to_string()))
    } else {
        let conn = state.db.lock().await;
        crate::db::get_deleted_messages(&conn)
            .map_err(|err: rusqlite::Error| AppError::persistence(err.to_string()))
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_deleted_threads_page(
    before: Option<String>,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> AppResult<crate::contracts::DeletedThreadsPage> {
    let load = |conn: &rusqlite::Connection| {
        crate::db::get_deleted_threads_page(conn, before.as_deref(), limit.unwrap_or(24))
            .map_err(|err| AppError::persistence(err.to_string()))
    };
    if let Some(read_conn) = state.db_read.as_ref() {
        let conn = read_conn.lock().await;
        load(&conn)
    } else {
        let conn = state.db.lock().await;
        load(&conn)
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_deleted_thread_preview(
    id: String,
    state: State<'_, AppState>,
) -> AppResult<Option<String>> {
    let load = |conn: &rusqlite::Connection| {
        crate::db::get_deleted_thread_preview(conn, &id)
            .map_err(|err| AppError::persistence(err.to_string()))
    };
    if let Some(read_conn) = state.db_read.as_ref() {
        let conn = read_conn.lock().await;
        load(&conn)
    } else {
        let conn = state.db.lock().await;
        load(&conn)
    }
}

#[tauri::command]
#[specta::specta]
pub async fn restore_deleted_thread(id: String, state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.lock().await;
    let changed = crate::db::restore_deleted_thread(&conn, &id)
        .map_err(|err| AppError::persistence(err.to_string()))?;
    if changed {
        Ok(())
    } else {
        Err(AppError::not_found("Deleted project not found."))
    }
}

#[tauri::command]
#[specta::specta]
pub async fn hide_deleted_message(message_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.lock().await;
    let changed = crate::db::hide_deleted_message(&conn, &message_id)
        .map_err(|err: rusqlite::Error| AppError::persistence(err.to_string()))?;
    if changed {
        Ok(())
    } else {
        Err(AppError::not_found(
            "Deleted message not found or already hidden.",
        ))
    }
}

#[tauri::command]
#[specta::specta]
pub async fn finalize_thread(
    id: String,
    message_id: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let conn = state.db.lock().await;
    history_service::finalize_thread(&conn, &id, message_id.as_deref())
}

#[tauri::command]
#[specta::specta]
pub async fn reopen_thread(id: String, state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.lock().await;
    history_service::reopen_thread(&conn, &id)
}

#[tauri::command]
#[specta::specta]
pub async fn get_inventory(state: State<'_, AppState>) -> AppResult<Vec<Thread>> {
    if let Some(read_conn) = state.db_read.as_ref() {
        let conn = read_conn.lock().await;
        history_service::get_inventory(&conn)
    } else {
        let conn = state.db.lock().await;
        history_service::get_inventory(&conn)
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_thread_window_layout(
    thread_id: String,
    state: State<'_, AppState>,
) -> AppResult<Option<crate::contracts::ThreadWindowLayout>> {
    if let Some(read_conn) = state.db_read.as_ref() {
        let conn = read_conn.lock().await;
        crate::db::get_thread_window_layout(&conn, &thread_id)
            .map_err(|err| AppError::persistence(err.to_string()))
    } else {
        let conn = state.db.lock().await;
        crate::db::get_thread_window_layout(&conn, &thread_id)
            .map_err(|err| AppError::persistence(err.to_string()))
    }
}

#[tauri::command]
#[specta::specta]
pub async fn save_thread_window_layout(
    thread_id: String,
    layout: crate::contracts::ThreadWindowLayout,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let conn = state.db.lock().await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let saved = crate::db::save_thread_window_layout(&conn, &thread_id, &layout, now)
        .map_err(|err| AppError::persistence(err.to_string()))?;
    if saved {
        Ok(())
    } else {
        Err(AppError::not_found("Thread not found."))
    }
}

#[cfg(test)]
mod tests {
    use super::release_preview_directory;
    use std::fs;

    #[test]
    fn history_preview_release_deletes_only_valid_lease_directory() {
        let root = std::env::temp_dir().join(format!(
            "ecky-history-preview-release-{}",
            uuid::Uuid::new_v4()
        ));
        let lease_id = uuid::Uuid::new_v4().to_string();
        let lease_root = root.join(&lease_id);
        let sibling = root.join("keep");
        fs::create_dir_all(&lease_root).expect("lease directory");
        fs::create_dir_all(&sibling).expect("sibling directory");
        fs::write(lease_root.join("model.stl"), b"preview").expect("preview STL");

        release_preview_directory(&root, &lease_id).expect("release lease");

        assert!(!lease_root.exists(), "lease directory must be deleted");
        assert!(sibling.exists(), "sibling directory must stay");
        assert!(release_preview_directory(&root, "../keep").is_err());
        assert!(
            sibling.exists(),
            "invalid lease must not escape preview root"
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
