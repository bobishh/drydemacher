# Design: Rust-owned semantic control edits

## Authority

`services::semantic_manifest` owns deterministic control-view upsert/delete.
Callers specify actor source (`manual` for workbench, `llm` for MCP), while the
service owns id trimming, source assignment, order preservation/allocation,
sorting, referential validation, and delete cleanup.

The same service owns tagged semantic edit intents for primitive save/delete,
advisory save/delete, relation save/delete, and imported enrichment-proposal
status. Rust assigns manual identities and sources, protects non-manual entities,
cleans dependent references, rebuilds proposal-derived parameter bindings, and
validates the complete manifest before persistence.

Proposal status supports single-entry workbench edits and atomic batch edits.
Batch validation resolves every proposal id and status before cloning or
rebuilding canonical bindings; one invalid entry rejects the whole command.

Tauri command flow:

`ApplySemanticManifestEditInput { edit: saveView | deleteView | ... } -> load
persisted manifest -> mutate -> validate runtime bundle -> persist
file/message/snapshot -> return canonical ModelManifest`.

MCP keeps its turn/version lifecycle, but delegates manifest mutation to the
same service before its existing immutable-version persistence.

## Frontend boundary

ParamPanel keeps temporary composer fields. It sends one tagged `saveView` or
`deleteView` semantic edit with `modelId` and optional `messageId` in camelCase.
It never supplies the complete manifest or assigns view id/source/status/order.
Returned manifest updates local projections. A bounded pending flag prevents
duplicate user actions; backend errors remain unmodified through existing error
formatting. Superseded direct view save/delete Tauri commands are absent.

For remaining semantic edits ParamPanel sends one tagged `edit` payload plus
`modelId` and optional `messageId`. Composer inference may choose draft labels,
field kinds, scopes, and numeric values; it does not assign canonical ids,
sources, order, derived proposal bindings, or replacement manifest collections.
Import enrichment may send `setProposalStatuses` with camelCase entries; Rust
applies all entries before deriving one aggregate status and binding projection.

Semantic value changes use
`{threadId, targetMessageId, primitiveId, value}`. Rust resolves the exact
immutable target, loads its manifest and UI schema, applies primitive binding
scale/offset/min/max, traverses enabled relations once in canonical order, and
returns a parameter patch. Generated Ecky `ast-param:<key>` controls resolve only
against declared non-frozen UI fields. Legacy derived `primitive-<slug>` identities
are regenerated from declared UI keys in Rust; ambiguous slugs are rejected. The workbench stages the returned patch;
the existing explicit Apply action remains the persistence/render boundary.

## Compatibility

- Imported/legacy non-Ecky manifests keep persisted manual views.
- Ecky-native manifests reject manual view edits.
- Generic `save_model_manifest` remains for measurement and unrelated manifest
  edits only.
