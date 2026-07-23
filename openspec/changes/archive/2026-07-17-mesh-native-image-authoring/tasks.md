## 1. Mesh Literal Outer Loop

- [x] 1.1 Add failing backend integration test: valid `.ecky` tetrahedron
  `polyhedron` renders one preview STL with one component and zero non-manifold
  edges.
- [x] 1.2 Confirm failure is unknown/unsupported mesh form before production
  changes.
- [x] 1.3 Add smallest failing compiler tests for `mesh`/`polyhedron` keyword,
  point3-list, integer-triple, and stable source-span behavior.
- [x] 1.4 Add surface exports, compiler lowering, typed Core mesh-literal node,
  Core operation identity, language reference, and backend capability entries.
- [x] 1.5 Re-run compiler tests, then integration test; keep integration red only
  on missing runtime execution.

## 2. Mesh Validation And Runtime

- [x] 2.1 Add failing unit tests for out-of-range indices, repeated indices,
  zero-area triangles, duplicate triangles, boundary edges, non-manifold edges,
  inconsistent winding, multiple components, and zero signed volume.
- [x] 2.2 Implement bounded mesh-list evaluation and static validation with raw
  operation/part/count diagnostics.
- [x] 2.3 Implement linear topology analysis and distinct `mesh` versus
  `polyhedron` closure rules.
- [x] 2.4 Add failing mesh-runtime unit test converting validated Core mesh
  literal into `IrMesh` without source expansion or silent welding.
- [x] 2.5 Implement Rust mesh execution, mesh-safe transforms/grouping, digest,
  and topology evidence wiring.
- [x] 2.6 Re-run tetrahedron integration test to green; refactor under passing
  compiler/runtime coverage.

## 3. Mesh Failure Outer Loop

- [x] 3.1 Add failing integration test: open `polyhedron` rejects before render,
  names boundary-edge count, and preserves last good artifact.
- [x] 3.2 Wire topology error through structured authoring/render error surface
  and structural verification.
- [x] 3.3 Add equivalent open `mesh` integration case: preview succeeds while
  printability evidence remains red.
- [x] 3.4 Add budget-exceeded integration case proving observed/allowed counts
  and no oversized allocation/render.

## 4. Dispatch, Hybrid Bridge, And Export

- [x] 4.1 Add failing render-dispatch integration tests for PureMesh mesh literal,
  Hybrid closed polyhedron plus exact cutter, and rejected open mesh plus BRep
  consumer.
- [x] 4.2 Classify mesh literal as mesh-boundary input and reuse existing
  partition slicing and `import-stl -> solidify` bridge.
- [x] 4.3 Add live hybrid proof: closed polyhedron boolean produces valid STL and
  STEP with faceted poly-BRep provenance and current non-manifold gate passing.
- [x] 4.4 Add failing artifact/export tests: pure mesh exposes STL/3MF where
  applicable and suppresses STEP; solidified mesh exposes STEP only with
  faceted provenance.
- [x] 4.5 Implement manifest/bundle/export-option fields and regenerate camelCase
  TypeScript contracts.
- [x] 4.6 Add MCP artifact assertions preventing analytic/exact STEP claims for
  polyhedral output.

## 5. Heightfield Outer Loop

- [x] 5.1 Add failing Playwright test: image-driven model with no selected image
  shows pending state and sends no render request.
- [x] 5.2 Add failing backend integration test: fixture grayscale image plus
  dimensions produces deterministic closed STL with zero non-manifold edges.
- [x] 5.3 Add smallest compiler tests for `heightfield` arguments, physical
  bounds, image parameter dependency, and stable AST/source mapping.
- [x] 5.4 Implement `heightfield` surface/Core operation and reuse extracted
  lithophane/displacement image sampling, refinement, and safe mesh writer
  helpers.
- [x] 5.5 Add invalid-dimension and corrupt-image tests; prove raw backend error
  display and last-good-preview retention.
- [x] 5.6 Run Playwright happy path: select image, apply, render relief, inspect
  topology evidence, export STL.

## 6. Orthographic Raster Trace Outer Loop

- [x] 6.1 Add failing Playwright happy path: calibrated Front/Top/Side line-art
  images yield selectable contours, reviewed `SketchDocument`, and existing
  preview-hull request.
- [x] 6.2 Add failing Playwright failure path: noisy image yields no closed
  contour, raw extraction evidence, pending reconstruction, and preserved last
  reviewed sketch.
- [x] 6.3 Add failing Rust unit tests for deterministic thresholding, contour
  connectedness, simplification, closure checks, calibration, and bounded image
  size/pixel count.
- [x] 6.4 Implement Rust raster trace request/response contracts using
  `#[serde(rename_all = "camelCase")]`; regenerate frontend bindings.
- [x] 6.5 Implement per-view reference/calibration/extraction controls and
  overlays inside existing Sketch Workspace theme/layout boundaries.
- [x] 6.6 Convert selected contours into raster-provenance sketch primitives;
  route only reviewed document through existing candidate preview/search.
- [x] 6.7 Prove accepted-CAD gates unchanged: preview hull stays pending; matching
  exact STEP plus hidden-line projection can pass with image provenance.

## 7. Stable Draft Persistence

- [x] 7.1 Add failing Playwright test: re-extraction updates same draft without
  new thread/version.
- [x] 7.2 Add failing backend/frontend restore tests for asset identity,
  calibration, extraction settings, contour id, extractor version, and edited
  primitive provenance.
- [x] 7.3 Extend existing sketch draft command contracts; persist through backend
  commands only, never direct SQLite writes.
- [x] 7.4 Prove discard removes draft reference state without deleting user asset
  files outside existing ownership rules.

## 8. Agent, MCP, And Reference Truth

- [x] 8.1 Add failing language-manifest tests covering mesh, polyhedron, and
  heightfield syntax/support/export notes.
- [x] 8.2 Update generation and MCP authoring guidance: use formula-generated
  lists, require topology verifies, treat vision source as inferred, inspect
  artifact truth before STEP claims.
- [x] 8.3 Add MCP inspect -> AST validate -> preview -> verify -> commit smoke
  test for one bounded parametric polyhedron.
- [x] 8.4 Add vision-reference test proving response text cannot mark one-photo
  inference as accepted CAD without exact artifact evidence.

## 9. Regression And Closure

- [x] 9.1 Run targeted Rust compiler, mesh runtime, topology, render, hybrid,
  heightfield, sketch reconstruction, MCP, and contract tests.
- [x] 9.2 Run `cd src-tauri && cargo check`.
- [x] 9.3 Run `npm run test:unit`.
- [x] 9.4 Run targeted Playwright image pending/happy/failure, raster trace, draft
  persistence, and accepted-CAD flows on a non-conflicting local port.
- [x] 9.5 Run existing pure OCCT, pure mesh, displacement/lithophane, sketch
  preview, export, and MCP regression suites.
- [x] 9.6 Update Direct OCCT coverage matrix and language field guide only after
  matching runtime tests pass.
- [x] 9.7 Run `openspec validate mesh-native-image-authoring --strict` and resolve
  all proposal/spec/design/task consistency failures.
