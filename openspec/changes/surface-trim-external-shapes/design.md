# Design: Surface Trim for External Shapes

## Goal

Provide one general surface segmentation and cutting primitive for external
triangle meshes. Interaction stays fast on raw STL. Applied result stays
reproducible in `.ecky` and can cross the mesh-to-BRep boundary through
`solidify`.

## User Flow

```text
EXTERNAL SHAPES > CROP
  CUT PLANE          planar boundary, three points
  TRACE SURFACE      non-planar boundary, N points

TRACE SURFACE
  1. Click sparse points around desired boundary.
  2. See live surface path from last point to cursor.
  3. Undo or move bad points.
  4. Close loop.
  5. Click the region to keep.
  6. Choose Open, Flat, or Surface Fill.
  7. Preview retained region and boundary.
  8. Apply, Edit, Remove, or Cancel.
```

The Viewer shows the source mesh, numbered hard points, computed surface path,
closed-loop state, retained-region tint, and cap preview. Plane and box overlays
remain disabled while Surface Trim interaction is active.

## Canonical Source

Surface Trim is a mesh operation inside `solidify`, not an out-of-band STL
rewrite:

```lisp
(solidify
  (surface-trim
    (import-stl "donor.stl")
    :schema-version 1
    :source-digest "sha256:..."
    :loop
      ((mesh-anchor 1842 0.12 0.31 0.57)
       (mesh-anchor 1907 0.44 0.08 0.48)
       (mesh-anchor 2051 0.09 0.66 0.25)
       (mesh-anchor 2210 0.51 0.20 0.29))
    :keep-seed (mesh-anchor 722 0.30 0.30 0.40)
    :path-mode "feature"
    :cap "flat"))
```

`mesh-anchor` stores source triangle index followed by normalized barycentric
weights. The enclosing digest prevents replay against another mesh. Point
coordinates are recomputed from source vertices; frontend coordinates never
become canonical authority.

Applied trims compose with surrounding mesh and BRep nodes. Schema v1 permits
one `surface-trim` wrapper per exact `import-stl` source. A second contour on the
same import must edit that wrapper; nested trim replay is rejected because its
anchors are indexed against immutable input STL triangles, not the first trim's
derived mesh. Separate imported sources may each own one trim. Edit replaces the
selected node by AST path. Remove replaces the selected wrapper with its shape
child. Neither action rewrites unrelated source.

### Lossless Trim Versioning

Every settled change to a persisted trim file, path/loop candidate, keep-region
selection, cap preview, or Apply draft first appends an immutable version with
the exact payload/digest. Anchor, topology, cap, source-digest, and render
validation attach raw status/evidence to that version after append. Failed,
pending, and stale attempts remain history and become head.

Head is the last serialized append. A successful-render query is a separate
projection and never substitutes for head. Stale or concurrent writers append
both changed snapshots in serialization order; version writes do not emit
`conflict`, `threadAdvanced`, or require `force`. Geometric failures such as a
stale source digest, invalid loop, or non-manifold result remain explicit
validation failures attached to the attempted version.

## Geometry Pipeline

### 1. Source validation

- Resolve the exact bound `import-stl` node and canonical file path.
- Reparse bytes and recompute digest.
- Reject digest drift before preview or Apply.
- Validate finite barycentrics, sum tolerance, triangle bounds, non-degenerate
  source triangle, and reconstructed hit position.
- Require all boundary anchors on one connected surface component.

### 2. Cached surface graph

Build once per source digest:

- indexed vertices and triangles;
- edge-to-face adjacency;
- connected components;
- face normals and signed/absolute dihedral features;
- spatial lookup for hit and path refinement;
- optional boundary/non-manifold diagnostics.

Cache is derived and disposable. Digest is cache key. No cache entry is geometry
authority.

### 3. Live path

Each committed point is a hard constraint. Between consecutive points, solve a
deterministic surface path with Dijkstra/A* over mesh adjacency. Cost combines:

```text
edge length
+ deviation from local point-to-point direction
+ smooth-surface penalty
- bounded crease attraction from absolute dihedral magnitude
```

Weights are versioned constants in schema v1. Equal costs resolve by stable
vertex/edge index. `path-mode "shortest"` disables feature attraction;
`"feature"` is default.

The first implementation may use the robust edge path to identify a triangle
corridor, but final emitted cut SHALL refine inside that corridor and split
crossed triangles. Output quality must not depend on the loop landing only on
pre-existing mesh edges.

Pointer preview is throttled. Each request carries a monotonically increasing
preview id. Viewer ignores stale responses. No OCCT render occurs while placing
points.

### 4. Loop validation

Closing the loop SHALL reject:

- fewer than three distinct anchors;
- repeated zero-length segments;
- disconnected path segments;
- topological or geometric self-intersection;
- a loop that does not partition the selected surface;
- non-manifold ambiguity at the boundary;
- a selected region below configured area/triangle tolerances.

