# Design: External Shapes Workbench

## Goal

One bounded workflow for external meshes, from acquisition/import through
source-backed cropping and evidence-guided BRep reconstruction.

## Information Architecture

```text
EXTERNAL SHAPES
  IMPORT       choose existing external mesh
  CAPTURE      pair phone, acquire photos, reconstruct source mesh
  CROP         box crop or one/two source-backed planes
  GUIDES       landmarks, axes, profiles, symmetry/reference planes
  RECONSTRUCT  bounded evidence handoff and canonical .ecky authoring
  VALIDATE     overlay, correspondences, observed-region deviation
```

The large camera/pairing viewport belongs only to Capture. Other steps reuse the
mesh Viewer with their own bounded controls.

## Canonical Plane Crop

```lisp
(clip-plane donor
  :origin (12.4 8.1 94.7)
  :normal (0.18 -0.06 0.98)
  :keep "positive")
```

Two planes are ordinary composition:

```lisp
(clip-plane
  (clip-plane donor :origin p1 :normal n1 :keep "positive")
  :origin p2 :normal n2 :keep "negative")
```

Three source-mesh clicks create digest-bound barycentric anchors. Backend
rejects duplicate/collinear/non-finite evidence and derives normalized origin,
normal, and stable kept-side preview. `FLIP` reverses normal/keep. Frontend does
not become coordinate authority and does not write source directly; Apply uses
the backend-owned source binding/AST patch path.

## Pairing Disclosure

Capture initially shows pairing state and primary action. Certificate QR, trust
URL, and Settings path live under `PHONE TRUST SETUP`. The disclosure may open
automatically only after an actionable TLS/trust failure. Pairing QR remains
visible when a live token exists because it is the current action, not permanent
setup documentation.

## Ownership

- Backend: source mesh identity, plane validation, crop execution, source/AST
  persistence, history guards.
- Viewer: raycast hits, point handles, preview-only plane and kept-side overlay.
- External Shapes shell: step navigation and scoped state presentation.
- Capture step: phone session and reconstruction acquisition only.
- `.ecky`: canonical applied geometry operations.
- Guide artifact: evidence anchors and fit/validation provenance.

Applied crop and trim edits enter one Rust intent boundary. Rust loads bound
source and selected version context, checks expected source/mesh digests, applies
the exact AST patch, renders and appends one immutable success/error version,
updates bound source/runtime/manifest/snapshot through manual authoring service,
then rereads canonical external sources. Frontend submits intent and projects
returned version/runtime/source state only.

## Rejected Paths

- Rotated giant boxes as authored plane cuts: obscures intent and weakens bounds.
- UI-only plane crop: disappears from code and cannot be reproduced by agent.
- Certificate wall as default view: setup detail dominates routine capture.
- Separate Import and Capture histories: both are external mesh acquisition.

## Proof Plan

- Playwright checks step-scoped view, collapsed/expanded trust setup, pairing
  pending state, and raw failure state.
- Compiler/planner test checks one and two `clip-plane` operations.
- Native OCCT test checks kept half and empty/collinear failure diagnostics.
- Viewer test checks three source anchors, Flip, and box-overlay suppression.
- Live Rocksteady crop checks tilted neck plane and bounded Viewer topology.
