# Proposal: Poly BRep Bridge

## Intent

Enable hybrid rendering where exact OCCT BRep operations (extrude, chamfer,
fillet, boolean) and mesh-only operations (wall-pattern, future image/description-
generated meshes) coexist in a single part and are combined into a single
printable solid.

Today the two backends are disjoint: a part either goes entirely through OCCT
(exact BRep, STEP export) or entirely through the Rust mesh renderer (triangle
soup, STL only). When a part uses `wall-pattern` followed by CSG `difference`,
the mesh renderer produces garbage — 30 000+ non-manifold edges, thousands of
disconnected components — because boolean operations over displaced triangle
meshes are unstable.

## Problem

The iPhone 17e case demonstrates the failure:

```
(rear-panel-raw (extrude outer-case-profile rear-panel-thickness))
(rear-panel-patterned (wall-pattern (:mode cellular ...) rear-panel-raw))   ;; mesh displacement
(rear-panel-finished (chamfer ... rear-panel-patterned))                    ;; CSG on displaced mesh = garbage
```

The mesh renderer can do all operations individually, but CSG over a
displacement-modified mesh destroys topology. The OCCT renderer produces clean
booleans but cannot do `wall-pattern` at all. There is no bridge between them.

This blocks two product visions:

1. **Textured parametric parts** — cellular/gyroid/rib patterns on functional
   geometry (phone cases, grips, lattice structures).
2. **Generated mesh content** — user-supplied or AI-generated meshes (face
   embossing on a knob, image-relief panels, Meshy-style generated shapes)
   fused into parametric BRep bodies.

## Root Cause

The render dispatch is all-or-nothing per part:

- `resolve_dispatch_backend` picks one backend for the entire part.
- OCCT receives a CoreProgram and either plans every op as BRep or rejects
  the model.
- The mesh renderer receives the same CoreProgram and evaluates every op as
   triangle mesh CSG.

There is no mechanism to:

- Execute a sub-tree through OCCT, tessellate the result, hand the mesh to
  the displacement op, then wrap the displaced mesh back into a BRep shell
  for downstream exact operations.
- Represent a triangle mesh as a first-class OCCT polyhedral BRep face so
  that `BRepAlgoAPI_Cut` / `Fuse` can operate on the hybrid.

## Scope

- Define a **partition analysis** pass over Core IR that identifies where the
  exact-BRep → mesh boundary falls within each part.
- Define a **mesh → OCCT poly BRep** serialization bridge so displaced meshes
  re-enter OCCT as polyhedral solids.
- Wire **hybrid dispatch** into the render pipeline: pre-boundary ops → OCCT,
  mesh-only ops → mesh renderer, post-boundary ops → OCCT poly BRep booleans.
- Preserve STEP export for the exact sub-trees; tag poly faces in the final
  BRep so slicers and STEP consumers can distinguish precision.
- Establish a **mesh asset interface** so future image/relief/AI-generated
  meshes flow through the same bridge.

## Out of Scope

- Replacing the existing pure-mesh renderer for models that use no BRep ops
  after displacement (short-circuit: if all post-displacement ops are mesh-
  safe, stay in mesh renderer).
- AI mesh generation itself (Meshy integration, image-to-mesh). This change
  defines the pipeline slot and interface; generation is a separate change.
- Automatic mesh repair / remeshing of displaced geometry. The bridge passes
  through what it receives; quality of the displacement output is the mesh
  op's responsibility.
- Removing build123d/FreeCAD lowering backends.

## Approach

Three-phase pipeline per part:

```
Core IR part tree
  → [T1] partition analysis
      identifies boundary node (first mesh-only op)
  → [T2] pre-boundary sub-tree → OCCT exact BRep
      tessellate to mesh at boundary
  → [T3] mesh-only ops → Rust mesh renderer
      wall-pattern / displacement on tessellated mesh
  → [T4] wrap displaced mesh as OCCT poly BRep solid
  → [T5] post-boundary ops → OCCT hybrid boolean
      exact + poly in same BRepAlgoAPI call
  → export: STL (full), STEP (exact faces exact, poly faces poly)
```

If no post-boundary BRep ops exist, skip T4/T5 — mesh renderer output is the
final STL.

## Future: Generated Mesh Content

The poly BRep bridge is the enabler for mesh-from-image / mesh-from-description
content. Once the bridge exists, a generated mesh (face relief, organic shape)
enters the pipeline at the same point as `wall-pattern` output — as a triangle
mesh wrapped into OCCT poly BRep, then fused with the parametric body. This
change defines the interface (`MeshAsset` data model) but does not implement
generation.

## Proof Gates

- A model using `wall-pattern` followed by `difference` renders as a single
  manifold solid with < 100 non-manifold edges (vs current 30 000+).
- The iPhone 17e case fixture renders as 3 clean parts.
- STEP export includes exact faces for non-displaced geometry.
- STL export is sliceable without manual repair.
- Models with no mesh-only ops still route through pure OCCT (no regression).
- Models with only mesh ops still route through pure mesh renderer (no regression).
- `cd src-tauri && cargo check` passes.
