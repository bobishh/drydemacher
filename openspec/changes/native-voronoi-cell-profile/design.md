# Design: Native Voronoi Cell Profile

## Variables

- **Goal:** exact angular Voronoi apertures usable as tangent BRep cutters.
- **Artifact model:** one closed convex 2D sketch per selected site.
- **Variables:** site order, bounds, inset, duplicate sites, degeneracy, units,
  backend support, vertex ordering, compiler value kind.
- **Decision:** centered rectangular bounds; zero-based integral index; constant
  half-plane inset; CCW deterministic polygon rotated to a stable first vertex;
  native-only initial backend.
- **Rejected paths:** scalar `voronoi2`; radius-jittered regular polygons; baked
  profiles; scale-about-site inset; mesh wall pattern.
- **Proof plan:** compiler red/green; clipping unit tests; Direct OCCT expansion
  test; native render fixture; real bottle-cage preview then FEM.

## Surface

```scheme
(voronoi-cell sites index width height inset)
```

`sites` is a finite point2 list. `index` is zero-based. `width`, `height`, and
`inset` use model length units. Output coordinates are relative to the selected
site, so `(place tangent-plane (extrude ...))` remains straightforward.

## Algorithm

Start with the centered bounds inset by `inset`. For every other site `j`, clip
against the selected-site half-plane:

`2 (site_j - site_i) dot p <= |site_j|^2 - |site_i|^2 - 2 inset |site_j-site_i|`

This shifts each Voronoi edge inward by exactly `inset`. Sutherland-Hodgman
clipping preserves convexity and ordering. Reject duplicate sites, empty cells,
fewer than three vertices, and negligible signed area.

## Backend boundary

Direct OCCT expands the op to the existing `polygon` primitive before planning.
Build123d and FreeCAD reject with a native-only diagnostic until exact parity is
implemented. No silent approximation or triangulated fallback.

This boundary does not disable hybrid BRep. An imported closed STL may still
flow through `solidify(import-stl(...))` into faceted poly-BRep and participate
in a Direct OCCT boolean with an analytic extruded Voronoi profile. Artifact
provenance remains faceted whenever a mesh-origin operand contributes.
