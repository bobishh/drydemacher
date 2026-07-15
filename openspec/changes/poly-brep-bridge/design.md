# Design: Poly BRep Bridge

## Architecture

### Two renderers, not interchangeable

**Direct OCCT** (BRep): NURBS surfaces. Handles extrude, chamfer, fillet,
difference, union exactly. Does NOT understand wall-pattern. Produces STEP +
STL.

**Mesh renderer** (csgrs): triangle mesh. Handles ALL ops including
wall-pattern and CSG. Lower precision, no STEP export. CSG over displaced
meshes (wall-pattern output) produces garbage — 30k+ non-manifold edges.

The bridge lets a single part use both: mesh renderer for displacement, OCCT
for the booleans that follow.

### Target: Hybrid Dispatch

```
Partition analysis classifies each part:

PureOcct  → existing OCCT path (no change)
PureMesh  → existing mesh renderer path (no change)

Hybrid:
  1. Mesh renderer evaluates the WHOLE part (extrude + wall-pattern + any
     mesh-safe ops). Produces a displaced STL.
  2. OCCT plan: import-stl(displaced.stl) → solidify → post-boundary booleans.
  3. Export STL + STEP from OCCT.
```

The key simplification vs the original design: we do NOT tessellate OCCT
exact geometry to feed the mesh renderer (T2), and we do NOT teach wall-pattern
to accept external meshes (T3). The mesh renderer already evaluates its own
sub-tree including extrude; wall-pattern runs on that. OCCT only sees the
final displaced STL.

### Partition Boundary Definition

A node is a **mesh boundary** if its operation is in `ECKY_RUST_ONLY_CAD_OPS`
(currently `["wall-pattern"]`).

```
Example part tree (iPhone 17e case):
  difference                          ← BRep-required, post-boundary
    ├── wall-pattern                  ← MESH BOUNDARY
    │   └── extrude (case profile)    ← pre-boundary (mesh renderer handles)
    ├── cylinder (camera hole)        ← pre-boundary (OCCT boolean cutter)

Classification: Hybrid
  Mesh renderer produces:  wall-pattern(extrude(profile)) → displaced STL
  OCCT plan:               solidify(import-stl(displaced.stl))
                           → difference(_, camera_cylinder)
                           → difference(_, mic_cylinder)
```

Partition rules (implemented in `poly_partition.rs`):

- **PureOcct**: no mesh-only ops anywhere in the part.
- **PureMesh**: mesh-only ops exist, but no BRep-required op (difference,
  union, chamfer, fillet, shell, offset) consumes their output. Only
  mesh-safe ops (translate, rotate, scale, mirror, group) sit above.
- **Hybrid**: at least one mesh-only op AND at least one BRep-required op
  whose input is post-boundary (consumes displaced output, directly or
  transitively).

### OCCT Planar Faceted BRep

OCCT's `StlAPI_Reader::Read` internally calls `BRepBuilderAPI_MakeShapeOnMesh`,
which converts each triangle into:
- shared `BRepBuilderAPI_MakeVertex` (deduped by index)
- shared linear `BRepBuilderAPI_MakeEdge`
- planar `BRepBuilderAPI_MakeFace` per triangle

Result: a `TopoDS_Compound` of planar faces — real BRep topology with real
geometry (plane through 3 points), real edges, real vertices. Boolean
algorithms work on it natively.

**Critical step: solidify.** `StlAPI_Reader` produces a compound of faces,
NOT a solid. `BRepAlgoAPI_Cut` on an unsewn compound produces non-manifold
garbage (73 non-manifold edges in the VertexGenie proof) because OCCT cannot
determine inside/outside without shell topology. The `solidify` OcctOp solves
this:

```
BRepBuilderAPI_Sewing(1.0e-6)  → merges coincident edges → closed shell
BRepBuilderAPI_MakeSolid(shell) → closes shell into solid
VolumeProperties check          → reverse if inverted
```

This is proven: `solidify(import-stl(genie))` → `difference(cylinder)` → 0
non-manifold edges. Without `solidify`: 73 non-manifold edges.

### The `solidify` OcctOp

Added as `OcctOp::Solidify` to the enum. Takes one shape operand (a compound
of faces). Emits C++ that sews + makes solid. The pattern mirrors the existing
`solidify_swept_shell` C++ helper already used by the hull operation.

