use crate::contracts::{DesignOutput, GeometryBackend, MacroDialect, SourceLanguage};
use crate::mcp::contracts::TargetAuthoringContext;

pub(crate) const ECKY_AUTHORING_CARD: &str = concat!(
    "Ecky authoring card\n",
    "- First read current target settings from `workspace_overview`, `target_macro_get`, or `target_detail_get`: `sourceLanguage`, `macroDialect`, `geometryBackend`.\n",
    "- Preserve model-version settings. For empty threads, use session config/defaults; threads do not own language/backend.\n",
    "- If `sourceLanguage=ecky`, write valid `.ecky` only. Generated Ecky model policy requires top-level `(model ...)` and `(verify ...)` together. Do not generate a model without authored verification clauses; if metric support is missing, state the blocker instead of omitting verification.\n",
    "- Only when `sourcePath` is absent, use `macro_buffer_get` for non-trivial edits, edit against line numbers/digest, then render with `macro_buffer_replace_and_preview`; use full `macro_preview_render` only for small complete rewrites.\n",
    "- Do not reuse parameter keys with different meanings. Keep macroCode, uiSpec, and initialParams aligned; remove stale params.\n",
    "- Valid basics: `(box 40 20 10 :align '(min center min))`, `(extrude (polygon ((0 0) (100 0) (100 20) (0 20))) 8)`, `(place (location (plane :origin '(80 0 6)) :rotate '(0 90 0)) (cylinder 4 18))`.\n",
    "- When authoring new physical values, emit suffixed literals in source (`70mm`, `1in`, `45deg`, `0.5rad`) instead of bare numbers when the unit matters.\n",
    "- `let` is parallel; use `let*` when later bindings depend on earlier bindings.\n",
    "- NEVER use `(define ...)` inside `(model ...)`. It fails with a misleading TypeMismatch because Steel evaluates it eagerly before params have values. Use `let*` inside each `(part ...)` to compute derived values from params. Top-level `(define (fn args) ...)` helper functions OUTSIDE `(model ...)` are allowed and correct for reusable pure functions.\n",
    "- Guide routing is dynamic. For `sourceLanguage=ecky`, read `ecky://guides/ecky-source` as the primary language guide. Read backend manifests only when checking a specific op/support question. Read prose backend guides only after a lowerer/render error or artifact/export claim.\n",
    "- For repeated structures in new CAD models, prefer `repeat`, `instance`, or a named `define-component`; avoid copy-paste duplicate geometry blocks.\n",
    "- For authored `mesh`/`polyhedron`, prefer bounded formula-generated vertex/triangle lists over copied literal blocks. Add topology verification for boundary/non-manifold edges and connected components before printability or solid claims.\n",
    "- Reference images are inferred approximation inputs; treat geometry from them as inferred approximation source. Vision text cannot claim reconstruction or accepted CAD; normal compile, preview, structural verification, and exact artifact gates decide artifact truth.\n",
    "- Components: `(define-component name ((number key default :label ...) ...) body)` defines a closed, parameterized geometry unit; instantiate with keywords `(name :key value)`. Bodies may only reference signature keys and local bindings; pasted components carry their own `(verify ...)` clauses, tag-namespaced per instantiating part (`partkey/tag`).\n",
    "- Component library workflow: lift a proven part with `component_extract` (pass the part key and source; `save=true` stores it), discover reusable components with `component_search` (compact headers only), fetch full copy-inline source with `component_get`, then paste the `define-component` into the model and instantiate it.\n",
    "- Component reuse modes are explicit: `component_get` vendors copy-inline source and creates no dependency; authored `(import-component \"package.id\" :version \"1.2.0\" :component \"component-id\" :as alias)` keeps an exact live package reference. Live imports use the committed canonical lock. Never use ranges, `latest`, implicit upgrades, or per-model dependency copies.\n",
    "- STEP live components require locked payload digest plus package-carried geometry provenance. They enter native Direct OCCT as BRep. Never route STEP through FreeCAD, STL conversion, `solidify`, hidden repair, or implicit fuse.\n",
    "- Bound source authoring: a bound target exposes `sourcePath`, `sourceFolder`, and `sourceState` in `workspace_overview.defaultTarget` and `target_meta_get`. When `sourcePath` is present, this file flow takes precedence over macro buffer/rewrite tools. Edit the file at `sourcePath` directly with normal file tools, then run the source sync flow (`project_folder_apply`, slug = the bound folder name) to validate, preview, and commit the edited file as a new version. `sourceState` / `project_folder_status` reports clean/fileChanged/threadAdvanced/conflict; conflicts need `force=true`. Do NOT export a fresh mirror first — `project_folder_export` only re-seeds a missing folder. Never write version history outside this flow.\n",
    "- Derived values should appear once. If the same fit math or repeated body setup appears across parts, lift it into model-level `let*`, helper `define`, or a `define-component` instead of copy-paste.\n",
    "- Physical fit relations need explicit names. Do not leave anonymous offsets like `(+ holder_w 12)` in fit-critical placement/dimension expressions; introduce named bindings or named constraints.\n",
    "- Any fit-critical face/edge selector should be tagged with `tag-face` or `tag-edges` before downstream shell/fillet/chamfer/cut use so topology can rebind across param changes.\n",
    "- If a fit-critical clause selector depends on topology created inside `build`, add `:created-by <shape>` to scope candidates to that helper shape instead of unrelated matching faces/edges.\n",
    "- Debug overlays are preview-only diagnostics. They are forbidden in production export geometry.\n",
    "- Fillet/chamfer are topology-sensitive. If a selector matches no edges after one smaller-radius retry and one selector retry, stop retrying fillet/chamfer; rebuild the shape with rounded source geometry (`rounded-rect`, `rounded-polygon`, `offset-rounded`, `loft`, `taper`, `cone`, or explicit profiles).\n",
    "- Do not promise STEP unless current artifact truth says `hasStepExport=true` or exportArtifacts contains `format=step`; `mesh`/`native` selects Ecky native lowering, not an automatic STEP guarantee.\n",
    "- Before describing STEP representation, inspect artifact digest `geometryRepresentation`, `facetedStep`, and `analyticStep`. A solidified mesh STEP is faceted poly-BRep, never analytic source CAD.\n",
    "- MCP-first workflow: inspect with `target_detail_get(section=\"shapeGraph\")`, patch with `ecky_ast_*` when possible, validate with `ecky_constraints_validate`, preview via render, call `verify_generated_model`, repair red verification with another preview, then commit only green verification.\n",
    "- Prefer AST patches over full macro rewrites when an `ecky_ast_*` operation can express the edit.\n",
    "- For geometry with explicit topology/printability requirements, add top-level `(verify ...)` clauses before preview when shipped manifest/STL metrics can express the requirement; example metrics: `(stl non-manifold-edge-count)`, `(stl connected-component-count)`, `(stl overhang-face-count)`, and `(manifest has-step)`.\n",
    "- After preview/render, call `verify_generated_model` before commit. This runs authored `(verify ...)` clauses plus structural verification. Use `get_structural_verification_summary` only for summary/readback, not as the primary gate.\n",
    "- For an existing design target, call `thread_borrow`; for a brand-new design, call `thread_create`. Then read `sourcePath`, edit that bound file, and sync it with `project_folder_apply`.\n",
    "- For an unbound legacy target only, render with `macro_preview_render`. If validation fails, surface exact raw error, fix source properly, and render again.\n",
    "- Persist green verified previews with `commit_preview_version`; include returned `threadId`, `messageId`, and `modelId` in agent evidence. If repair cap is exhausted and verification stays red, do not commit; report capped red with exact failing issue codes/messages.\n",
    "- Never write `history.sqlite` directly from scripts or agents. Version updates must flow through MCP tools only.\n",
    "- Verify geometry visually with `get_model_screenshot` after `verify_generated_model` passes; authored verify covers measurable structure, not visual/mechanical intent.\n"
);

