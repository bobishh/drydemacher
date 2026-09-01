# Design: Foreign Component Import Lifecycle

## Artifact Model

Imported versions use one explicit aggregate:

```text
ImportedComponentVersion
  descriptor: freecad-component + sourceDigest + parameters
  donor: immutable FCStd or STEP
  assets: calculated STL + native runtime files
  evidence: bounds, topology/features, warnings
  bindings: manifest part/object/parameter mapping
  runtime: artifactBundle + contentHash + modelManifest
```

Generated Ecky versions use the existing Ecky aggregate. Both implement the same
history and Viewer handoff contract, but have different materializers: Ecky source
renders through `render_model`; imported-component parameters apply through
`apply_imported_model`. Frontend payloads use camelCase; Rust structs use
snake_case and `#[serde(rename_all = "camelCase")]`.

## Import Flow

```text
select recursive root
-> discover foreign files
-> calculate printable STL once
-> persist donor, STL, native runtime, manifest, and bindings by digest
-> write typed freecad-component descriptor
-> create imported-component visible version with ready runtime
-> expose analysisPending to normal agent context
-> agent authors Ecky through inspect -> validate -> preview -> commit
-> optionally component_extract --save
```

No chat message represents the job. Imported-component record and referenced
assets are authority. STL is a preview/export derivative, not component source.

## Materialization and Cache

All versions follow one Viewer handoff, after strategy-specific materialization:

```text
version identity
-> inspect persisted runtime bundle by content hash
-> cache hit: load model STL without render
-> imported parameter change: apply imported component bindings
-> Ecky source change: render Ecky source
-> atomically hand artifact to Viewer
```

Opening or switching to an unchanged imported version does not invoke either
materializer. Missing persisted imported assets produce an exact repair error;
the app does not silently replace component semantics with a mesh import.

The Viewer retains the previous committed model during target inspection. It
replaces the scene only after the target model loads successfully. Clearing
session pointers is not a visual blanking operation.

## Evidence and Code

Code displays the exact compact evidence passed to the agent. FCStd evidence is a
feature-tree document. STEP/BREP evidence includes units, bounds, volume/centroid,
body hierarchy, analytic surface/curve facts, axes/radii, adjacency, and repeated
feature candidates where deterministically measurable. Raw binary bytes never
appear in Code.

Code exposes two tabs while foreign provenance remains active. `SUMMARY` contains
complete read-only evidence. `COMPONENT` shows the typed
`(freecad-component ...)` descriptor with source identity, bindings, and editable
parameters. `APPLY` and `COMMIT VERSION` submit one imported-parameter intent by
canonical thread and message identity. Rust resolves the stored runtime, calls
`apply_imported_model`, carries semantic bindings, optionally appends the immutable
success/error version, and writes the canonical runtime snapshot. `OPEN CAD` opens
the copied donor. No macro-source lookup runs for this mode.

Evidence storage is complete. Token bounds apply only to each agent response,
never to stored evidence or the user-visible report. Agent access has three
levels: compact document summary, cursor-paged/searchable part index, and exact
part detail by `partId`. Every page reports `total`, `returned`, and `nextCursor`.
Cursors bind to `sourceDigest` plus stable part ordering, so source drift rejects
continuation instead of silently changing the result set. The agent may traverse
every page. Context references this access surface once rather than duplicating
evidence into chat history.

The UI report is not paged or truncated by token policy. It renders every part;
visual filtering or virtualization may optimize interaction without hiding data.

## Units and Printing

FCStd/STEP import uses the CAD runtime's millimetre geometry contract and shows
final `X x Y x Z mm` bounds. Conversion status never blocks a ready printable
artifact. Standalone mesh-unit handling is outside this change.

## Heuristics

Import enrichment may state deterministic provenance and warnings. It must not
display a probability unless produced by a defined calibrated model with recorded
method/version. The existing constant `0.42` has no semantic meaning and is not
presented.

## Rewrite and GC

Agent reconstruction creates a new source-backed version. Old imported source and
donor remain immutable history. GC may remove an asset only after no live version
references its digest. Geometric equality never authorizes history deletion.

## Ownership

- Library scanner: recursive discovery and stable path identity.
- Import runtime: immutable donor, measurements, calculated STL, evidence.
- History/service layer: typed version identity and durable runtime references.
- Materializers: imported-component apply and Ecky render behind one handoff contract.
- Agent context: pending import projection and evidence reference.
- Evidence reader: summary, paged/searchable index, exact part detail.
- Agent authoring: semantic parametrization and verified Ecky source.
- Frontend: state display and atomic Viewer artifact handoff; no provenance dispatch.
