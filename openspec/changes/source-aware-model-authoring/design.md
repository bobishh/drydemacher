# Design: Source-Aware Model Authoring

## Goal

Make existing Ecky models directly understandable and manipulable without
creating a second geometry source of truth.

## Decisions

- `.ecky` source is canonical.
- Backend owns parse, stable AST identity, dependency analysis, handle binding,
  patch validation, compilation, render, and serialization.
- Frontend owns raycasting, synchronized selection, overlays, gizmo interaction,
  draft presentation, and version intent controls.
- Three.js scene is rendered output and interaction surface, never canonical
  geometry.
- LLM may propose guarded AST patches. LLM does not build authoritative
  AST-to-geometry projections.
- Backend emits editability. Frontend never infers editable status from missing
  manifest data.
- Every fit-critical manipulation targets named parameter, binding, or
  constraint.
- Repeated geometry remains authored through `repeat` or `instance`.
- Tauri TypeScript payloads use camelCase. Rust fields use snake_case with
  `#[serde(rename_all = "camelCase")]` at boundary structs.
- Major viewer, overlay, and graph containers use `overflow: hidden`.
- Debug overlays never enter STL or STEP export geometry.

## Rejected Paths

- Raw vertex dragging. Tessellation vertices are unstable compiler output and
  cannot roundtrip safely to source.
- Orthographic projection reconstruction. It loses feature intent and repeats
  geometry already represented by source and manifest.
- Frontend-authored geometry patches. Browser state cannot safely own parse,
  validation, or concurrency.
- Whole dependency graph overlaid on model. It obscures geometry. Overlay shows
  selected path and directly affected nodes only.
- Pretending imported meshes are parametric. Missing provenance stays explicit
  and read-only.

## Unified Authoring Graph

Backend service combines existing AST, dependency, selector, constraint, shape
graph, and model manifest contracts. Tauri command reuses service; it does not
duplicate MCP logic.

```text
AuthoringGraph
  sourceDigest
  coreDigest
  artifactDigest
  astNodes[]
  features[]
  dependencies[]
  constraints[]
  targets[]
  handles[]
```

```text
HandleBinding
  handleId
  kind: linear | radial | angular | point2 | point3 | transform
  stableNodeKey
  targetIds[]
  frame: origin + axes
  parameterKey or valuePath
  value + bounds + step
  constraintIds[]
  editable
  nonEditableReason
```

Stable identity chain:

```text
parameter / AST node
  -> dependency or constraint
  -> feature
  -> feature output
  -> stable viewer target
```

## Manipulation Classes

### Parameter And Feature Handles

First supported matrix:

- translate, rotate, scale source operations
- box width, depth, height
- cylinder radius and height
- extrusion depth
- hole center and radius
- fillet or chamfer amount where source ownership is exact

### Source-Authored Control Points

Later slice supports point tuples owned by polygon, path, and loft AST nodes.
Compiler emits stable point identity and plane/frame mapping. Drag patches point
coordinates, not resulting tessellation vertices.

### Derived Geometry

Faces, edges, or vertices created by boolean, fillet, BRep topology, or
tessellation remain selectable but read-only when no exact source binding exists.
Selection reveals owning feature, upstream params, and reason.

## Drag Lifecycle

```text
pointer drag
  -> raycast stable target
  -> resolve HandleBinding
  -> convert world delta in emitted frame
  -> append exact AST patch draft as immutable version
  -> advance head to appended version
  -> validate and compile that version
  -> attach success or raw failure evidence
  -> highlight affected targets
```

Frontend may show a clearly marked local ghost during drag. Backend-rendered
preview is authoritative for rendered geometry. Content-identical drag samples
may be coalesced before persistence, but every distinct persisted draft is an
immutable version. Superseded, invalid, and failed versions remain in history.
Head always identifies the latest appended version; latest successful render is
a separate projection/filter and never rewrites head.

## Synchronized Lenses

- Geometry click selects target, owning feature, stable AST node, upstream
  params, and affected outputs.
- AST or parameter click highlights corresponding model targets.
- Viewport renders only focused trace:
  `param -> AST operation -> feature -> output target`.
- Full topology and source internals remain in source lens.
- Ambiguous mappings show candidates and require user choice.

## Sketch Retirement Boundary

First slice hides launcher and suppresses stale saved visibility. Sketch code,
commands, draft persistence, and test corpus remain untouched. Deletion requires
separate inventory proving no reused projection, raster, or draft contracts are
needed by source-aware authoring.

## BDD Proof Strategy

Hide slice:

```gherkin
Given workbench loads
When dock controls render
Then Sketch Workspace launcher and window are absent
```

```gherkin
Given saved layout marks Sketch Workspace visible
When layout restores
Then Sketch Workspace stays hidden and other windows restore normally
```

Manipulation slice:

```gherkin
Given an existing source-backed cylinder
When author drags its radius handle from 8 to 12
Then exact draft source is appended as a version, head advances to it, preview
rerenders, and affected targets highlight
```

```gherkin
Given selected derived vertex has no exact source binding
When author requests direct manipulation
Then vertex is read-only, source stays unchanged, and raw binding reason appears
```

Rust changes require `cd src-tauri && cargo check` before completion report.
