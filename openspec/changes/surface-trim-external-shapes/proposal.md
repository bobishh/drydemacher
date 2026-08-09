# Proposal: Surface Trim for External Shapes

## Intent

Add a general surface-following trim operation for imported and captured triangle
meshes. Plane crop remains the fast tool for planar cuts. Surface Trim handles
semantic boundaries such as a neck, flange, seam, damaged scan edge, or organic
parting line that cannot be represented by one plane or a stack of planes.

The user marks sparse points on the visible source surface. The application
connects them with a feature-aware live path, closes the contour, and asks for
one point in the region to keep. Apply writes one canonical `surface-trim` node
to bound `model.ecky`. No derived STL, Viewer selection, or database row becomes
a second geometry authority.

## Research Basis

- VTK `vtkSelectPolyData`: non-planar surface loops, robust Dijkstra edge search,
  and region choice by a user-specified point.
- 3D intelligent scissors: curvature-sensitive live paths between sparse surface
  points.
- Constrained random walks: optional keep/remove strokes and hard boundary points
  for ambiguous geometry.
- SeamCut: connectivity-independent curve refinement for a later quality layer.
- Generalized winding numbers: later repair path for open, intersecting, or
  non-manifold source meshes.

## Scope

- Add `surface-trim` as a canonical mesh operation usable before `solidify`.
- Bind every contour and keep-region point to immutable source mesh identity,
  triangle index, and barycentric coordinates.
- Support any number of ordered boundary points, minimum three.
- Preview feature-aware paths on the raw source mesh without running OCCT.
- Close and validate a non-self-intersecting loop.
- Select the retained region using an explicit surface seed.
- Split intersected triangles and emit the selected mesh region.
- Support explicit cap modes: Open, Flat, and Surface Fill.
- Expose applied trims with Edit and Remove through exact AST node identity.
- Preserve later `solidify`, boolean, thread, pocket, and other BRep operations.
- Keep Guides/Reconstruct/Validate subordinate to Capture; Surface Trim belongs
  to the general Crop step.

## Out Of Scope

- A Rocksteady-specific component filter or neck detector.
- Inferring a semantic cut without user evidence.
- Screen-space lasso interpreted as geometry.
- Approximating the contour with multiple hidden planes.
- Destructive writes to the imported source file.
- Silent healing that changes geometry without a canonical operation or report.
- Full volumetric remeshing in the first delivery.

## Proof Gates

- Sparse surface points produce a deterministic live contour on two differently
  tessellated fixtures.
- The contour follows the surface rather than a fitted plane.
- A keep seed selects one bounded region and rejects an ambiguous or disconnected
  result.
- Apply writes one editable `surface-trim` AST node and no hidden crop state.
- Flat cap rejects excessive non-planarity with the raw measured deviation.
- Surface Fill produces a closed mesh or reports the exact boundary failure.
- `solidify(surface-trim(import-stl ...))` renders and accepts a later boolean.
- Reload reconstructs applied trim controls from canonical source.
- Edit replaces the exact selected node; Remove unwraps only that node.
- Happy, pending, stale-source, invalid-loop, and cap-failure UI states pass.

