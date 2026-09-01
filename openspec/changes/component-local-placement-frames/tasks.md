## 1. Acceptance Contract First

- [x] 1.1 Add a failing end-to-end compiler/render fixture with one local
  `c-latch`, front-wall and side-wall target ports, and two model variants whose
  only placement difference is the target `port-ref`; assert orthogonal placed
  bounds and unchanged local component body digest.
- [x] 1.2 Add a failing legacy-source regression proving models without ports or
  mates retain byte-identical stable keys, CoreProgram digest, part bounds, and
  emitted source.
- [x] 1.3 Add failing diagnostics fixtures for invalid axes, missing ports,
  incompatible port types, unrooted mate graphs, and conflicting redundant mates.

## 2. Shared Frame And Mate Solver

- [x] 2.1 Add focused unit tests for finite normalized right-handed `PortFrame`
  validation, `yAxis = zAxis × xAxis`, and source-context error payloads.
- [x] 2.2 Add focused unit tests for
  `targetFrame * modifiers * inverse(sourceFrame)` covering aligned/opposed
  normals, target-local offset, and roll.
- [x] 2.3 Add focused unit tests proving local `x`/`y` reflection is applied
  before rigid placement and final `placementFrame` remains right-handed.
- [x] 2.4 Extract pure frame validation, compatibility, clearance, graph, and
  transform solving from `commands/component_package.rs` into shared
  `component_placement.rs`; keep installed-package assembly tests green.
- [x] 2.5 Implement deterministic rooted graph traversal, redundant-mate
  agreement checks, cycle handling, and exact conflict/underconstraint errors.

## 3. Source Surface And Roundtrip

- [x] 3.1 Add parser red tests for `ports`, `port`, `frame`, `port-ref`, and
  `place-component` with explicit normal mode and optional roll/offset/mirror.
- [x] 3.2 Implement interface AST nodes and literal stable component/port ids;
  allow parameter expressions only in frame coordinates and named metadata.
- [x] 3.3 Add emitter roundtrip tests, then preserve authored interface/mate
  spelling and ordering without rewriting it to raw transforms.
- [x] 3.4 Add lexical-scope tests proving geometry and port expressions receive
  identical signature defaults/overrides across expanded-AST and Steel compile
  paths.
- [x] 3.5 Add compile-time failures for duplicate ids, unknown references,
  dynamic ids/types, non-finite axes, non-orthogonal frames, and anonymous
  fit-critical offsets.

## 4. Inline Instance Expansion

- [x] 4.1 Add a failing compiler test that builds evaluated local port templates
  and solves the inline instance/mate relation before Core IR planning.
- [x] 4.2 Resolve component-source and output-part port paths by stable ids, and
  retain deterministic recursive component-reference cycle detection.
- [x] 4.3 Lower solved instances into existing `mirror`/`place` Core nodes while
  retaining the authored `place-component` call as source anchor; do not change
  public geometry Core IR structs.
- [x] 4.4 Prove through the shared graph solver and installed-package compiler
  path that multiple consistent mates validate one transform and inconsistent
  mates fail rather than average or select by declaration order.
- [x] 4.5 Prove component parameter changes update both local geometry and port
  frame values before placement, with resolved values included in cache identity.

## 5. Placement Evidence And Diagnostics

- [x] 5.1 Define Rust `ComponentPlacementEvidence` contracts with snake_case
  fields and `#[serde(rename_all = "camelCase")]`; generate TypeScript contracts.
- [x] 5.2 Persist instance/component ids, port references, solved frame,
  normal/roll/offset/mirror modifiers, and mate status in ArtifactBundle and
  ModelManifest.
- [x] 5.3 Expose compact placement evidence through target/MCP inspection and add
  tests proving agents can explain a 90-degree placement without reading mesh
  coordinates.
- [x] 5.4 Include part, instance, ports, source span, and resolved frame/fit values
  in every placement failure; preserve raw backend/runtime detail.

