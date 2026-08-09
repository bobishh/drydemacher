# Proposal: External Shapes Workbench

## Intent

Replace the narrowly named Capture window with one task-owned workspace for
bringing external geometry into canonical Ecky authoring. Phone capture remains
one acquisition step. Imported STL/OBJ/3MF, cropping, scan evidence, parametric
reconstruction, and validation share the same source-identity boundary.

The current pairing screen consumes the whole viewport and permanently exposes
certificate-install instructions. Those instructions matter during first trust
setup, not during every capture. The Capture view shall remain available only
inside the Capture step, while certificate details become an explicit on-demand
disclosure.

## Scope

- Rename the workbench window to `EXTERNAL SHAPES`; Dock label `EXT`, accessible
  name and tooltip `Work with external shapes`.
- Add ordered steps: Import, Capture, Crop, Guides, Reconstruct, Validate.
- Keep current phone viewfinder/session UI inside Capture only.
- Collapse certificate installation and trust instructions by default.
- Support imported meshes and captured meshes through one immutable source-mesh
  identity contract.
- Add one composable `clip-plane` Ecky operation. Two-plane crop is two nested
  operations.
- Add three-click scan-plane authoring. Viewer raycasts source mesh anchors;
  backend derives the plane; Apply patches canonical `model.ecky`.
- Disable box-crop overlay while plane picking or guided reconstruction is active.

## Out Of Scope

- Hidden mesh mutations not represented by source or guide artifacts.
- Permanent certificate instructions in the main capture viewport.
- A second model/history authority for imported shapes.
- Free-form sculpting or destructive editing of raw source files.

## Proof Gates

- Capture viewport appears in Capture step and not in Import/Crop/Guides steps.
- Pairing starts with certificate details hidden; disclosure reveals exact trust
  URL and instructions; camera/pairing error remains visible.
- Three non-collinear clicks produce a preview plane and explicit kept side.
- Collinear points reject Apply with raw reason.
- Applied crop exists as `clip-plane` in bound `model.ecky`.
- Two planes compose without a separate hidden crop representation.
- Browser happy and failure states pass; OpenSpec strict validation and
  `cd src-tauri && cargo check` pass.
