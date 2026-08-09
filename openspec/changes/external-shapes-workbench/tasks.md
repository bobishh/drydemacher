# Tasks: External Shapes Workbench

## 1. Product Shell

- [x] 1.1 Add failing Playwright coverage for `EXT` navigation and ordered steps.
- [x] 1.2 Rename Capture window to External Shapes without adding another Dock item.
- [x] 1.3 Keep capture viewfinder/session UI scoped to Capture step.
- [x] 1.4 Keep major containers bounded with `overflow: hidden` and existing theme.

## 2. Pairing UX

- [x] 2.1 Add failing pending/failure tests with trust setup collapsed by default.
- [x] 2.2 Move certificate QR, trust URL, and Settings path under on-demand disclosure.
- [x] 2.3 Keep current pairing QR/action visible for active pairing token.

## 3. Plane Crop Contract

- [x] 3.1 Add failing compiler/planner tests for one and two `clip-plane` nodes.
- [x] 3.2 Add `clip-plane shape :origin point3 :normal point3 :keep text` to Core IR signatures.
- [x] 3.3 Lower `clip-plane` to direct OCCT and reject zero normals/empty results exactly.
- [x] 3.4 Mark operation as BRep boundary for hybrid imported meshes.

## 3A. Bound Imports

- [x] 3A.1 Add failing Playwright coverage for automatic Rocksteady STL discovery.
- [x] 3A.2 Extract `import-stl` nodes from bound backend source with resolved path and digest.
- [x] 3A.3 Auto-select a single import and render its raw STL without solidify/OCCT.
- [x] 3A.4 Preserve selected source across Import, Crop, and Guides.
- [x] 3A.5 Report missing bound files without substituting capture geometry.

## 4. Three-Click Authoring

- [x] 4.1 Add failing Viewer test for three source-coordinate mesh anchors.
- [x] 4.2 Add Crop-step `CUT PLANE` mode with count, Undo, Flip, Apply, Cancel.
- [x] 4.3 Hide box crop while plane picker is active.
- [x] 4.4 Persist plane evidence with source mesh digest and derive plane in backend.
- [x] 4.5 Apply through backend AST/source binding so code, diff, renderer, and file agree.
- [x] 4.6 Support second plane by appending another ordinary `clip-plane` node.
- [x] 4.7 Expose each applied cut with Edit and Remove controls.
- [x] 4.8 Replace/remove the selected nested `clip-plane` through exact AST paths.
- [x] 4.9 Label kept side Above/Below Plane and show the positive-normal arrow.

## 5. Verification

- [x] 5.1 Run strict OpenSpec validation.
- [x] 5.2 Run happy plus pairing/trust failure Playwright states.
- [x] 5.3 Run compiler/planner/native crop tests.
- [x] 5.4 Run frontend unit/type checks and `cd src-tauri && cargo check`.
- [ ] 5.5 Apply tilted neck crop to Rocksteady donor and inspect preview.
