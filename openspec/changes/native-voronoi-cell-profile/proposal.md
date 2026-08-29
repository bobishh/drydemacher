# Proposal: Native Voronoi Cell Profile

## Intent

Add an exact native 2D Voronoi-cell profile primitive. Existing `voronoi2`
returns only a scalar field and `voronoi-cells` returns jittered sites; neither
can author polygonal cellular cutouts without baked point lists.

## Scope

- Add `(voronoi-cell sites index width height inset)` as a first-class Ecky CAD
  profile op.
- Compute the selected bounded cell by deterministic half-plane clipping.
- Apply a true constant inward offset to every clipping half-plane.
- Return a local profile centered on the selected site for tangent placement.
- Expand to `polygon` before Direct OCCT planning, preserving analytic BRep
  downstream through `extrude` and boolean operations.
- Prove composition with `solidify(import-stl(...))`; mixed analytic/faceted
  poly-BRep booleans remain inside the same Direct OCCT pipeline.
- Reject unsupported interop backends explicitly; no mesh fallback.

## Out of scope

- 3D Voronoi volumes.
- Weighted/power diagrams.
- Automatic wrapping onto cylinders; authors place profiles with normal frames.
