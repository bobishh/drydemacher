use crate::campaign_projects;
use crate::contracts::{
    ActiveProjectNavigation, AppResult, CampaignRun, CreateCampaignRunInput, ThreadWindowLayout,
};
use crate::models::AppState;
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn transition_campaign_run(
    input: crate::services::campaign_transition::TransitionCampaignRunInput,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> AppResult<crate::services::campaign_transition::TransitionCampaignRunResult> {
    let root = crate::campaign_definition::packaged_root(&app)?;
    let mut db = state.db.lock().await;
    crate::services::campaign_transition::transition_campaign_run(&mut db, &root, input)
}

#[tauri::command]
#[specta::specta]
pub async fn open_campaign_project(
    intent: crate::services::campaign_project_open::OpenCampaignProjectIntent,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> AppResult<crate::services::campaign_project_open::OpenCampaignProjectResult> {
    let root = crate::campaign_definition::packaged_root(&app)?;
    let mut db = state.db.lock().await;
    crate::services::campaign_project_open::open_campaign_project(&mut db, &root, intent)
}
#[tauri::command]
#[specta::specta]
pub async fn create_campaign_run(
    input: CreateCampaignRunInput,
    state: State<'_, AppState>,
) -> AppResult<CampaignRun> {
    let mut db = state.db.lock().await;
    campaign_projects::create(&mut db, input)
}
#[tauri::command]
#[specta::specta]
pub async fn list_campaign_runs(state: State<'_, AppState>) -> AppResult<Vec<CampaignRun>> {
    let db = state.db.lock().await;
    campaign_projects::list(&db)
}
#[tauri::command]
#[specta::specta]
pub async fn get_campaign_run(id: String, state: State<'_, AppState>) -> AppResult<CampaignRun> {
    let db = state.db.lock().await;
    campaign_projects::get(&db, &id)
}
#[tauri::command]
#[specta::specta]
pub async fn save_campaign_run(
    run: CampaignRun,
    state: State<'_, AppState>,
) -> AppResult<CampaignRun> {
    let mut db = state.db.lock().await;
    campaign_projects::save(&mut db, run)
}
#[tauri::command]
#[specta::specta]
pub async fn delete_campaign_run(id: String, state: State<'_, AppState>) -> AppResult<()> {
    let mut db = state.db.lock().await;
    campaign_projects::delete_with_navigation(&mut db, &id)
}

#[tauri::command]
#[specta::specta]
pub async fn get_active_project_navigation(
    state: State<'_, AppState>,
) -> AppResult<Option<ActiveProjectNavigation>> {
    let db = state.db.lock().await;
    campaign_projects::get_active_project_navigation(&db)
}

#[tauri::command]
#[specta::specta]
pub async fn save_active_project_navigation(
    navigation: ActiveProjectNavigation,
    state: State<'_, AppState>,
) -> AppResult<ActiveProjectNavigation> {
    let db = state.db.lock().await;
    campaign_projects::save_active_project_navigation(&db, navigation)
}

#[tauri::command]
#[specta::specta]
pub async fn clear_active_project_navigation(state: State<'_, AppState>) -> AppResult<()> {
    let db = state.db.lock().await;
    campaign_projects::clear_active_project_navigation(&db)
}

#[tauri::command]
#[specta::specta]
pub async fn get_app_window_layout(
    state: State<'_, AppState>,
) -> AppResult<Option<ThreadWindowLayout>> {
    let db = state.db.lock().await;
    campaign_projects::get_app_window_layout(&db)
}

#[tauri::command]
#[specta::specta]
pub async fn save_app_window_layout(
    layout: ThreadWindowLayout,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let db = state.db.lock().await;
    campaign_projects::save_app_window_layout(&db, layout)
}