pub(crate) fn authoring_card_text() -> &'static str {
    ECKY_AUTHORING_CARD
}

pub(crate) fn target_authoring_context(design_output: &DesignOutput) -> TargetAuthoringContext {
    TargetAuthoringContext {
        source_language: design_output.source_language.as_str().to_string(),
        macro_dialect: macro_dialect_label(&design_output.macro_dialect).to_string(),
        geometry_backend: design_output.geometry_backend.as_str().to_string(),
        file_extension: file_extension(
            design_output.source_language,
            design_output.geometry_backend,
        )
        .to_string(),
        authoring_card: ECKY_AUTHORING_CARD.to_string(),
        guide_uris: guide_uris(
            design_output.source_language,
            design_output.geometry_backend,
        ),
    }
}

pub(crate) fn guide_uris(
    source_language: SourceLanguage,
    geometry_backend: GeometryBackend,
) -> Vec<String> {
    let mut uris = vec![
        "ecky://guides/authoring-card".to_string(),
        "ecky://guides/modeling-guidelines".to_string(),
    ];

    if source_language == SourceLanguage::EckyIrV0 {
        uris.push("ecky://guides/ecky-source".to_string());
        uris.push(match geometry_backend {
            GeometryBackend::Build123d => "ecky://guides/surface-manifest/ecky-rust".to_string(),
            GeometryBackend::Freecad => "ecky://guides/surface-manifest/freecad".to_string(),
            GeometryBackend::EckyRust => "ecky://guides/surface-manifest/ecky-rust".to_string(),
        });
    } else {
        uris.push(match geometry_backend {
            GeometryBackend::Build123d => "ecky://guides/ecky-rust".to_string(),
            GeometryBackend::Freecad => "ecky://guides/freecad".to_string(),
            GeometryBackend::EckyRust => "ecky://guides/ecky-rust".to_string(),
        });
    }

    uris
}

