use std::fs;
use std::path::PathBuf;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root")
        .to_path_buf()
}

#[test]
fn boot_restore_submits_one_backend_projection_instead_of_composing_history_reads() {
    let source = fs::read_to_string(repository_root().join("src/lib/boot/restore.ts"))
        .expect("restore source");

    assert!(
        source.contains("getBootProjection"),
        "boot restore must submit one Rust-owned boot projection intent"
    );
    for forbidden in [
        "getConfig,",
        "getHistory,",
        "getLastDesign,",
        "getWorkspaceProjection,",
        "repairDefaultAuthoringContext",
        "getRuntimeCapabilities,",
        "listModels,",
        "saveConfig as persistConfig",
    ] {
        assert!(
            !source.contains(forbidden),
            "boot restore still owns backend policy through {forbidden}"
        );
    }
}

#[test]
fn boot_projection_contract_stays_camel_case() {
    let source =
        fs::read_to_string(repository_root().join("src-tauri/src/contracts/boot_projection.rs"))
            .expect("boot projection contract");

    assert!(source.contains("#[serde(rename_all = \"camelCase\")]"));
    assert!(source.contains("pub struct BootProjection"));
    assert!(source.contains("pub struct BootRuntimeProjection"));
    assert!(source.contains("pub struct ModelCatalogProjection"));
}

#[test]
fn superseded_frontend_mutation_bypasses_are_not_public() {
    let root = repository_root();
    let bindings =
        fs::read_to_string(root.join("src-tauri/src/bindings.rs")).expect("bindings source");
    let client = fs::read_to_string(root.join("src/lib/tauri/client.ts")).expect("client source");
    for forbidden in [
        "component_import_copy_inline",
        "apply_external_shape_plane_crop",
        "remove_external_shape_plane_crop",
        "apply_external_shape_surface_trim",
        "remove_external_shape_surface_trim",
        "save_model_manifest",
        "analyze_sketch_brep_candidates",
        "extract_brep_hidden_line_projections",
        "generate_sketch_preview_hull",
        "load_sketch_preview_draft",
    ] {
        assert!(
            !bindings.contains(forbidden),
            "public binding remains: {forbidden}"
        );
    }
    for forbidden in [
        "applyExternalShapePlaneCrop",
        "removeExternalShapePlaneCrop",
        "applyExternalShapeSurfaceTrim",
        "removeExternalShapeSurfaceTrim",
        "saveModelManifest",
        "analyzeSketchBrepCandidates",
        "extractBrepHiddenLineProjections",
        "generateSketchPreviewHull",
        "loadSketchPreviewDraft",
    ] {
        assert!(
            !client.contains(forbidden),
            "client bypass remains: {forbidden}"
        );
    }
}
