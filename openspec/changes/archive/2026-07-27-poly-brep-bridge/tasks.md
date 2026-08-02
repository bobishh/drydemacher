# Tasks: Poly BRep Bridge

## Worker Rules

- Each workstream has a disjoint write scope.
- Workers must not break existing pure-OCCT or pure-mesh render paths.
- Workers must not remove build123d/FreeCAD lowering.
- Workers must list changed files and tests run.
- BDD: write a failing test first, then implement.

## Status Legend

- [x] Done and tested
- TODO — unchecked task

---

## T1 — Partition Analysis ✅

Write scope:

- `src-tauri/src/ecky_ir/poly_partition.rs`
- `src-tauri/src/ecky_ir/mod.rs`

Tasks:

- [x] 1.1 `PartRenderStrategy` enum: PureOcct | PureMesh | Hybrid.
- [x] 1.2 `PartPartition` struct: strategy, boundary_node_ids,
  has_post_boundary_brep_op.
- [x] 1.3 `analyze_program(program) -> Vec<PartPartition>`: walks CoreNode
  trees bottom-up.
- [x] 1.4 Mesh-only op detection via `is_ecky_rust_only_cad_head`.
- [x] 1.5 BRep-required op detection (boolean, chamfer, fillet, shell,
  offset).
- [x] 1.6 Post-boundary propagation: a BRep-required op is Hybrid only if
  its input is transitively post-boundary.
- [x] 1.7 Tests (13): PureOcct, PureMesh (wall-pattern alone/then
  translate/scale/chained), Hybrid (wall-pattern + difference/chamfer/fillet/
  union), cutter-branch wall-pattern, multi-part independent classification,
  boundary node IDs.

**Completed:** Commit `8b79e4b`. 13 BDD tests.

---

## T2 — Poly BRep Bridge (import-stl + solidify) ✅

Write scope:

- `src-tauri/src/ecky_cad_host/direct_occt.rs`
- `src-tauri/src/ecky_cad_host/direct_occt_executor.rs`
- `src-tauri/src/ecky_cad_host/direct_occt_normalize.rs`
- `src-tauri/src/ecky_cad_host/direct_occt_runner.rs`

Tasks:

- [x] 2.1 Add `OcctOp::Solidify` to enum.
- [x] 2.2 Add `Solidify` to executor emission (BRepBuilderAPI_Sewing +
  BRepBuilderAPI_MakeSolid + volume orientation).
- [x] 2.3 Add `Solidify` to normalizer allow-list.
- [x] 2.4 Add `Solidify` to runner op token + runner_op_supported (if runner
  should handle it; otherwise it falls to executor compile path).
- [x] 2.5 Add `solidify` to CoreOperation::Custom → OcctOp planning.
- [x] 2.6 Fix latent `.c_str()` bug in `emit_import_stl_operation` (string
  literal is already `const char*`).
- [x] 2.7 Empirical boolean proof (BDD):
  - Fixture: VertexGenie (Ecky mascot seed 1) — 120 organic displaced
    triangles, 0 non-manifold.
  - Test: `solidify(import-stl(genie))` → `difference(cylinder)` → assert
    0 non-manifold, volume reduced, 1 component. **PASSED.**
  - Diagnostic: import-stl round-trip preserves topology (120→120, 0 NM).
  - Diagnostic: import-stl WITHOUT solidify → 73 non-manifold edges
    (proves solidify is necessary).

**Completed:** Commit `d69974d`. Fixture at
`src-tauri/tests/fixtures/vertex-genie-ecky.stl`.

---

## T3 — Hybrid Dispatch Wiring ✅

Write scope:

- `src-tauri/src/services/render.rs`
- possibly `src-tauri/src/ecky_cad_host/direct_occt.rs` (plan reconstruction)
- targeted tests

Tasks:

- [x] 3.1 In `render_model_unlocked`, after backend resolution for Ecky IR
  sources: compile to CoreProgram, run `poly_partition::analyze_program`.
  Classify per-part strategy.
- [x] 3.2 If ALL parts PureOcct → existing OCCT path (no change, regression
  guard test).
- [x] 3.3 If ALL parts PureMesh → existing mesh path (no change, regression
  guard test).
- [x] 3.4 If ANY part Hybrid → hybrid pipeline:
  a. Render each independent mesh island through the mesh renderer.
  b. Store each displaced STL as an engine-independent `MeshAsset`.
  c. Construct OCCT plan: `import-stl(stl) → solidify → [post-boundary ops]`.
  d. Execute OCCT plan → STL + STEP.
- [x] 3.5 Post-boundary op reconstruction: tree-slicing in `poly_partition.rs`
  (`clone_program_for_mesh_phase` + `clone_program_for_occt_phase`). Mesh
  phase strips post-boundary ops; OCCT phase replaces wall-pattern with
  `solidify(import-stl(path))`.
- [x] 3.6 Integration test: wall-pattern + difference → manifold result
  (< 100 non-manifold edges). **PASSING.** PG1.
- [x] 3.7 Integration test: iPhone 17e case fixture → 3 clean parts. PG2.
  Exact chamfer runs before the mesh boundary, cellular displacement runs on
  its tessellated result, and final OCCT booleans preserve the rear relief.
  The test is active and enforces the `< 100` non-manifold threshold.
- [x] 3.8 Regression test: PureOcct model still routes to OCCT (no
  partition overhead beyond the analyze call). PG5/PG8.
- [x] 3.9 Regression test: PureMesh model still routes to mesh renderer. PG6/PG9.

---

