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
  1. Mesh renderer evaluates each independent mesh island until the first
     BRep-required operation and emits STL.
  2. Each STL becomes an engine-independent MeshAsset.
  3. OCCT plan: import-stl(asset.stl) → solidify → post-boundary
     BRep-required ops.
  4. Export STL + STEP from OCCT with representation provenance.
```

`MeshAsset` contains validated STL plus provenance, not renderer state.
Internal displacement, imported meshes, and provider/LLM-generated meshes use
the same OCCT bridge. Typed `polyhedron` is the direct LLM-friendly input.

### Surface Operation Route Authority

`chamfer` and `fillet` are BRep surface operations. For analytic Core IR
inputs, they belong to Direct OCCT and must not be rewritten into mesh
operations. For hybrid parts, they stop the mesh phase exactly like booleans,
shell, and offset. The OCCT phase then applies them to either analytic BRep
or a solidified mesh-origin poly BRep.

The Rust mesh evaluator may keep mesh-native edge-op helpers only for geometry
whose origin is already mesh-native (`mesh`, `polyhedron`, `import-stl`, or a
mesh-only op such as `wall-pattern`). It is not an alternate implementation
for analytic `box/cylinder/extrude -> chamfer/fillet`.

Mesh-origin surface ops have an explicit admission policy, not an exception
or fallback. After `solidify(import-stl(...))`, OCCT may apply
`chamfer`/`fillet` only when the selected edge set is bounded and intentional:

- exact target ids or a selector that resolves to a small, reported edge set;
- estimated selected-edge count and faceted-face count under configured
  limits;
- resulting STEP is marked `facetedPolyBRep`, never `analyticBrep`.

If a broad selector such as `all` would touch thousands of polyhedral edges,
render validation rejects the model with the selected-edge count and the
route reason. It must not silently fall back to polygon chamfer, global
simplification, or a different kernel.

This boundary prevents three failure modes:

- analytic models silently losing exact topology because Direct OCCT was
  unavailable or a dispatcher used a mesh fallback;
- hybrid partition rewriting a post-boundary surface op into the mesh island;
- broad chamfer/fillet on decorative faceted shells becoming a multi-minute
  OCCT job with poor semantics;
- UI/MCP/tooling claiming “mesh” or “OCCT” from `geometryBackend` instead of
  artifact representation truth.

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
- **Surface op stop**: `chamfer` and `fillet` are BRep-required and stop the
  mesh phase. The mesh output node is the last mesh-native node below them,
  not the surface op itself.

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
StlAPI_Reader shared shell       → preserve triangle topology
BRepBuilderAPI_MakeSolid(shell)  → close shell into solid
VolumeProperties check           → reverse if inverted
```

This is proven: `solidify(import-stl(genie))` → `difference(cylinder)` → 0
non-manifold edges. Without `solidify`: 73 non-manifold edges.

### The `solidify` OcctOp

Added as `OcctOp::Solidify` to the enum. Takes one shape operand (a compound
of faces). Emits C++ that sews + makes solid. The pattern mirrors the existing
`solidify_swept_shell` C++ helper already used by the hull operation.

Source: `src-tauri/src/ecky_cad_host/direct_occt_executor.rs`,
`OcctOp::Solidify` match arm.

### Hybrid Dispatch Wiring

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
1. Find maximal mesh islands. Multiple mesh branches remain separate assets.
2. Render each mesh island and validate its STL as `MeshAsset`.
3. Construct an OCCT plan for the post-boundary ops:
   - import-stl(mesh_asset_path) → solidify → [post-boundary ops]
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

### Decision: BRep surface ops are never polygon pushdown

Ordinary mesh-native inputs stay in the mesh renderer until they meet a
BRep-required consumer. `chamfer` and `fillet` are consumers. They do not get
pushed into the mesh island, because applying those operations to triangle
soup changes semantics and breaks the exact-CAD contract. Once a mesh-origin
island is solidified into a poly BRep, OCCT applies the surface operation.

Exact prelude exists only for the opposite crossing: when an exact BRep shape
is deliberately converted into a mesh-only operation input. It must not be
used to justify polygon edge-op fallback for analytic CAD.

### Decision: representation provenance is authoritative

`geometryBackend` describes the requested/runtime bucket. It does not prove
the representation of the artifact. Every render path must publish artifact
truth:

- Direct OCCT pure exact path: `GeometryRepresentation::AnalyticBrep` on the
  bundle, manifest, and STEP export.
- Hybrid OCCT poly bridge: `FacetedPolyBrep` on the bundle, manifest, and
  STEP export when STEP is emitted.
- Mesh-only path: `MeshNative`, no fabricated STEP.

MCP digests, export labels, and UI evidence read these fields. They must not
infer STEP exactness, faceting, or mesh fallback from the name `EckyRust`,
`mesh`, or from presence of a preview STL.

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

### Decision: MeshAsset is provider-neutral

`MeshAsset` validates a non-empty STL and records provenance only:
`EckyMeshPhase`, `Imported`, or `Generated { provider, model }`. Hybrid
consumers never depend on csgrs, Meshy, or another generator SDK. OBJ and other
formats require normalization to STL before entering this contract.

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

### Risk: chamfer/fillet on poly-origin BRep edges

OCCT `BRepFilletAPI_MakeChamfer` on polyhedral edges may produce approximate
results. This is acceptable only for mesh-origin geometry. Analytic geometry
must stay analytic and use OCCT directly.

Mitigation: require mesh-origin surface-op admission before execution. Small
explicit selected edge sets may run in OCCT; broad selected sets over dense
faceted shells reject with diagnostics and no hidden fallback. A future
mesh-native decorative bevel op can be added under a separate name if the
product needs “soften every triangle edge” behavior.

### Risk: STEP file size

Poly faces inflate STEP files. The VertexGenie STEP is 271KB for 120 faces.
A real phone case could be large. Mitigation: the STEP is still valid and
importable; size is a warning, not a blocker.
