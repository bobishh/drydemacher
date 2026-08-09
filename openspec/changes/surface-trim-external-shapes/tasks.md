# Tasks: Surface Trim for External Shapes

## 1. Outer Behavior Contract

- [x] 1.1 Add failing Playwright flow: Import source, Trace Surface, place N points,
  close loop, choose keep region, preview, Apply.
- [x] 1.2 Add failing Playwright flow for stale source digest; raw failure visible,
  canonical source unchanged.
- [x] 1.3 Add failing Playwright flow for self-intersecting/non-partitioning loop.
- [x] 1.4 Add reload/edit/remove flow proving applied state comes from `model.ecky`.
- [x] 1.5 Keep tests on real External Shapes route with box/plane overlays absent
  during Surface Trim.

## 2. Versioned Contracts and Source Anchors

- [x] 2.1 Add strict camel-case boundary DTOs for path preview, loop validation,
  apply, applied-node report, and cap report.
- [x] 2.2 Reuse source digest + triangle + barycentric anchor semantics; reject
  unknown fields and mixed schema variants.
- [x] 2.3 Reparse exact source bytes and validate triangle bounds, barycentrics,
  finite reconstructed position, triangle area, and digest.
- [x] 2.4 Add deterministic tests for digest mismatch, stale triangle, degenerate
  face, non-finite weights, and transformed Viewer coordinates.

## 3. Mesh Graph and Live Path

- [x] 3.1 Add failing synthetic-fixture tests for connected-component discovery,
  edge/face adjacency, normals, and dihedral features.
- [x] 3.2 Build digest-keyed derived graph cache with invalidation and diagnostics.
- [x] 3.3 Add failing shortest-path test with stable tie breaking.
- [x] 3.4 Add failing feature-path test where a crease is preferred over a nearby
  smooth shortcut under schema-v1 weights.
- [x] 3.5 Implement deterministic Dijkstra/A* corridor search.
- [x] 3.6 Add preview command with monotonic preview id and stale response handling.
- [x] 3.7 Prove repeat requests reuse graph cache without changing results.

## 4. Loop and Region Mechanics

- [x] 4.1 Add failing tests for ordered N-point closure and explicit last-to-first
  path.
- [x] 4.2 Reject duplicate, disconnected, self-intersecting, non-manifold, and
  non-partitioning loops with exact segment ids.
- [x] 4.3 Add keep-seed projection and barrier-constrained region traversal.
- [x] 4.4 Define and test that only seed-containing region survives, including a
  source with multiple disconnected shells.
- [x] 4.5 Add selected-region preview result without mutating source or history.

## 5. Exact Triangle Cutting

- [x] 5.1 Add failing fixture where boundary crosses triangle interiors rather
  than following existing edges.
- [x] 5.2 Refine graph corridor into continuous surface segments.
- [x] 5.3 Insert boundary vertices and split crossed edges/triangles with stable
  indexing and winding.
- [x] 5.4 Remove unselected faces and emit ordered boundary loops.
- [x] 5.5 Verify area, orientation, duplicate vertices, boundary count, and
  non-manifold edges across coarse and irregular tessellations.

## 6. Cap Modes

- [x] 6.1 Implement Open mode and prove it reports an open boundary explicitly.
- [x] 6.2 Add failing Flat tests for valid tilted loop, excessive planarity
  deviation, and self-intersecting projection.
- [x] 6.3 Implement least-squares fit, measured tolerance report, constrained cap
  triangulation, and orientation.
- [x] 6.4 Add failing Surface Fill tests for a non-planar loop and foldover case.
- [x] 6.5 Implement constrained non-planar fill or report exact failure without
  falling back to Flat.
- [x] 6.6 Verify closed outputs are watertight, oriented, and manifold.

## 7. Canonical Ecky Operation

- [x] 7.1 Add failing parser/compiler tests for `mesh-anchor` and `surface-trim`.
- [x] 7.2 Add Core IR signature and versioned constants for path/cap modes.
- [x] 7.3 Route `import-stl -> surface-trim` through indexed-mesh runtime.
- [x] 7.4 Make `surface-trim` legal inside `solidify` without routing trim through
  OCCT boolean operations.
- [x] 7.5 Add failing end-to-end runtime test for
  `solidify(surface-trim(import-stl ...))` followed by one BRep difference.
- [x] 7.6 Preserve structural verification and reject open/non-manifold trim before
  solidification.

## 8. AST Apply, Edit, Remove

- [x] 8.1 Add failing AST patch test inserting one trim around exact imported
  shape node.
- [x] 8.2 Apply only after target thread/message/source snapshot guard passes.
- [x] 8.3 Add failing nested-operation test replacing exact selected trim node.
- [x] 8.4 Add failing nested-operation test removing exact wrapper and preserving
  its shape child and unrelated operations.
- [x] 8.5 Extract applied trims from canonical source for UI reload.
- [x] 8.6 Keep failed Apply/Edit/Remove atomic: file, preview, and history unchanged.
- [x] 8.7 Reject a second schema-v1 trim on the same exact import; require Edit or
  Remove while allowing one trim on each separate imported source.

## 9. Viewer and Crop UX

- [x] 9.1 Add `TRACE SURFACE` beside `CUT PLANE`, not as another workbench step.
- [x] 9.2 Support point placement, numbered hard points, live path, Undo, Close
  Loop, Cancel, and point movement/removal.
- [x] 9.3 After closure require explicit `CLICK REGION TO KEEP`.
- [x] 9.4 Show retained-region tint and cap preview before Apply.
- [x] 9.5 Label cap choices Open, Flat, Surface Fill with tooltips and measured
  Flat suitability.
- [x] 9.6 List applied trims with point count, cap mode, Edit, and Remove.
- [x] 9.7 Disable box/plane/guided overlays and conflicting orbit/drag handlers
  during trim interaction.
- [x] 9.8 Bound major layout containers with `overflow: hidden`; preserve Tactical
  Midnight theme.
- [x] 9.9 Display raw backend failures; never render `[object Object]`.

## 10. Verification

- [x] 10.1 Run focused frontend and Rust unit tests after each inner loop.
- [x] 10.2 Run Playwright happy plus stale/invalid/cap failure states.
- [x] 10.3 Run compiler/planner/runtime integration tests.
- [x] 10.4 Run `npm run typecheck`.
- [x] 10.5 Run `cd src-tauri && cargo check`.
- [x] 10.6 Run strict OpenSpec validation.
- [x] 10.7 Browser-proof real imported STL interaction, reload, Edit, Remove, and
  later BRep boolean; capture screenshots and exact source diff.
