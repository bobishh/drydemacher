use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn public_tauri_boundary_exposes_intents_not_legacy_mutation_steps() {
    let root = workspace_root();
    let bindings =
        fs::read_to_string(root.join("src-tauri/src/bindings.rs")).expect("read Tauri bindings");
    let client =
        fs::read_to_string(root.join("src/lib/tauri/client.ts")).expect("read frontend client");

    let legacy_rust_commands = [
        "crate::commands::config::open_or_create_blank_design_thread,",
        "crate::commands::config::save_project_source,",
        "crate::commands::render::apply_imported_model,",
        "crate::commands::render::import_fcstd,",
        "crate::commands::render::import_freecad_library_part,",
        "crate::commands::render::get_model_manifest,",
        "crate::commands::render::get_default_macro,",
        "crate::commands::design::add_manual_version,",
        "crate::commands::design::add_imported_model_version,",
        "crate::commands::history::delete_version,",
        "crate::commands::history::restore_version,",
        "crate::commands::history::delete_thread,",
        "crate::commands::history::finalize_thread,",
        "crate::commands::history::reopen_thread,",
        "crate::commands::fem::validate_fem_study,",
        "crate::commands::fem::preview_fem_mesh,",
        "crate::commands::fem::run_fem_study,",
        "crate::commands::fem::run_fem_convergence,",
        "crate::commands::fem::get_cached_fem_convergence,",
        "crate::commands::fem::export_fem_result_vtu,",
        "crate::commands::capture::get_capture_reconstruction_guide,",
        "crate::commands::capture::get_capture_guide_source_identity,",
        "crate::commands::capture::get_capture_guide_context,",
        "crate::commands::capture::save_capture_reconstruction_guide,",
        "crate::commands::capture::evaluate_capture_reconstruction_guide,",
        "crate::commands::capture::pair_capture_session,",
        "crate::commands::design::update_ui_spec,",
        "crate::commands::design::update_parameters,",
        "crate::commands::render::save_control_view,",
        "crate::commands::render::delete_control_view,",
        "crate::commands::session::get_message_attachments,",
        "crate::commands::session::resolve_agent_prompt,",
    ];
    for command in legacy_rust_commands {
        assert!(
            !bindings.contains(command),
            "legacy multi-step command remains public: {command}"
        );
    }

    let legacy_client_wrappers = [
        "export async function saveProjectSource(",
        "export async function applyImportedModel(",
        "export async function importFcstd(",
        "export async function importFreecadLibraryPart(",
        "export async function getModelManifest(",
        "export async function getDefaultMacro(",
        "export async function addManualVersion(",
        "export async function addImportedModelVersion(",
        "export async function deleteVersion(",
        "export async function restoreVersion(",
        "export async function deleteThread(",
        "export async function finalizeThread(",
        "export async function reopenThread(",
        "export async function validateFemStudy(",
        "export async function previewFemMesh(",
        "export async function runFemStudy(",
        "export async function runFemConvergence(",
        "export async function getCachedFemConvergence(",
        "export async function exportFemResultVtu(",
        "export async function getCaptureReconstructionGuide(",
        "export async function getCaptureGuideSourceIdentity(",
        "export async function getCaptureGuideContext(",
        "export async function saveCaptureReconstructionGuide(",
        "export async function evaluateCaptureReconstructionGuide(",
        "export async function pairCaptureSession(",
        "export async function updateUiSpec(",
        "export async function updateParameters(",
        "export async function saveControlView(",
        "export async function deleteControlView(",
        "export async function getMessageAttachments(",
        "export async function resolveAgentPrompt(",
    ];
    for wrapper in legacy_client_wrappers {
        assert!(
            !client.contains(wrapper),
            "legacy multi-step client wrapper remains public: {wrapper}"
        );
    }
}

#[test]
fn local_history_projection_intents_do_not_emit_refresh_echo_events() {
    let source = fs::read_to_string(workspace_root().join("src-tauri/src/services/history.rs"))
        .expect("read history service");
    let start = source
        .find("pub async fn delete_version_intent")
        .expect("delete version intent");
    let end = source[start..]
        .find("#[cfg(test)]")
        .map(|offset| start + offset)
        .expect("history test module");
    let local_intents = &source[start..end];

    assert!(local_intents.contains("pub async fn restore_version_intent"));
    assert!(local_intents.contains("pub async fn delete_thread_intent"));
    assert!(local_intents.contains("pub async fn finalize_thread_intent"));
    assert!(local_intents.contains("pub async fn reopen_thread_intent"));
    assert!(local_intents.contains("pub async fn open_inventory_thread_intent"));
    assert!(
        !local_intents.contains("emit_history_changed"),
        "authoritative local intent response must not trigger a second refresh IPC"
    );
}
