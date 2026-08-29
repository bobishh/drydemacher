## Context

Current Core IR preserves parameters, parts, explicit `feature :params`, named
`build/shape` bindings, and topology selector tags. Runtime part dependencies are
already computed. Native and direct-OCCT manifest builders then erase them by assigning
the global parameter list to each `PartBinding` and one global `core` group.

Direct OCCT additionally emits authored shape bindings on topology records, but runtime
deserialization drops that field. Mesh-native output retains part identity but cannot
prove exact face-to-shape correspondence.

Frontend ownership logic already trusts `PartBinding.parameterKeys`; its default raw
surface remains flat. Dormant viewport controls exist but are globally disabled because
their former inputs were not trustworthy.

## Goals / Non-Goals

**Goals:**

- Keep parameter ownership deterministic and source-derived.
- Make the default 49-parameter dryer surface navigable before any manual view authoring.
- Preserve exact selection provenance where backend evidence exists.
- Permit direct viewport editing only for a non-empty proven scope.
- Make ambiguity visible without unrelated-control fallback.

**Non-Goals:**

- Recover semantic parameters from STEP.
- Ask an LLM to author `controlViews` or duplicate UI structure in source.
- Claim face-level provenance for mesh-only output.
- Require tags on every generated face or edge.
- Redesign imported FreeCAD controls.

## Decisions

### 1. Core IR is authority

Ownership chain:

`parameter -> AST reference -> named shape/feature -> part/topology target -> control`.

Part dependencies SHALL follow the reachable result graph. Unused build bindings SHALL
not claim parameters. Named shape dependencies SHALL include transitive dependencies of
referenced earlier bindings.

Alternative: scan source text in frontend. Rejected because it duplicates compiler
semantics and loses expanded/helper references.

### 2. Existing manifest structures carry the projection

Use `PartBinding.parameterKeys` for exact part ownership, `ParameterGroup` for stable
model/part/shape sections, and `FeatureGraph` for source/output trace. No `controlViews`
are created for Ecky.

Stable generated identifiers:

- model remainder: `model:parameters`;
- part: `part:<part-id>`;
- named shape: `shape:<part-id>:<shape-name>`;
- explicit feature keeps authored feature id.

Explicit `feature :params` determines primary controls. Inferred reachable dependencies
remain the complete safety set. A parameter claimed by multiple parts appears once in
the model/shared section during default presentation, but remains discoverable from
each selected part.

Each Ecky preview keeps the freshly rendered semantic manifest. It does not carry
forward earlier LLM control primitives, relations, Views, feature graphs, or topology
bindings. This prevents stale semantic data from replacing compiler-derived provenance
or triggering graph-removal fallback warnings after geometry changes.

### 3. Topology evidence is backend-specific

Direct OCCT SHALL decode authored shape bindings on topology entities and map exact
matching bindings to named-shape dependency sets. Multiple exact bindings produce the
stable union of those sets. A tag pointing at a named shape uses the same mapping.

Mesh-native output SHALL provide part-level selection only. It SHALL NOT invent face
ownership from triangle position.

External STEP remains geometry-only. Imported target enrichment stays proposal-based.

### 4. Params starts with ownership sections

Directly below search, Params renders model/shared and part sections in manifest order.
Sections show label and parameter count. Dense sections start collapsed. Parameters are
rendered once in default view; shared parameters are not duplicated under every part.

Selecting a viewport target expands its owning section and foregrounds only exact target
keys when available, otherwise exact part keys. Other sections remain collapsed below.

Alternative: keep flat Parameters and expose grouping only in New Params. Rejected: the
default surface remains the 49-control failure.

### 5. Viewport editing is provenance-gated

Select mode may render the existing Tactical Midnight square control overlay only when:

- a generated Ecky target or part is selected;
- resolved parameter key set is non-empty;
- every shown control maps to that set.

Ambiguous or empty targets show no editable overlay and no global fallback. Orbit and
Measure modes never open controls.

### 6. Prompt guides; compiler proves

Authoring guidance requests stable part IDs, meaningful named build stages, explicit
feature primary params, and tags for interaction-critical topology. Compiler dependency
inference and validation remain authoritative when generated code ignores guidance.

### 7. Dryer is acceptance fixture

Use bound thread `fb741dcf-a1c6-41de-a0eb-c2d9dc939cfd`. Reinspect, validate, preview,
then verify through supported MCP operations. Do not write its SQLite record directly.
Known baseline structural findings for print-layout disconnected parts remain distinct
from this change.

### 8. Watch settled filesystem edits, do not wait for a minute poll

Project mirrors use filesystem notifications as the primary signal, coalesced with a
one-second trailing debounce. A short fallback poll repairs missed platform events.
Only one apply may own a slug/revision; repeated notifications for the same settled
digest do not create retries or duplicate versions.

Alternative: reduce the existing long poll interval only. Rejected because it keeps
latency proportional to polling and increases redundant full-tree scans.

### 9. Geometry cache identity excludes semantic-only topology declarations

Direct OCCT keeps separate identity for evaluated geometry and semantic declarations.
When source geometry digest, evaluated parameters, backend binary digest, and dependency
lock are unchanged, the runtime reuses the stored BRep plus topology report. It then
re-resolves authored/tag selectors and rebuilds selection targets, tagged anchors,
parameter groups, feature graph, and manifest.

`authoredBindings` are semantic provenance attached to reusable topology; their presence
must not disable part geometry cache. A native-runner binary digest change still
invalidates old geometry artifacts, so the first render after a runner rebuild remains
cold by design.

Alternative: cache the full command by complete source digest. Rejected because a
tag-only edit changes source identity while leaving geometry identical.

### 10. Project preview represents current head

Project-card preview identity includes the head message id. A head without `imageData`
shows a placeholder/stale marker rather than an older raster under the head timestamp.
Intentional separated print layout may report disconnected parts as nonblocking evidence;
syntax, missing artifacts, non-manifold geometry, and authored verify failures remain
blocking.

## Risks / Trade-offs

- Reachability analysis may expose latent unused-shape bugs -> unit-test unused and
  transitive bindings before manifest use.
- Existing tests encode global parameter ownership -> update only Ecky-generated
  expectations; imported contracts remain unchanged.
- Overlay can obscure geometry -> selection-only, narrow controls, square bounded shell,
  major containers `overflow: hidden`.
- Dirty worktree overlaps Params and authoring files -> patch minimal regions; preserve
  existing changes.

## Proof

1. Rust red/green: reachable per-part and transitive named-shape dependencies; direct
   topology binding mapping; ambiguous target no fallback.
2. Frontend red/green: 49-field ownership projection, dense collapse, selected scope.
3. Playwright real route: default grouped Params; part selection opens only relevant
   overlay controls; ambiguous/pending selection opens none.
4. `cargo check` from `src-tauri`.
5. MCP dryer validation, rebuild, persisted manifest inspection, and model verification.
6. Watcher integration: settled edit observed/applied within two seconds without duplicate
   versions.
7. Direct-runtime integration: tag-only edit changes semantic manifest while native runner
   invocation count and geometry/topology digests stay unchanged.
8. Project-card BDD: head without image does not display an older thumbnail; real structural
   failures still block while declared print layout disconnects do not.