## 6. Backend, Preview, And Export Parity

- [x] 6.1 Add native OCCT versus portable Core-planner parity tests for the front/side latch
  fixture: placed bounds, port origins, axis directions, component count, and
  manifold status.
- [x] 6.2 Add FreeCAD and mesh-preview coverage proving solved placement uses the
  same ordinary transform semantics.
- [x] 6.3 Add STEP, multipart STL, and 3MF tests proving manufacturing exports
  bake solved placement exactly once.
- [x] 6.4 Add exploded-view tests proving view offsets compose after solved
  placement and do not change STL/STEP content digests.
- [x] 6.5 Add mirrored-latch export tests for winding, normals, manifold output,
  and right-handed manifest frames.

## 7. Extraction And Component Packages

- [x] 7.1 Add extraction red tests for a closed parameterized local port, then
  preserve its id/type/frame/fit metadata in copy-inline source and header.
- [x] 7.2 Add extraction failure coverage for ports depending on unresolved
  parent/world bindings; never freeze evaluated world coordinates.
- [x] 7.3 Extend compact component search/header responses with port ids and
  compatibility types without returning body source.
- [x] 7.4 Prove an extracted inline component and the installed package form solve
  byte-equivalent placement evidence through the shared solver.
- [x] 7.5 Add one Rust intent owning copy-inline AST patch, render, immutable
  version, runtime/manifest, bound source, and snapshot persistence.
- [x] 7.6 Add Rust BDD coverage for success, stale digest, raw render failure,
  and camelCase boundary.
- [x] 7.7 Wire LibraryPanel to the applied import intent and remove frontend
  replacement-source/manual-preview chain.

## 8. Canonical Guidance And Tooling

- [x] 8.1 Update the canonical language-surface manifest and generated Ecky,
  native, build123d, and FreeCAD prompt/reference outputs with the local-frame and
  mate pattern.
- [x] 8.2 Update MCP authoring guidance: component bodies stay local; placement
  belongs at `place-component`; raw Euler rotations remain available but are not
  the reusable-component default.
- [x] 8.3 Add an authoring example that moves the dryer latch from front to side
  by changing only `(port-ref enclosure-body ...)`, including mirrored opposite
  side and named clearance.
- [x] 8.4 Add language-reference checks preventing stale generated docs or prompts
  from omitting component ports/mates.

## 9. Final Proof

- [x] 9.1 Run focused compiler, placement solver, package assembly, extraction,
  backend parity, manifest, and export test filters.
- [x] 9.2 Run `cargo check` from `src-tauri` and relevant frontend contract unit
  tests.
- [x] 9.3 Render the dryer latch front/side fixture through the normal app runtime,
  inspect placement evidence, and verify manifold output plus one intentional
  invalid-mate failure state.
- [x] 9.4 Run `openspec validate component-local-placement-frames --strict` and
  record final evidence in task notes before implementation completion.

## Final evidence

- Shared placement solver: 9 focused tests passed, including right-handed
  frames, semantic normals, mirror, graph cycles/conflicts, and underconstraint.
- Compiler component surface: 31 focused tests passed; portable planner fixture
  passed with exactly one `place` per placed part.
- Normal app mesh runtime: front/side/mirrored fixture rendered four closed
  manifold parts with zero winding mismatches and persisted three placement
  evidence records. Invalid target port failed before render with source span.
- Native OCC and portable plans agreed on port origins and normals. Live native
  export produced STEP plus four placed part STLs. FreeCAD lowering used the
  same solved `place` semantics without Euler calls.
- Installed-package 3MF and multipart STL export placement tests passed.
  Exploded-view regression preserved identical manufacturing STL bytes.
- Package graph, inline/package parity, extraction, compact search, MCP shape
  graph, camelCase contracts, generated docs/prompts/skill, frontend lexer, and
  `cargo check` passed.
- `openspec validate component-local-placement-frames --strict` passed on
  2026-08-23.