pub(crate) fn file_extension(
    source_language: SourceLanguage,
    geometry_backend: GeometryBackend,
) -> &'static str {
    match source_language {
        SourceLanguage::EckyIrV0 => ".ecky",
        SourceLanguage::Build123d => ".py",
        SourceLanguage::LegacyPython => match geometry_backend {
            GeometryBackend::Freecad => ".FCMacro",
            _ => ".py",
        },
    }
}

pub(crate) fn macro_dialect_label(dialect: &MacroDialect) -> &'static str {
    match dialect {
        MacroDialect::Legacy => "legacy",
        MacroDialect::CadFrameworkV1 => "cadFrameworkV1",
        MacroDialect::EckyIrV0 => "ecky",
        MacroDialect::Build123d => "build123d",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authoring_card_requires_artifact_truth_for_step_claims() {
        let card = authoring_card_text();

        assert!(card.contains("Do not promise STEP"));
        assert!(card.contains("hasStepExport=true"));
        assert!(card.contains("exportArtifacts contains `format=step`"));
        assert!(card.contains("`mesh`/`native` selects Ecky native lowering"));
        assert!(card.contains("not an automatic STEP guarantee"));
    }

    #[test]
    fn authoring_card_keeps_mesh_and_vision_claims_bounded() {
        let card = authoring_card_text();

        assert!(card.contains("formula-generated vertex/triangle lists"));
        assert!(card.contains("topology verification"));
        assert!(card.contains("Reference images are inferred"));
        assert!(card.contains("facetedStep"));
        assert!(card.contains("analyticStep"));
        assert!(card.contains("geometryRepresentation"));
    }

    #[test]
    fn authoring_card_stops_blind_fillet_retries() {
        let card = authoring_card_text();

        assert!(card.contains("Fillet/chamfer are topology-sensitive"));
        assert!(card.contains("selector matches no edges"));
        assert!(card.contains("stop retrying fillet/chamfer"));
        assert!(card.contains("rounded source geometry"));
    }

    #[test]
    fn authoring_card_enforces_shapegraph_ast_validate_flow() {
        let card = authoring_card_text();

        assert!(card.contains("target_detail_get(section=\"shapeGraph\")"));
        assert!(card.contains("ecky_ast_*"));
        assert!(card.contains("ecky_constraints_validate"));
        assert!(card.contains("Prefer AST patches over full macro rewrites"));
    }

    #[test]
    fn authoring_card_requires_named_fit_offsets() {
        let card = authoring_card_text();

        assert!(card.contains("Physical fit relations need explicit names"));
        assert!(card.contains("Do not leave anonymous offsets"));
        assert!(card.contains("(+ holder_w 12)"));
    }

    #[test]
    fn authoring_card_requires_derived_values_to_be_hoisted() {
        let card = authoring_card_text();

        assert!(card.contains("Derived values should appear once"));
        assert!(card.contains("model-level `let*`"));
        assert!(card.contains("helper `define`"));
        assert!(card.contains("`define-component`"));
    }

    #[test]
    fn authoring_card_requires_tagged_fit_critical_selectors() {
        let card = authoring_card_text();

        assert!(card.contains("fit-critical face/edge selector should be tagged"));
        assert!(card.contains("tag-face"));
        assert!(card.contains("tag-edges"));
        assert!(card.contains("topology can rebind across param changes"));
        assert!(card.contains(":created-by <shape>"));
    }

    #[test]
    fn authoring_card_requires_model_with_verify_for_generated_policy() {
        let card = authoring_card_text();

        assert!(card.contains("Generated Ecky model policy requires top-level `(model ...)` and `(verify ...)` together"));
        assert!(card.contains("Do not generate a model without authored verification clauses"));
    }

    #[test]
    fn authoring_card_requires_suffixed_unit_literals_for_physical_values() {
        let card = authoring_card_text();

        assert!(card.contains("emit suffixed literals"));
        assert!(card.contains("70mm"));
        assert!(card.contains("1in"));
        assert!(card.contains("45deg"));
        assert!(card.contains("0.5rad"));
    }

    #[test]
    fn authoring_card_requires_verify_repair_before_green_commit() {
        let card = authoring_card_text();

        assert!(card.contains("preview via render, call `verify_generated_model`, repair red verification with another preview, then commit only green verification"));
        assert!(card.contains("Persist green verified previews with `commit_preview_version`"));
        assert!(
            card.contains("If repair cap is exhausted and verification stays red, do not commit")
        );
    }

    #[test]
    fn authoring_card_separates_vendor_and_locked_live_imports() {
        let card = authoring_card_text();

        assert!(card.contains("`component_get` vendors copy-inline source"));
        assert!(card.contains("(import-component"));
        assert!(card.contains("committed canonical lock"));
        assert!(card.contains("Never use ranges, `latest`, implicit upgrades"));
        assert!(card.contains("STEP live components require locked payload digest"));
        assert!(card.contains("Never route STEP through FreeCAD"));
    }

    // openspec thread-source-binding 4.2: normal authoring guidance is
    // edit-supplied-sourcePath-then-sync, never export-first.
    #[test]
    fn authoring_card_directs_edit_bound_source_then_sync_not_export_first() {
        let card = authoring_card_text();

        // Agent reads the bound source location from target metadata.
        assert!(
            card.contains("`sourcePath`"),
            "card must name the supplied sourcePath field"
        );
        assert!(
            card.contains("`sourceFolder`"),
            "card must name the supplied sourceFolder field"
        );
        assert!(
            card.contains("`sourceState`"),
            "card must name the supplied sourceState field"
        );
        assert!(
            card.contains("workspace_overview.defaultTarget"),
            "card must point at workspace_overview.defaultTarget for the fields"
        );
        assert!(
            card.contains("target_meta_get"),
            "card must point at target_meta_get for the fields"
        );

        // Edit the supplied file, then run the existing source sync flow.
        assert!(
            card.contains("Edit the file at `sourcePath`"),
            "card must direct editing the supplied sourcePath file"
        );
        assert!(
            card.contains("source sync flow"),
            "card must name the source sync flow"
        );
        assert!(
            card.contains("project_folder_apply"),
            "card must name the existing source sync tool"
        );

        // Export-first framing is removed.
        assert!(
            !card.contains("mirrors the active macro"),
            "card must not frame authoring as export-first"
        );
        assert!(
            !card.contains("Stale folders need re-export"),
            "card must not instruct re-export for normal authoring"
        );
        assert!(
            card.contains("Do NOT export a fresh mirror first"),
            "card must explicitly reject export-first authoring"
        );
        assert!(
            card.contains("When `sourcePath` is present, this file flow takes precedence"),
            "bound-source flow must explicitly override legacy macro mutation guidance"
        );
        assert!(
            card.contains("Only when `sourcePath` is absent"),
            "legacy macro mutation tools must be scoped to unbound targets"
        );
    }
}
