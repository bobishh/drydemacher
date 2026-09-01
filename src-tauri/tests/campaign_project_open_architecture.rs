use std::fs;
use std::path::PathBuf;

fn workspace(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(path)
}

#[test]
fn campaign_start_and_resume_submit_identity_only_to_one_rust_projection() {
    let app = fs::read_to_string(workspace("src/App.svelte")).expect("App source");
    let client = fs::read_to_string(workspace("src/lib/projects/campaignRunClient.ts"))
        .expect("campaign client");
    let bindings =
        fs::read_to_string(workspace("src-tauri/src/bindings.rs")).expect("Tauri bindings");

    assert!(app.contains("campaignRunClient.open({ kind: 'start', definitionId:"));
    assert!(app.contains("campaignRunClient.open({ kind: 'resume', runId:"));
    assert!(!app.contains("campaignRunClient.create({"));
    assert!(client.contains("openCampaignProject(input)"));
    assert!(bindings.contains("open_campaign_project"));
}

#[test]
fn campaign_open_contract_is_tagged_and_camel_case() {
    let service = fs::read_to_string(workspace("src-tauri/src/services/campaign_project_open.rs"))
        .expect("campaign project open service");

    assert!(service.contains("tag = \"kind\""));
    assert!(service.contains("rename_all = \"camelCase\""));
    assert!(service.contains("definition_id"));
    assert!(service.contains("run_id"));
}
