# Design: Parameterized STL Import Preparation

## Goal

Make dense imported STL geometry usable through normal Ecky source and UI. Keep
lossy preparation explicit, bounded, reproducible, cacheable, and visible.

## Source Contract

`import-stl` gains three optional keywords:

```lisp
(import-stl path
  :target-triangles integer-expression
  :max-error length-expression
  :preserve-boundaries boolean-expression)
```

- No keywords: current exact import.
- `:target-triangles`: desired upper triangle count, minimum 4.
- `:max-error`: maximum absolute surface deviation in model millimetres.
- `:preserve-boundaries`: protect existing boundaries and source-operation
  boundaries; default `#t` when preparation is enabled.
- Preparation requires both `:target-triangles` and `:max-error`. A triangle
  target alone would authorize unbounded shape loss. An error alone has no clear
  stopping target.
- Keywords accept normal parameter expressions and participate in parameter
  validation and render identity.

`solidify` remains separate:

```text
import-stl        bytes -> validated indexed mesh
preparation       indexed mesh -> bounded derived indexed mesh
solidify          closed indexed mesh -> faceted BRep solid
```

For STL/3MF mesh Boolean output, no `solidify` is required. For BRep Boolean or
faceted STEP output, `solidify` consumes the prepared mesh.

## Representation and Execution Order

An imported source owns two identities:

1. `rawSourceDigest`: immutable source bytes plus canonical indexed parse.
2. `preparedMeshDigest`: raw digest plus normalized preparation policy,
   algorithm version, and deterministic output.

Import evaluation retains the raw indexed mesh and preparation policy until the
source-local mesh stage is complete. Order is:

```text
read bytes
-> validate/index raw mesh
-> resolve raw-source anchors and source-local Crop/Surface Trim
-> protect retained/cap boundaries and anchored vertices
-> apply import preparation policy
-> transforms / mesh Boolean / export
-> optional solidify / BRep operations / faceted STEP
```

This lazy materialization prevents simplification from invalidating existing
`mesh-anchor` triangle indices. `surface-trim` continues to resolve anchors
against the raw digest and raw triangles. Its retained result receives the import
policy only after exact boundary insertion and capping. Plane crop coordinates
remain source-coordinate evidence and do not depend on prepared triangle ids.

## Deterministic Simplification

Use one in-process, versioned indexed-mesh simplifier. Initial implementation may
use meshoptimizer's error-bounded simplifier after a fixed 1e-6 mm seam weld used
by current indexed-mesh admission.

Invariants:

- deterministic output for identical raw digest and normalized policy;
- source winding retained;
- protected boundaries and vertices retained;
- no component merge/split;
- no new boundary or non-manifold edge;
- no inverted or zero-area triangles;
- absolute error `<= max-error`;
- target is soft, error is hard.

If the target cannot be reached within the error/topology constraints, return a
valid prepared mesh with a higher achieved triangle count and a typed
`targetNotReached` warning. Never exceed the error to manufacture the requested
count.

Invalid source topology remains a raw validation failure. Preparation is not a
repair operation.

## Fit and Manufacturing Boundary

Preparation changes manufacturing geometry. UI and provenance state that fact.
Fit-critical imported geometry requires an authored error smaller than its fit
tolerance. Source anchors, trim boundaries, cap boundaries, and future tagged fit
zones are protected. Ecky never silently selects a tolerance or classifies a mesh
as decorative.

## Cache and Provenance

Cache key includes:

```text
raw source digest
+ normalized target triangle count
+ max error in canonical mm bits
+ preserve-boundaries value
+ protected vertex/border digest
+ simplifier algorithm/version
+ indexed-mesh admission version
```

Only successful immutable indexed meshes enter the byte-bounded cache. Concurrent
identical preparation uses existing singleflight behavior. Failures are not
cached.

Artifact provenance adds:

- raw source digest and path identity;
- original vertex/triangle counts;
- requested target triangle count;
- requested maximum error;
- achieved vertex/triangle counts;
- measured achieved maximum and RMS error;
- protected boundary/vertex counts;
- simplifier name/version;
- prepared mesh digest;
- cache hit/miss state.

STL remains an export format. Cache authority is indexed mesh sidecar data, not a
rewritten external STL.

## External Shapes Import UX

Selected source card owns a `DETAIL` section:

```text
GEOMETRY DETAIL
  ORIGINAL | PREPARED
  TARGET TRIANGLES     40 000
  MAX DEVIATION        0.05 mm
  PRESERVE BOUNDARIES  ON

  Original: 375 692 triangles
  Prepared: 39 814 triangles
  Achieved deviation: 0.047 mm

  PREVIEW PREPARED   APPLY
```

- Original is default and writes no keywords.
- Preview operates on selected source only and never changes canonical source.
- Apply AST-patches the exact `import-stl` node guarded by thread, message,
  source digest, and node id.
- Reopen derives controls from canonical source; no hidden panel state.
- Reset removes preparation keywords and restores original evaluation.
- Crop/Guides continue to display raw mesh unless user explicitly selects a
  Prepared Preview overlay. Their anchors always bind raw source identity.
- Progress stages are visible in the active task: `import`, `validate`,
  `prepare`, `preview`, `apply`. Raw backend failure remains visible.

Frontend payloads use camelCase. Rust contracts use snake_case plus
`#[serde(rename_all = "camelCase")]`.

## Compiler and Runtime Ownership

- Scheme compiler: parse typed keywords and expressions into Core IR import
  policy.
- Core IR: store normalized optional `StlPreparationPolicy`; do not invent a
  second public operation.
- Mesh runtime: raw parse, validation, protected-set construction,
  simplification, error measurement, indexed output.
- Hybrid partition: carry prepared indexed asset to mesh Boolean or `solidify`.
- Direct OCCT planner: receive prepared indexed asset reference; never simplify
  independently.
- External Shapes backend: discover policy from selected AST node, preview,
  guard, and patch.
- Viewer: switch raw/prepared assets and show counts; never author coordinates or
  rewrite STL.

## Failure Model

Typed raw errors cover:

- missing/unreadable source;
- invalid target count or error;
- source digest drift;
- stale AST node/thread/message;
- invalid source topology;
- simplifier topology violation;
- measured error above requested maximum;
- protected-set conflict;
- cancellation;
- cache corruption.

Failure leaves canonical source, original STL, and last green artifact unchanged.

## Proof Plan

- Compiler red/green tests for old signature, complete policy, missing paired
  keyword, parameter expressions, and invalid values.
- Mesh-runtime fixture tests for determinism, bounds, error, topology,
  components, protected borders, and target-not-reached warning.
- Cache tests for source/policy/algorithm identity and singleflight.
- Surface Trim test proving raw anchor replay survives preparation.
- Hybrid test proving prepared import feeds `solidify` and later difference.
- Playwright real External Shapes route: Original -> Prepared preview -> Apply ->
  reopen; plus stale-source/raw-failure state.
- `cargo check`, focused Rust tests, focused frontend tests, strict OpenSpec
  validation.
