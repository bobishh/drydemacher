## Mesh and Image Geometry: Polygons in 3D

Ecky supports typed triangle geometry alongside analytic B-rep operations. Mesh execution is bounded and deterministic; it does not run Blender Python or arbitrary scripts.

### Open surfaces and closed solids

Use `mesh` for a triangle surface. Use `polyhedron` when the triangles form a printable solid.

```scheme
(define vertices
  '((0 0 0) (20 0 0) (0 20 0) (0 0 20)))

(define triangles
  '((0 2 1) (0 1 3) (1 2 3) (2 0 3)))

(model
  (verify
    (tag mesh_clean)
    (metric bad_edges (stl non-manifold-edge-count))
    (expect bad_edges (= 0)))
  (part tetrahedron
    (polyhedron
      :vertices vertices
      :triangles triangles)))
```

`mesh` permits boundaries and previews them honestly as an open surface. `polyhedron` requires one closed orientable component with nonzero volume. Both reject invalid indices, repeated vertices, zero-area faces, duplicates, inconsistent winding, and resource-budget overflow before render.

Prefer formula-generated vertex/triangle lists for repeated or mathematical geometry. Keep one binding for each list instead of expanding thousands of copied triangles into source.

### Heightmaps become dimensioned relief

`heightfield` samples image luminance into a closed planar mesh. Physical dimensions remain explicit.

```scheme
(model
  (verify
    (tag relief_closed)
    (metric bad_edges (stl non-manifold-edge-count))
    (expect bad_edges (= 0)))
  (part relief
    (heightfield image-path
      :width 100
      :depth 70
      :relief-height 4
      :base-thickness 1.2
      :invert #f)))
```

The image path points to a staged local asset. Empty selection is pending, not fake geometry. Decode errors retain raw path/error evidence. Width, depth, relief height, and base thickness must be positive.

### Orthographic images become reviewed sketches

Front, Top, and Side line art follows a different route:

1. select each raster and enter physical calibration;
2. tune threshold/inversion;
3. extract closed contour candidates;
4. review a candidate into an editable sketch primitive;
5. run existing preview-hull and exact candidate validation.

Raster provenance records asset digest, view, calibration, threshold, inversion, contour id, and extractor version. Failed extraction preserves the last reviewed sketch. Preview hull remains diagnostic until STEP and hidden-line validation pass.

### Export truth follows representation

Pure mesh output offers STL. Multipart viewer assets also enable 3MF or multipart STL export. Pure mesh does not offer STEP.

A closed mesh may enter the hybrid `import-stl -> solidify` bridge before a supported BRep boolean. Successful STEP from that route is labeled **Faceted poly-BRep** and carries source mesh digests/topology evidence. It is triangle-derived, not analytic source CAD.

Reference photos are another route: a vision model can propose inferred `.ecky` source, then normal compilation and verification run. One perspective photo remains an inferred approximation; response text alone cannot mark it reconstructed or accepted CAD.
