# Proposal: Rust-owned semantic control edits

## Intent

Remove canonical semantic-control mutation from `ParamPanel.svelte`. UI submits
one semantic intent; Rust loads, validates, mutates, persists, and returns the
canonical manifest. MCP semantic tools and Tauri UI commands share the same
mutation service.

## Scope

- Add shared Rust upsert/delete operations for control views.
- Add one tagged Rust edit boundary for manual primitives, advisories, relations,
  and imported enrichment-proposal status changes.
- Add one Rust value-resolution intent for primitive bindings, clamps, relation
  propagation, and Ecky AST-provenance parameters.
- Add Tauri commands for manual control-view save/delete.
- Keep frontend ownership limited to composer draft, pending/error projection,
  and applying the returned manifest.
- Preserve generated Ecky rule: `controlViews` remain empty.

## Out of scope

- Generation/exploration lifecycle.
- Measurement edit migration.
- Semantic materialization used only for UI display.
- Persisting/rendering a staged semantic value before the existing Apply action.

## Success criteria

- ParamPanel no longer constructs a replacement manifest for control-view
  save/delete and no longer calls generic `saveModelManifest` for those actions.
- Rust rejects Ecky-native control-view persistence and invalid references.
- MCP control-view handlers use the same mutation functions.
- UI proves success, pending suppression, and raw backend failure display.
- Primitive, advisory, relation, and proposal edits no longer submit replacement
  manifests; Rust owns identity, ownership checks, cleanup, binding rebuild, and
  canonical persistence.
- Semantic value changes send exact target/primitive identity plus value; Rust
  returns the canonical parameter patch and the frontend only stages that result.
