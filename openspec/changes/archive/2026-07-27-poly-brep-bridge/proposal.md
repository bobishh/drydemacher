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

- Generator/provider SDK integration. Generated geometry enters through the
  provider-neutral `MeshAsset` STL contract or typed `polyhedron` data.
- OBJ-to-STL conversion. The bridge consumes STL; provider adapters normalize
  other triangle formats before creating a `MeshAsset`.
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
      1. Render each independent mesh island up to the first BRep-required
         surface/boolean op and store its STL as `MeshAsset`.
      2. Feed each asset to OCCT: import-stl(asset.stl) → solidify.
      3. Run post-boundary BRep-required ops (difference/union/chamfer/fillet)
         in OCCT, never by polygon edge rewriting.
      4. Export STL + STEP from OCCT, tagged with representation provenance.
```

The handoff is engine-independent. Internal displacement, imported STL, and
LLM/provider-generated polyhedra all become the same validated mesh asset
before OCCT solidification.

`chamfer` and `fillet` are CAD surface operations. When their input is an
analytic Core IR shape, they remain on Direct OCCT. The mesh evaluator may only
handle them when the input is already mesh-origin and no exact BRep route is
available. Silent analytic-to-mesh fallback for these operations is forbidden.

Preview STL is not the source of CAD truth. Direct OCCT artifacts must mark
their bundle, manifest, and STEP export as `analyticBrep`; hybrid faceted STEP
must mark `facetedPolyBrep`; mesh-only output must mark `meshNative`.
Consumers must read that representation instead of inferring exactness from
`geometryBackend`.

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
- [x] PG1 A model using `wall-pattern` followed by `difference` renders as a
  single manifold solid with < 100 non-manifold edges.
- [x] PG2 The iPhone 17e case fixture renders as 3 clean parts and retains the
  displaced rear panel.
- [x] PG3 STEP export includes exact faces for non-displaced geometry.
- [x] PG4 STL export is sliceable without manual repair. Verified 2026-07-17
  in Bambu Studio: after automatic plate arrangement, the exported multipart
  STL completed slicing with `Slice ok` and no geometry repair.
- [x] PG5 Models with no mesh-only ops still route through pure OCCT.
- [x] PG6 Models with only mesh ops still route through pure mesh renderer.
- [x] PG7 `cd src-tauri && cargo check` passes.
- [x] PG8 Existing direct-OCCT fixtures still render (no regression).
- [x] PG9 Existing mesh-renderer fixtures still render (no regression).