## T4 — Export and Manifest ✅

Write scope:

- `src-tauri/src/model_runtime.rs`
- `src-tauri/src/ecky_cad_host/direct_occt_runtime.rs`

Tasks:

- [x] 4.1 Hybrid part export: STL from final OCCT tessellation.
- [x] 4.2 Hybrid part export: STEP (exact faces exact, poly faces poly).
- [x] 4.3 Tag hybrid parts in manifest so UI/export knows they used the
  bridge (manifest warning: "Hybrid poly BRep bridge: N part(s)...")
- [x] 4.4 Tests: hybrid part → STL + STEP exist and are valid.
  (`hybrid_poly_brep_exports_both_stl_and_step`)

---

## T5 — Documentation and Integration Gates

Write scope:

- docs
- final regression sweep

Tasks:

- [x] 5.1 `cd src-tauri && cargo check` passes.
- [x] 5.2 Full backend suite passes (1625 library + 93 integration passed,
  0 failures, 2 ignored parity cases).
- [x] 5.3 Document hybrid pipeline in coverage matrix.
- [x] 5.4 Update coverage matrix with hybrid classification.

---

## Completed Extensions (from original spec)

### T2 (original): Pre-boundary OCCT Tessellation

**Superseded by T7.** Exact BRep may be tessellated when it intentionally
crosses into a mesh-only operation input, but `chamfer`/`fillet` are not mesh
preludes and must not be pushed into the mesh phase.

### T3 (original): Mesh-Only Op Execution on Tessellated Mesh

**Done.** `wall-pattern` accepts evaluated mesh inputs, including OCCT
tessellation, imported STL, and typed `polyhedron` output.

### T6 (original): MeshAsset Interface

**Done.** `MeshAsset` is the engine-independent handoff. Provenance supports
internal mesh phases, imported STL, and generated providers such as Meshy.
Typed `polyhedron` gives an LLM-native triangle mesh input without binding the
pipeline to a generator SDK.

### ~~`import_poly_mesh` combined op~~

**Removed.** The original design described a combined `import_poly_mesh`
OcctPlan command with a `meshPath` field. We correctly decomposed it into
`import-stl` + `solidify` — two composable ops. No combined op needed.

---

## Proof Gates

- [x] PG-PROOF OCCT boolean on solidified poly BRep works (VertexGenie proof).
- [x] PG1 wall-pattern + difference → < 100 non-manifold edges.
- [x] PG2 iPhone 17e case → 3 clean parts with displaced rear panel retained.
- [x] PG3 STEP includes exact faces for non-displaced geometry.
- [x] PG4 STL is sliceable without geometry repair. Bambu Studio completed with
  `Slice ok`. A thin authored wall beside the side-button opening fell below
  configured line width; that product-model thickness issue is outside bridge
  correctness.
- [x] PG5 Pure OCCT model → still routes to OCCT.
- [x] PG6 Pure mesh model → still routes to mesh renderer.
- [x] PG7 `cargo check` passes.
- [x] PG8 Existing direct-OCCT fixtures still render.
- [x] PG9 Existing mesh-renderer fixtures still render.

---

## T6 — Surface Operation Route Authority ✅

Write scope:

- `src-tauri/src/ecky_ir/poly_partition.rs`
- `src-tauri/src/ecky_ir/mesh_ops.rs`
- `src-tauri/src/ecky_cad_host/direct_occt_runtime.rs`
- `src-tauri/src/mcp/handlers/mod.rs`
- `src/lib/SketchWorkspace.svelte`
- targeted tests

Tasks:

- [x] 6.1 Add partition tests proving `chamfer`/`fillet` stop mesh
  phase: `wall-pattern -> chamfer` must output the `wall-pattern` node to the
  mesh phase and apply `chamfer` in the OCCT phase.
- [x] 6.2 Remove/supersede `push_bound_edge_op_before_mesh` behavior for
  `chamfer`/`fillet`; no polygon edge-op pushdown for BRep surface ops.
- [x] 6.3 Add mesh evaluator guard: analytic-origin `chamfer`/`fillet` must
  reject with an exact-route diagnostic instead of silently producing polygon
  output. Mesh-origin usage remains explicit and marks `meshNative`.
- [x] 6.4 Direct OCCT pure renders set `GeometryRepresentation::AnalyticBrep`
  on `ArtifactBundle`, `ModelManifest`, and STEP `ExportArtifact`.
- [x] 6.5 MCP artifact digest reports `geometryRepresentation=analyticBrep`,
  `analyticStep=true`, `facetedStep=false` for pure Direct OCCT artifacts.
- [x] 6.6 UI artifact evidence displays representation truth separately from
  `geometryBackend`.
- [x] 6.7 Add regression test: `(chamfer 1 (box 20 20 10))` under EckyRust
  produces STEP + `analyticBrep` and never enters mesh chamfer.
- [x] 6.8 Add regression test: `wall-pattern -> chamfer` produces hybrid
  `solidify(import-stl(...)) -> chamfer` OCCT plan and marks faceted output.
- [x] 6.9 Add mesh-origin surface-op admission: bounded selector/edge-count
  gate for `chamfer`/`fillet` on solidified faceted BRep; reject dense broad
  selectors with selected-edge count and no fallback.
- [x] 6.10 Run `openspec validate poly-brep-bridge --strict`.
- [x] 6.11 Run `cd src-tauri && cargo check` plus targeted Rust tests.
- [x] Do not enable glue globally or change fuzzy tolerance without a named
  tolerance policy and regression fixture.
