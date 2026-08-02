use crate::campaign_definition::{self, CampaignStepPayload, CampaignSummary};
use crate::contracts::AppResult;

#[tauri::command]
#[specta::specta]
pub fn list_campaign_definitions(app: tauri::AppHandle) -> AppResult<Vec<CampaignSummary>> {
    campaign_definition::summaries(&campaign_definition::packaged_root(&app)?)
}

#[tauri::command]
#[specta::specta]
pub fn get_campaign_step(
    definition_id: String,
    step_id: String,
    app: tauri::AppHandle,
) -> AppResult<CampaignStepPayload> {
    campaign_definition::step(
        &campaign_definition::packaged_root(&app)?,
        &definition_id,
        &step_id,
    )
}

#[tauri::command]
#[specta::specta]
pub fn check_campaign_step(
    definition_id: String,
    step_id: String,
    candidate_source: String,
    app: tauri::AppHandle,
) -> AppResult<crate::commands::mission_evaluation::MissionCoreIrEvaluation> {
    campaign_definition::check_step(
        &campaign_definition::packaged_root(&app)?,
        &definition_id,
        &step_id,
        candidate_source,
    )
}
