use crate::freecad;
use crate::models::{AppResult, AppState, ArtifactBundle, DesignParams, PathResolver};

pub fn configured_freecad_cmd(state: &AppState) -> Option<String> {
    let config = state.config.lock().unwrap();
    let cmd = config.freecad_cmd.trim();
    if cmd.is_empty() {
        None
    } else {
        Some(cmd.to_string())
    }
}

pub fn is_freecad_available(state: &AppState) -> bool {
    freecad::resolve_freecad_path(configured_freecad_cmd(state).as_deref()).is_ok()
}

pub async fn render_stl(
    macro_code: &str,
    parameters: &DesignParams,
    state: &AppState,
    app: &dyn PathResolver,
) -> AppResult<String> {
    let _guard = state.render_lock.lock().await;
    let result = freecad::render(
        macro_code,
        parameters,
        configured_freecad_cmd(state).as_deref(),
        app,
    );
    if result.is_ok() {
        let runtime_cache_dir = freecad::runtime_cache_dir(app)?;
        freecad::evict_cache_if_needed(&runtime_cache_dir);
    }
    result
}

pub async fn render_model(
    macro_code: &str,
    parameters: &DesignParams,
    state: &AppState,
    app: &dyn PathResolver,
) -> AppResult<ArtifactBundle> {
    let _guard = state.render_lock.lock().await;
    let result = freecad::render_model(
        macro_code,
        parameters,
        configured_freecad_cmd(state).as_deref(),
        app,
    );
    if result.is_ok() {
        let runtime_cache_dir = freecad::runtime_cache_dir(app)?;
        freecad::evict_cache_if_needed(&runtime_cache_dir);
    }
    result
}