Source: `src-tauri/src/ecky_cad_host/direct_occt_executor.rs`,
`OcctOp::Solidify` match arm.

### Hybrid Dispatch Wiring (the remaining work)

In `render_model_unlocked` (services/render.rs), after the existing
backend-resolution logic:

```
1. If source is Ecky IR:
   a. Compile to CoreProgram.
   b. Run poly_partition::analyze_program(program).
   c. If ALL parts are PureOcct → existing OCCT path.
   d. If ALL parts are PureMesh → existing mesh path.
   e. If ANY part is Hybrid → hybrid pipeline for those parts.
```

For each Hybrid part:

```
1. Render the part through the mesh renderer (existing path, unmodified).
   The mesh renderer handles extrude + wall-pattern + mesh-safe transforms.
2. The mesh renderer produces an STL at a known path.
3. Construct an OCCT plan for the post-boundary ops:
   - import-stl(mesh_stl_path) → solidify → [post-boundary ops]
   - Post-boundary ops are reconstructed from the CoreNode tree: everything
     above the boundary that is BRep-required (difference, chamfer, etc.).
   This reconstruction is the main implementation challenge — the part tree
     above the boundary may be deeply nested.
4. Execute the OCCT plan → STL + STEP export.
```

### Short-Circuit Paths

- **PureOcct** (no mesh ops): standard OCCT exact path. Zero overhead, zero
  regression.
- **PureMesh** (mesh ops but no BRep-required consumer): standard mesh
  renderer path. Zero overhead, zero regression.
- These cover the majority of existing models. Only Hybrid parts pay the
  bridge cost.

## Design Decisions

### Decision: mesh renderer handles the full pre-boundary chain

The mesh renderer already evaluates extrude, profile, and wall-pattern. We do
not need to tessellate OCCT exact geometry and feed it to wall-pattern. This
avoids the T2 (OCCT tessellation) and T3 (mesh ops on external mesh) tasks
from the original design. Trade-off: the base geometry under wall-pattern is
the mesh renderer's tessellation, not OCCT's exact NURBS. This is acceptable
because wall-pattern displaces the surface anyway — the exact base precision
is lost regardless.

### Decision: solidify is a separate op, not bundled into import-stl

`import-stl` is used in existing models where a compound of faces is the
desired output (no booleans follow). `solidify` is opt-in for the hybrid path
where booleans need a solid. Keeping them separate preserves existing import-stl
behavior and makes the intent explicit in the plan.

### Decision: post-boundary boolean ops run in OCCT, not mesh CSG

This is the core fix. OCCT boolean over solidified poly shells is stable
because OCCT's intersection algorithm works on the BRep topology graph, not
on raw triangle soup. The mesh CSG (csgrs) fails on displaced meshes because
it assumes coincident vertices and clean edge topology that displacement
destroys.

### Decision: no MeshAsset abstraction yet

There is one mesh source today: `wall-pattern`. The bridge works on any STL
path. When a second source (imported mesh, image relief, AI-generated) exists,
it plugs into the same `import-stl` + `solidify` path. Designing the enum now
would be premature.

## Technical Risks

### Risk: post-boundary op tree reconstruction

Extracting the post-boundary BRep ops from the CoreNode tree and rebuilding
them as an OCCT plan is the main implementation challenge. The tree above the
boundary may have transforms, conditionals, and nested operations. The
partition analysis identifies boundary node IDs; the dispatch must slice the
tree and rebuild the OCCT-side plan.

Mitigation: start with the common case (single difference with exact
cylinders/boxes as cutters). The iPhone 17e case fits this pattern. Generalize
incrementally.

### Risk: OCCT poly boolean performance

Boolean over dense poly shells (100k+ triangles from fine displacement) can
be slow. The VertexGenie (120 triangles) boolean runs in ~5s. A real phone
case with thousands of displaced triangles may be slower. Mitigation: control
tessellation density at the mesh renderer stage; allow coarse poly for the
boolean pass if profiling shows a need.

### Risk: chamfer/fillet on poly edges

OCCT `BRepFilletAPI_MakeChamfer` on polyhedral edges may produce approximate
results. This is acceptable — the user chose mesh displacement, accepting
surface approximation.

### Risk: STEP file size

Poly faces inflate STEP files. The VertexGenie STEP is 271KB for 120 faces.
A real phone case could be large. Mitigation: the STEP is still valid and
importable; size is a warning, not a blocker.
