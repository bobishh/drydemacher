# Proposal: Poly BRep Bridge

## Intent

Enable hybrid rendering where exact OCCT BRep operations (extrude, chamfer,
fillet, boolean) and mesh-only operations (wall-pattern, future imported/AI
meshes) coexist in a single part and are combined into a single printable
solid.

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
(rear-panel-finished (difference rear-panel-patterned camera-cutout))       ;; CSG on displaced mesh = garbage
```

The mesh renderer can do all operations individually, but CSG over a
displacement-modified mesh destroys topology. The OCCT renderer produces clean
booleans but cannot do `wall-pattern` at all. There is no bridge between them.

## Root Cause

The render dispatch is all-or-nothing per part:

- `resolve_dispatch_backend` picks one backend for the entire part.
- OCCT receives a CoreProgram and either plans every op as BRep or rejects
  the model (rejecting wall-pattern).
- The mesh renderer receives the same CoreProgram and evaluates every op as
  triangle mesh CSG, producing garbage on post-displacement booleans.

There is no mechanism to split a part's op tree at the mesh boundary, run each
side through the engine that handles it, and recombine the results.

## Scope

- Define a **partition analysis** pass over Core IR that classifies each part
  as PureOcct, PureMesh, or Hybrid based on where mesh-only ops sit relative
  to BRep-required ops.
- Define a **poly BRep bridge**: convert a displaced triangle mesh into an
  OCCT planar-faceted BRep solid (via the existing `import-stl` +
  `solidify` OcctPlan ops), so it re-enters OCCT as real topology that
  boolean ops can consume.
- Wire **hybrid dispatch** into the render pipeline: mesh renderer handles
  the part up to and including displacement; the displaced STL is fed to OCCT
  for solidification and post-boundary boolean ops.
- Preserve STEP export for the final hybrid solid.
- Short-circuit PureOcct and PureMesh parts so they use existing paths with
  zero regression.

## Out of Scope

- **OCCT tessellation of exact pre-boundary geometry (former T2).** The mesh
  renderer already evaluates its own sub-tree including extrude. We do not
  need to tessellate OCCT output and feed it to wall-pattern — wall-pattern
  runs on the mesh renderer's own geometry. T2 was an optimization for
  higher base precision; it is deferred until profiling shows a need.
- **Mesh ops on an externally-provided mesh (former T3).** Same reason: the
  mesh renderer handles the full pre-displacement + displacement chain today.
- **MeshAsset interface (former T6).** There is exactly one mesh source
  (`wall-pattern`) today. Designing a `MeshSource` enum before a second
  source exists is premature. The bridge works on any STL path; future
  sources (imported mesh, image relief, AI-generated) plug in when they
  exist.
- Replacing the existing pure-mesh renderer for PureMesh parts.
- AI mesh generation itself.
- Removing build123d/FreeCAD lowering backends.

## Approach

Per-part pipeline, driven by partition analysis:

```
Core IR part tree
  │
  ├ [Partition] classify: PureOcct | PureMesh | Hybrid
  │
  ├ PureOcct:  existing OCCT path (no change)
  ├ PureMesh:  existing mesh renderer path (no change)
  │
  └ Hybrid:
      1. Render whole part through mesh renderer (it handles extrude +
         wall-pattern + any mesh-safe ops).
      2. Take the displaced STL output.
      3. Feed to OCCT plan: import-stl(displaced.stl) → solidify →
         post-boundary boolean ops (difference/union/chamfer/fillet).
      4. Export STL + STEP from OCCT.
```

This is simpler than the original 5-phase design because we do not need to
tessellate OCCT exact geometry into the mesh renderer or teach wall-pattern
to accept external meshes. The mesh renderer already does displacement; OCCT
already does booleans on solidified meshes. The bridge is just the handoff.

## What is proven

- **OCCT boolean on solidified poly BRep is reliable.** Empirical proof:
  VertexGenie mascot (120 organic displaced triangles, 0 non-manifold) →
  `solidify(import-stl(...))` → `difference(cylinder)` → 0 non-manifold edges,
  volume reduced, single component. Commit `d69974d`.
- **Naive `import-stl` without solidify fails.** Same model without
  `solidify` produces 73 non-manifold edges — OCCT cannot determine
  inside/outside on an unsewn compound. `solidify` (BRepBuilderAPI_Sewing +
  BRepBuilderAPI_MakeSolid) is the required fix.
- **OCCT preserves topology on round-trip.** `import-stl` → export STL keeps
  120→120 triangles, 0 non-manifold. The import path is lossless.
- **Partition analysis works.** 13 BDD tests classify PureOcct, PureMesh,
  Hybrid correctly across single/multi-part, cutter-branch, and chained
  wall-pattern scenarios. Commit `8b79e4b`.

## Proof Gates

- [x] PG-PROOF OCCT can boolean a solidified poly BRep from a realistic
  displaced mesh (VertexGenie proof, commit `d69974d`).
- [ ] PG1 A model using `wall-pattern` followed by `difference` renders as a
  single manifold solid with < 100 non-manifold edges.
- [ ] PG2 The iPhone 17e case fixture renders as 3 clean parts.
- [ ] PG3 STEP export includes exact faces for non-displaced geometry.
- [ ] PG4 STL export is sliceable without manual repair.
- [ ] PG5 Models with no mesh-only ops still route through pure OCCT.
- [ ] PG6 Models with only mesh ops still route through pure mesh renderer.
- [ ] PG7 `cd src-tauri && cargo check` passes.
- [ ] PG8 Existing direct-OCCT fixtures still render (no regression).
- [ ] PG9 Existing mesh-renderer fixtures still render (no regression).
