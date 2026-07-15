# Tasks: Poly BRep Bridge

## Worker Rules

- Each workstream has a disjoint write scope.
- Workers must not break existing pure-OCCT or pure-mesh render paths.
- Workers must not remove build123d/FreeCAD lowering.
- Workers must list changed files and tests run.
- BDD: write a failing test first, then implement.

## 1. T1 - Partition Analysis

Write scope:

- new `src-tauri/src/ecky_cad_host/poly_partition.rs`
- `src-tauri/src/ecky_cad_host/mod.rs`
- targeted tests

Tasks:

- [ ] 1.1 Add `PartPartition` data model: boundary node id, pre-boundary
  sub-tree, mesh-only op chain, post-boundary op chain.
- [ ] 1.2 Implement `analyze_part_partition(node: &CoreNode) -> PartPartition`
  that walks the CoreNode tree and finds the first mesh-only op
  (`ECKY_RUST_ONLY_CAD_OPS`).
- [ ] 1.3 Handle multiple mesh-only ops in sequence (chain them in mesh
  phase).
- [ ] 1.4 Handle mesh-only ops in different sub-branches (multiple boundaries
  per part → multiple poly shells, fused in T5).
- [ ] 1.5 Classify post-boundary ops as BRep-required (boolean, chamfer,
  fillet) vs mesh-safe (translate, rotate, scale). If all post-boundary ops
  are mesh-safe → short-circuit flag (skip T4/T5).
- [ ] 1.6 Add tests:
  - Pure OCCT part (no mesh ops) → no boundary.
  - Pure mesh part (no post-boundary BRep ops) → short-circuit flag.
  - wall-pattern + difference → boundary found, post-boundary BRep required.
  - Multiple mesh-only ops in sequence → chained.
  - Mesh-only op in cutter sub-branch (not main body) → handled.

## 2. T2 - Pre-boundary OCCT Tessellation

Write scope:

- `src-tauri/src/ecky_cad_host/direct_occt_runtime.rs`
- `src-tauri/src/ecky_cad_host/direct_occt_runner.rs`
- targeted tests

Tasks:

- [ ] 2.1 Add `tessellate_shape_to_mesh` that takes an OCCT shape (from
  pre-boundary plan execution) and produces a `csgrs::Mesh` at controlled
  tessellation density.
- [ ] 2.2 Serialize tessellated mesh to a temp file (STL binary) for handoff
  to mesh renderer.
- [ ] 2.3 Add tests:
  - Extrude sub-tree → tessellate → valid mesh (non-empty, manifold).
  - Tessellation density parameter controls triangle count.

## 3. T3 - Mesh-Only Op Execution on Tessellated Mesh

Write scope:

- `src-tauri/src/ecky_ir/mesh_ops.rs`
- `src-tauri/src/ecky_ir/runtime.rs`
- targeted tests

Tasks:

- [ ] 3.1 Add `render_mesh_ops_on_mesh(source_sub_tree, input_mesh, params)`
  that evaluates the mesh-only op chain (wall-pattern, etc.) on a pre-built
  mesh instead of evaluating the full part from scratch.
- [ ] 3.2 Ensure `wall-pattern` accepts an externally provided base mesh
  (currently it builds its own from the target node).
- [ ] 3.3 Add tests:
  - wall-pattern on externally provided extrude mesh → displaced mesh.
  - Chained wall-pattern ops on provided mesh.

## 4. T4 - Mesh → OCCT Poly BRep Bridge

Write scope:

- `src-tauri/native/direct_occt_runner.cpp`
- `src-tauri/src/ecky_cad_host/direct_occt_runner.rs`
- `src-tauri/src/ecky_cad_host/direct_occt_sdk.rs`
- targeted tests

Tasks:

- [ ] 4.1 Add `import_poly_mesh` OcctPlan command: reads STL/OBJ from path,
  builds OCCT polyhedral BRep solid via `BRepBuilder` + `Poly_Triangulation`.
- [ ] 4.2 Implement C++ runner handler: read mesh file → create triangulated
  shell → sew into closed solid → store in shape slot.
- [ ] 4.3 Validate mesh before wrapping: reject empty meshes, warn on
  non-manifold input (don't reject — let OCCT boolean handle or fail
  naturally).
- [ ] 4.4 Add runner parity test: import_poly_mesh produces a valid OCCT
  solid that can participate in boolean ops.
- [ ] 4.5 Add tests:
  - Simple box mesh → poly BRep solid → fuse with exact cylinder → valid.
  - Non-manifold input → graceful error, not crash.

## 5. T5 - Hybrid Boolean Dispatch

