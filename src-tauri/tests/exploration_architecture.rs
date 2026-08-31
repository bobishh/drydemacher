use std::fs;
use std::path::PathBuf;

#[test]
fn exploration_contract_has_no_parallel_attempt_candidate_or_commit_entity() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let contract = fs::read_to_string(root.join("src/contracts/exploration_cycle.rs")).unwrap();
    let commands = fs::read_to_string(root.join("src/commands/exploration_cycle.rs")).unwrap();
    let combined = format!("{contract}\n{commands}");

    for forbidden in [
        "struct ExplorationAttempt",
        "struct ExplorationCandidate",
        "promote_exploration",
        "commit_exploration",
        "finalize_exploration",
    ] {
        assert!(
            !combined.contains(forbidden),
            "forbidden parallel lifecycle: {forbidden}"
        );
    }
    assert!(combined.contains("result_version_id"));
    assert!(combined.contains("input_digest"));
    assert!(combined.contains("render_snapshot_id"));
    assert!(combined.contains("artifact_digest"));
}

#[test]
fn frontend_is_projection_not_generation_lifecycle_authority() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let orchestrator =
        std::fs::read_to_string(root.join("src/lib/controllers/requestOrchestrator.ts"))
            .expect("request orchestrator source");

    for forbidden in [
        "class GenerationPipeline",
        "while (attempt <=",
        "waitForThreadBuildSlot",
        "finalizeGenerationAttempt(",
        "persistGenerationDraft(",
        "renderModel(",
        "verifyGeneratedModel(",
        "verifyRender(",
        "findRecentDuplicateRequest",
        "DUPLICATE_REQUEST_WINDOW_MS",
        "persistLastSessionSnapshot(",
        "refreshHistory(",
        "saveConfig(",
    ] {
        assert!(
            !orchestrator.contains(forbidden),
            "frontend still owns generation lifecycle through `{forbidden}`"
        );
    }
    assert!(
        orchestrator.contains("startExplorationRun("),
        "frontend must submit work to the Rust-owned controller"
    );

    let rust_runner =
        std::fs::read_to_string(root.join("src-tauri/src/commands/exploration_run.rs"))
            .expect("Rust exploration runner");
    for forbidden in [
        "Apply one bounded authoring change from current evidence.",
        "Repair the exact deterministic verification failure.",
        "single generated draft",
        "single repair to generated draft",
    ] {
        assert!(
            !rust_runner.contains(forbidden),
            "Rust runner still manufactures generic PLAN content through `{forbidden}`"
        );
    }
    assert!(
        rust_runner.contains("generated.next_action"),
        "Rust runner must consume the provider turn's typed action"
    );

    assert!(
        !root
            .join("src/lib/controllers/verificationLoop.ts")
            .exists(),
        "frontend verification/retry loop must be removed"
    );
    let request_queue = std::fs::read_to_string(root.join("src/lib/stores/requestQueue.ts"))
        .expect("request queue projection source");
    for forbidden in ["coalescePendingInteractive", "MAX_CONCURRENT_LLM"] {
        assert!(
            !request_queue.contains(forbidden),
            "frontend request projection still owns queue policy through `{forbidden}`"
        );
    }

    let bindings = std::fs::read_to_string(root.join("src-tauri/src/bindings.rs"))
        .expect("Tauri binding registry");
    for forbidden in [
        "generation::generate_design",
        "generation::init_generation_attempt",
        "generation::persist_generation_draft",
        "generation::finalize_generation_attempt",
        "generation::verify_render",
        "generation::verify_generated_model",
        "exploration_cycle::next_exploration_cycle",
    ] {
        assert!(
            !bindings.contains(forbidden),
            "public Tauri boundary bypasses Rust controller through `{forbidden}`"
        );
    }
}

#[test]
fn sketch_workspace_submits_one_backend_preview_intent() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let workspace = std::fs::read_to_string(root.join("src/lib/SketchWorkspace.svelte"))
        .expect("SketchWorkspace source");

    assert!(workspace.contains("submitSketchPreview({"));
    assert!(workspace.contains("AUTO SNAP ORTHOGRAPHIC / ${orthographicEvidence"));
    for forbidden in [
        "generateSketchPreviewHull(",
        "analyzeSketchBrepCandidates(",
        "extractBrepHiddenLineProjections(",
        "applyBrepAutoRepairProjection(",
        "autoRepairOrthographicSketchStrokes",
        "AUTO_PREVIEW_DEBOUNCE_MS",
    ] {
        assert!(
            !workspace.contains(forbidden),
            "frontend still owns sketch preview pipeline through `{forbidden}`"
        );
    }
}
