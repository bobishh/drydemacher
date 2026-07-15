# Design: Poly BRep Bridge

## Architecture

### Current: Two Disjoint Worlds

```
CoreProgram
  ├── dispatch_backend == OCCT
  │     → plan every op as BRep
  │     → STEP + STL + topology
  │     → REJECTS wall-pattern
  │
  └── dispatch_backend == Mesh
        → evaluate every op as triangle soup
        → STL only (no STEP)
        → CSG over displaced mesh = garbage
```

No part can use both. The dispatch is all-or-nothing.

### Target: Hybrid Pipeline

```
CoreProgram part tree
  │
  ├ T1: Partition Analysis
  │   Walk the CoreNode tree. Find the FIRST node whose operation is
  │   mesh-only (wall-pattern, pattern, future: import-mesh).
  │   Everything above it = pre-boundary (exact BRep capable).
  │   Everything below it = post-boundary (must run as mesh first).
  │
  ├ T2: Pre-boundary sub-tree → OCCT
  │   Standard OcctPlan path. Produces an exact BRep solid.
  │   Tessellate result to triangle mesh at boundary point.
  │
  ├ T3: Mesh-only ops → Rust mesh renderer
  │   Apply wall-pattern / displacement to the tessellated mesh.
  │   All mesh renderer ops available (its own chamfer/fillet/CSG).
  │
  ├ T4: Mesh → OCCT Poly BRep (bridge)
  │   Wrap displaced triangle mesh as OCCT polyhedral BRep solid:
  │     - Each triangle → BRepBuilder face with Poly_Triangle
  │     - Sew into closed shell → make solid
  │   This is an OCCT first-class representation.
  │
  ├ T5: Post-boundary ops → OCCT hybrid boolean
  │   difference / union / fuse over hybrid solids.
  │   OCCT General Fuse Algorithm handles exact + poly.
  │   chamfer / fillet on poly edges may degrade (expected).
  │
  └ Export
      STL:  tessellate entire hybrid solid (one mesh)
      STEP: exact faces remain exact NURBS;
            poly faces exported as triangulated BRep faces
```

### Partition Boundary Definition

A part tree node is a **mesh boundary** if its operation is in
`ECKY_RUST_ONLY_CAD_OPS` (currently `["wall-pattern"]`, future:
`import-mesh`, `relief-from-image`, etc.).

```
Example part tree:
  difference                          ← post-boundary (T5: OCCT hybrid)
    ├── chamfer                       ← post-boundary
    │   └── wall-pattern              ← BOUNDARY NODE
    │       └── extrude               ← pre-boundary (T2: OCCT exact)
    ├── cylinder (camera hole)        ← pre-boundary (T5: OCCT boolean cutter)
    └── cylinder (mic hole)           ← pre-boundary (T5: OCCT boolean cutter)

Partition:
  T2 (OCCT exact):  extrude
  T3 (Mesh):        wall-pattern(extrude_result)
  T5 (OCCT hybrid): difference(chamfer(mesh), cylinder, cylinder)
```

When multiple cutters are exact-BRep (cylinders), they fuse directly in OCCT
without mesh conversion — only the displaced sub-tree enters as poly.

### OCCT Polyhedral BRep

OCCT supports polyhedral BRep as a standard representation:

- `BRep_Builder` can create faces with `Poly_Triangulation` geometry.
- `BRepAlgoAPI_Cut` / `Fuse` / `Common` work on mixed exact+poly solids.
- STEP export writes poly faces as faceted BRep (AS1/AP203 compatible).
- Quality of boolean results depends on tessellation density — finer mesh =
  slower but more stable intersection.

This is NOT reverse-engineering mesh to NURBS. The poly faces stay poly.
The boolean algorithm computes intersections on the triangulated shell
directly.

### Process Boundary

The Direct OCCT runner is a separate C++ process. The mesh renderer is pure
Rust. The bridge requires serialization:

```
Rust mesh renderer
  → produces csgrs::Mesh (triangles + vertices)
  → serialize to temp file (.stl or .obj or binary mesh blob)
  → pass path to OCCT runner plan

OCCT runner
  → reads mesh file
  → BRepBuilder creates poly shell from triangles
  → continues with hybrid boolean plan
```

The `OcctPlan` schema gains a new command type:

```json
{
  "op": "import_poly_mesh",
  "meshPath": "/tmp/ecky-mesh-xxxxx.stl",
  "resultSlot": "poly_shell_1"
}
```

Subsequent `cut` / `fuse` commands reference `poly_shell_1` as an operand,
same as any other shape slot.

### Short-Circuit: Pure Mesh Path

If partition analysis finds no post-boundary BRep ops (all ops after the
boundary are mesh-safe), skip T4/T5 entirely:

```
wall-pattern → chamfer (mesh) → difference (mesh)
                                          ↑ all mesh-safe
→ mesh renderer output is final STL, no OCCT round-trip
```

This preserves current behavior for models where mesh-only output is
acceptable and avoids OCCT overhead.

### Short-Circuit: Pure OCCT Path

If no mesh-only ops exist in the part, partition analysis returns no
boundary → standard OCCT exact path. Zero overhead, zero regression.

## Design Decisions

### Decision: Tessellate at boundary, not per-op

Tessellate once when crossing the exact→mesh boundary. This gives the
displacement op a clean, uniform mesh to work with. Retessellating per
operation would fragment topology.

### Decision: Poly BRep for post-boundary, not mesh CSG

Post-boundary booleans go through OCCT, not the Rust mesh CSG. This is the
core fix: OCCT boolean over poly shells is stable because OCCT's intersection
algorithm works on the BRep topology graph, not on raw triangle soup. The
Rust mesh CSG (`csgrs`) fails on displaced meshes because it assumes
coincident vertices and clean edge topology that displacement destroys.

### Decision: Chamfer/fillet on poly edges is best-effort

OCCT `BRepFilletAPI_MakeChamfer` on polyhedral edges may produce approximate
results. This is acceptable — the user chose mesh displacement, accepting
surface approximation. The alternative (mesh chamfer) is the current
non-working path.

### Decision: MeshAsset interface for future generated content

Define `MeshAsset` as the pipeline entry point for any triangle mesh that
needs to enter OCCT:

```
enum MeshSource {
    WallPattern { spec: WallPatternSpec, target: Mesh },
    ImportedMesh { path: String },
    // future:
    // ImageRelief { image: ImageBuffer, depth: f64 },
    // GeneratedMesh { prompt: String },
}
```

`wall-pattern` becomes `MeshSource::WallPattern`. Future image/relief/AI
content adds variants. All flow through the same poly BRep bridge.

## Technical Risks

### Risk: OCCT poly boolean performance

Boolean over dense poly shells (100k+ triangles from fine displacement) can
be slow (seconds to minutes). Mitigation: control tessellation density at
the boundary, allow coarse poly for boolean, fine poly for final export.

### Risk: OCCT poly boolean instability

General Fuse Algorithm on mixed representation is tolerance-sensitive.
Degenerate triangles (zero-area, collinear) from displacement can cause
intersection failures. Mitigation: mesh sanitization (already implemented)
before poly BRep wrapping. The current `sanitize_mesh_for_export` is the
first layer; poly BRep wrapping adds a second validation layer.

### Risk: STEP file size

Poly faces inflate STEP files (triangulated BRep is verbose). Mitigation:
only export STEP for parts with exact geometry; mark poly-only parts as
STL-only in the manifest.