Write scope:

- `src-tauri/src/services/render.rs`
- `src-tauri/src/ecky_cad_host/direct_occt_runtime.rs`
- targeted tests

Tasks:

- [ ] 5.1 Add hybrid dispatch path in `render_model_unlocked`: when partition
  analysis returns a boundary with post-boundary BRep ops, execute the
  hybrid pipeline (T2→T3→T4→T5).
- [ ] 5.2 Wire pre-boundary OCCT execution → tessellation → mesh op → poly
  BRep import → post-boundary OCCT boolean into a single render flow.
- [ ] 5.3 Pass poly shell shape slot to subsequent OCCT boolean commands
  (cut, fuse, common).
- [ ] 5.4 Implement short-circuit: if partition analysis flags mesh-safe-only
  post-boundary ops, skip T4/T5, use mesh renderer output directly.
- [ ] 5.5 Implement short-circuit: if no boundary found, use pure OCCT path
  (existing behavior, no regression).
- [ ] 5.6 Add integration tests:
  - wall-pattern + difference → manifold result (< 100 non-manifold edges).
  - iPhone 17e case fixture → 3 clean parts.
  - Pure OCCT model → still routes to OCCT (no regression).
  - Pure mesh model → still routes to mesh renderer (no regression).

## 6. T6 - MeshAsset Interface

Write scope:

- new `src-tauri/src/ecky_cad_host/mesh_asset.rs`
- `src-tauri/src/ecky_cad_host/mod.rs`
- targeted tests

Tasks:

- [ ] 6.1 Define `MeshSource` enum: `WallPattern`, `ImportedMesh`, (future:
  `ImageRelief`, `GeneratedMesh`).
- [ ] 6.2 Define `MeshAsset` struct: source, resolved mesh, metadata (vertex
  count, triangle count, bounding box).
- [ ] 6.3 Add `resolve_mesh_asset(source, params, app) -> MeshAsset` that
  produces a triangle mesh from any `MeshSource`.
- [ ] 6.4 Wire `wall-pattern` through `MeshSource::WallPattern`.
- [ ] 6.5 Add `MeshSource::ImportedMesh` for STL/OBJ file imports.
- [ ] 6.6 Add tests:
  - WallPattern source → displaced mesh.
  - ImportedMesh source → mesh from file.
  - Unknown source → error.

## 7. T7 - Export and Manifest

Write scope:

- `src-tauri/src/ecky_cad_host/direct_occt_runtime.rs`
- `src-tauri/src/model_runtime.rs`
- targeted tests

Tasks:

- [ ] 7.1 For hybrid parts: export STL from final tessellated solid.
- [ ] 7.2 For hybrid parts: export STEP with exact faces as NURBS, poly faces
  as faceted BRep.
- [ ] 7.3 Mark poly-only parts (no exact geometry) as STL-only in manifest.
- [ ] 7.4 Add manifest field `hasPolyFaces: bool` per part for downstream
  consumers.
- [ ] 7.5 Add tests:
  - Hybrid part → STL exists, STEP exists.
  - Poly-only part → STL exists, STEP absent or tagged.
  - Exact-only part → STEP has no poly faces.

## 8. T8 - Product Integration and Gates

Write scope:

- integration edits only after T1-T7 review
- docs updates

Tasks:

- [ ] 8.1 Review worker outputs, merge non-conflicting patches.
- [ ] 8.2 Run `cd src-tauri && cargo check`.
- [ ] 8.3 Run relevant Rust tests for hybrid pipeline modules.
- [ ] 8.4 Render iPhone 17e case fixture → 3 clean parts, manifold.
- [ ] 8.5 Verify pure-OCCT and pure-mesh regression tests pass.
- [ ] 8.6 Update coverage matrix with hybrid pipeline classification.
- [ ] 8.7 Document hybrid pipeline in Ecky IR field guide.

## Proof Gates

- [ ] PG1 A model using `wall-pattern` followed by `difference` renders as a
  single manifold solid with < 100 non-manifold edges.
- [ ] PG2 The iPhone 17e case fixture renders as 3 clean parts.
- [ ] PG3 STEP export includes exact faces for non-displaced geometry.
- [ ] PG4 STL export is sliceable without manual repair.
- [ ] PG5 Models with no mesh-only ops still route through pure OCCT.
- [ ] PG6 Models with only mesh ops still route through pure mesh renderer.
- [ ] PG7 `cd src-tauri && cargo check` passes.
- [ ] PG8 Existing direct-OCCT fixtures still render (no regression).
- [ ] PG9 Existing mesh-renderer fixtures still render (no regression).