The raw reason and involved segment ids reach UI.

### 5. Region selection

The computed loop is a traversal barrier. The keep seed is projected and
validated against the same source digest. Flood fill/graph cut selects the face
region containing that seed.

Semantics are explicit: `surface-trim` returns only the seed-containing region.
Other connected components are not silently retained. Flipping selection means
choosing another seed, not interpreting camera direction as inside/outside.

### 6. Exact mesh cut

- Insert boundary vertices at continuous path crossings.
- Split crossed edges and triangles deterministically.
- Preserve source winding on retained faces.
- Remove faces outside the selected region.
- Weld only newly coincident boundary vertices within a documented tolerance.
- Emit boundary-loop diagnostics before capping.

Original STL bytes remain unchanged.

### 7. Cap modes

`OPEN`

- Leave selected surface open.
- Valid for segmentation/export, not valid input to `solidify` unless another
  operation closes it.

`FLAT`

- Fit a least-squares plane to the emitted boundary.
- Report maximum and RMS deviation.
- Reject above planarity tolerance or when projected polygon self-intersects.
- Triangulate the projected polygon with preserved boundary and orient the cap.
- Preferred for later pockets, threads, mounting faces, and BRep booleans.

`SURFACE FILL`

- Triangulate a non-planar boundary using constrained advancing-front/minimal
  patch behavior.
- Reject foldovers, inverted triangles, or unresolved non-manifold output.
- Remains a mesh patch; `solidify` performs the later mesh-to-BRep transition.

No mode silently falls back to another.

## Runtime Boundary

`surface-trim` executes in the indexed-mesh runtime before `solidify`. Planner
must support:

```text
import-stl -> surface-trim -> solidify -> BRep booleans/features
```

It must not route raw mesh trimming through OCCT booleans. After a closed trim,
`solidify` receives a manifold oriented mesh and reports the same exact topology
checks used for direct imported solids.

## Contracts

Backend contracts own translation and use camel-case serialization:

- `SurfaceTrimAnchor`: source digest, triangle index, barycentric weights.
- `SurfaceTrimPathRequest`: source node id, from/to anchors, path mode, preview id.
- `SurfaceTrimPathPreview`: preview id, source positions, path metrics, warnings.
- `SurfaceTrimApplyRequest`: target thread/message snapshot, source node id,
  ordered loop, keep seed, cap mode, optional edited AST node id.
- `AppliedSurfaceTrim`: AST node id, source node id, loop count, path mode, cap
  mode, topology/cap report.

Frontend invoke payloads stay camelCase. Rust fields stay snake_case with
`#[serde(rename_all = "camelCase")]`.

## Ownership

- Viewer: raycast, visible handles, path/region/cap overlays, interaction routing.
- External Shapes Crop step: tool state, actions, raw failures, applied-node list.
- Rust surface-trim service: source identity, graph/cache, path, validation,
  segmentation, cutting, cap, deterministic report.
- Ecky compiler/planner: canonical operation signature and mesh-runtime route.
- Source binding/AST patch service: Apply/Edit/Remove and history guards.
- `model.ecky`: only durable geometry authority.

## Performance Budget

- Raw source mesh remains the only Viewer asset during point placement.
- Graph construction occurs once per digest and may run off main UI thread.
- Hover path requests are throttled and stale-safe.
- Region/cap preview runs only after loop closure or seed change.
- Apply performs one canonical render/version operation.
- No debug overlay enters STL/STEP export.

Budgets are verified through counters and deterministic fixture sizes rather
than brittle wall-clock assertions.

## Failure Model

UI displays backend body without generic replacement for:

- missing source file;
- source digest changed;
- invalid or stale anchor;
- no connected path;
- self-intersecting loop;
- non-partitioning/non-manifold loop;
- ambiguous keep seed;
- flat-cap planarity failure;
- surface-fill triangulation failure;
- resulting mesh not closed/oriented/manifold;
- `solidify` or later BRep failure.

Canonical source and current successful preview projection stay unchanged on
failure; the attempted version and raw failure evidence remain retained and
head.

## Later Refinement

Schema leaves room for optional keep/remove strokes. They can drive constrained
random-walk or graph-cut segmentation when sparse hard points remain ambiguous.
Connectivity-independent field curves and winding-number repair are later
algorithms behind new explicit modes, not silent changes to schema-v1 results.

## Rejected Paths

- Fit all points to one plane: extra points do not create a curved boundary.
- Stack many planes: creates facets and unrelated intersections.
- Screen lasso: loses hidden-side and source-coordinate meaning.
- Viewer-only clipping: cannot reproduce, edit, validate, or solidify.
- Save a generated STL path: creates a second hidden geometry authority.
- Always keep largest component: semantic region may be smaller.
- Always cap: arbitrary non-planar boundaries cannot safely receive a flat cap.
- Embed VTK as a new runtime dependency: behavior is useful; dependency size and
  native packaging are not justified for this operation.
