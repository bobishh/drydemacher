use tauri::{AppHandle, State};

use crate::contracts::{
    AppResult, BootProjection, BootRuntimeProjection, Config, ModelCatalogProjection,
};
use crate::models::AppState;

fn persist_repaired_config(config: Config, state: &AppState, app: &AppHandle) -> AppResult<Config> {
    let config_dir = crate::models::PathResolver::try_app_config_dir(app)?;
    let saved = crate::commands::config::persist_config_transaction(&config_dir, config, state)?;
    for warning in saved.warnings {
        state.push_log(warning);
    }
    Ok(state.config.lock().unwrap().clone())
}

#[tauri::command]
#[specta::specta]
pub async fn get_boot_projection(
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<BootProjection> {
    let mut config = state.config.lock().unwrap().clone();
    if crate::services::boot_projection::normalize_boot_config(&mut config) {
        config = persist_repaired_config(config, state.inner(), &app)?;
        let state_for_sync = state.inner().clone();
        tauri::async_runtime::spawn_blocking(move || {
            crate::mcp::runtime::sync_auto_agent_supervisors(state_for_sync);
        });
    }

    let last_snapshot = state.last_snapshot.lock().unwrap().clone();
    let result = {
        let conn = if let Some(read_conn) = state.db_read.as_ref() {
            read_conn.lock().await
        } else {
            state.db.lock().await
        };
        crate::services::boot_projection::build_boot_projection(
            &conn,
            config,
            last_snapshot.as_ref(),
            Some(50),
        )?
    };

    if result.clear_last_snapshot {
        *state.last_snapshot.lock().unwrap() = None;
        crate::services::session::write_last_snapshot(&app, None);
    }

    Ok(result.projection)
}

#[tauri::command]
#[specta::specta]
pub async fn get_boot_runtime_projection(
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<BootRuntimeProjection> {
    let configured_freecad_cmd = crate::services::render::configured_freecad_cmd(state.inner());
    let capabilities = crate::runtime_capabilities::collect_runtime_capabilities(
        configured_freecad_cmd.as_deref(),
        &app,
    );
    let mut config = state.config.lock().unwrap().clone();
    if crate::services::boot_projection::repair_default_authoring_context(
        &mut config,
        &capabilities,
    ) {
        config = persist_repaired_config(config, state.inner(), &app)?;
    }

    Ok(BootRuntimeProjection {
        config,
        capabilities,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn save_config_projection(
    mut config: Config,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<BootRuntimeProjection> {
    crate::services::boot_projection::normalize_boot_config(&mut config);
    let configured_freecad_cmd = match config.freecad_cmd.trim() {
        "" => None,
        command => Some(command.to_string()),
    };
    let capabilities = crate::runtime_capabilities::collect_runtime_capabilities(
        configured_freecad_cmd.as_deref(),
        &app,
    );
    crate::services::boot_projection::repair_default_authoring_context(&mut config, &capabilities);
    let config = persist_repaired_config(config, state.inner(), &app)?;
    let state_for_sync = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::mcp::runtime::sync_auto_agent_supervisors(state_for_sync);
    });

    Ok(BootRuntimeProjection {
        config,
        capabilities,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn refresh_model_catalog(
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<ModelCatalogProjection> {
    let (engine_id, provider, api_key, base_url) = {
        let config = state.config.lock().unwrap();
        let Some(engine) = config
            .engines
            .iter()
            .find(|engine| engine.id == config.selected_engine_id)
        else {
            return Ok(ModelCatalogProjection {
                config: config.clone(),
                models: Vec::new(),
            });
        };
        (
            engine.id.clone(),
            engine.provider.clone(),
            engine.api_key.clone(),
            engine.base_url.clone(),
        )
    };
    if api_key.is_empty() && provider != "ollama" {
        return Ok(ModelCatalogProjection {
            config: state.config.lock().unwrap().clone(),
            models: Vec::new(),
        });
    }

    let models = crate::llm::list_models(&provider, &api_key, &base_url)
        .await
        .map_err(crate::contracts::AppError::provider)?;
    let mut config = state.config.lock().unwrap().clone();
    if crate::services::boot_projection::select_first_available_model(
        &mut config,
        &engine_id,
        &models,
    ) {
        config = persist_repaired_config(config, state.inner(), &app)?;
    }
    Ok(ModelCatalogProjection { config, models })
}
