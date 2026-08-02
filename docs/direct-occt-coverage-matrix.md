# Direct OCCT Coverage Matrix

Status terms:

- `direct`: planned and executed by Direct OCCT.
- `runner-supported`: accepted by current precompiled runner gate.
- `normalized-direct`: rewritten by Rust normalizer into direct operations.
- `mesh-only`: intentionally handled by Rust mesh path, not BREP.
- `unsupported`: deterministic Direct OCCT rejection.
- `gap`: missing proof or unclear behavior.

## Core Operation Coverage

| Group | Operation | Status | Evidence | Notes |
| --- | --- | --- | --- | --- |
| primitive | `box` | direct | planner + live box export tests | BREP solid |
| primitive | `sphere` | direct | solid ops live test | BREP solid |
| primitive | `cylinder` | direct | solid ops live test | BREP solid |
| primitive | `cone` | direct | cone planner/live coverage | BREP solid |
| primitive | `circle` | direct | extrude/revolve/sweep tests | sketch profile |
| primitive | `rectangle` | direct | extruded sketch tests | sketch profile |
| primitive | `rounded-rect` | direct | rounded rectangle planner test | sketch profile |
| primitive | `rounded-polygon` | direct | rounded polygon planner test | sketch profile |
| primitive | `polygon` | direct | profile/SVG/path tests | sketch profile |
| primitive | `profile` | direct | profile holes live test | outer loop plus holes |
| primitive | `make-face` | direct | make-face planner test | face creation |
| primitive | `svg` | normalized-direct | SVG profile planner + live export tests | vector paths only |
| primitive | `text` | mesh-only | normalizer rejection test | no BREP text path yet |
| primitive | `stl` | mesh-only | normalizer rejection test | mesh import, not BREP |
| boolean | `union` | direct | solid ops/live boolean tests | BREP boolean |
| boolean | `difference` | direct | solid ops/live boolean tests | BREP boolean |
| boolean | `intersection` | direct | solid ops/live boolean tests | BREP boolean |
| boolean | `xor` | unsupported | planner/normalizer rejection tests | explicit unsupported |
| transform | `translate` | direct | transform live test | BREP transform |
| transform | `rotate` | direct | transform live test | BREP transform |
| transform | `scale` | direct | transform live test | BREP transform |
| transform | `mirror` | direct | mirror live/planner tests | axis-limited |
| surface | `extrude` | direct | extrude/profile/SVG tests | BREP surface/solid |
| surface | `revolve` | direct | revolve live test | BREP solid |
| surface | `loft` | direct | loft live test | BREP solid |
| surface | `sweep` | direct | sweep/bezier sweep live tests | BREP solid |
| surface | `shell` | direct | shell live tests | BREP shell |
| surface | `offset` | direct | offset sketch live test | sketch/shape offset |
| surface | `offset-rounded` | direct | mirror/taper/offset-rounded live test | emitted as offset |
| surface | `fillet` | direct | fillet/chamfer live test | target-id selectors supported |
| surface | `chamfer` | direct | fillet/chamfer live test | target-id selectors supported |
| surface | `taper` | direct | taper live test | BREP transform-like op |
| surface | `twist` | direct | twist live test | BREP generated op |
| surface | `draft` | direct | draft planner and runner live tests | side-wall face draft, runner-supported |
| path | `polyline` | direct | path frame/sweep tests | emitted as path |
| path | `bezier-path` | direct | bezier sweep live test | cubic-control validation |
| path | `bspline` | direct | bspline profile live test | closed/open profile usage |
| array | `linear-array` | direct | array ops live test | BREP copies |
| array | `radial-array` | direct | array ops live test | BREP copies |
| array | `grid-array` | direct | array ops live test | BREP copies |
| array | `arc-array` | direct | array ops live test | BREP copies |
| array | `repeat` | normalized-direct | normalizer tests | finite expansion |
| array | `repeat-union` | normalized-direct | normalizer tests | expands to union |
| array | `repeat-compound` | normalized-direct | normalizer tests | expands to group/compound |
| array | `repeat-pick` | normalized-direct | normalizer tests | finite selection |
| frame | `plane` | direct | plane/location/clip-box live test | frame primitive |
| frame | `location` | direct | plane/location/clip-box live test | frame placement |
| frame | `path-frame` | direct | path-frame/place live test | path placement |
| frame | `place` | direct | path-frame/place live test | placement op |
| frame | `clip-box` | direct | plane/location/clip-box live test | clipped BREP |
| meta | `group` | direct | multi-part/compound tests | emitted as compound |
| meta | `comment` | unsupported | normalizer/planner unsupported branch | rejected by operation name |
| meta | `annotate` | unsupported | normalizer/planner unsupported branch | rejected by operation name |
| custom | `sampled-radial-loft` | normalized-direct | sampled-radial-loft live tests | native and FreeCAD (not mesh) |
| custom | `hull` | direct | hull capsule runner live tests | native-only convex hull; FreeCAD rejects |
| custom | `helical-ridge` | normalized-direct | native helical-ridge render tests; FreeCAD lowering tests | planner-expanded into helix sweep + boolean forms |
| custom | `hole` | unsupported | typed-hole rejection test | must be filled before planning |
| custom | `wall-pattern` | mesh-only (hybrid-bridged) | mesh path tests + hybrid poly BRep tests | Rust mesh-only op; hybrid bridge routes to OCCT solidify + boolean when followed by BRep ops |
| custom | `pattern` | mesh-only (hybrid-bridged) | source classifier | legacy mesh alias; hybrid bridge applies when followed by BRep ops |
| custom | `mesh` | mesh-only | mesh literal runtime/topology/render tests | open triangle surface; never solidified when boundary evidence is nonzero |
| custom | `polyhedron` | mesh-only (hybrid-bridged) | closed tetrahedron + live hybrid boolean tests | closed typed triangle solid; STEP after solidify is faceted poly-BRep |
| custom | `heightfield` | mesh-only | deterministic image/closed-STL tests | bounded luminance sampling; closed relief mesh; STL only unless later hybrid-consumed |
| custom | `solidify` | normalized-direct | hybrid poly BRep tests | sew + make solid; enables booleans on imported meshes |
| custom | other custom ops | unsupported | normalizer rejection test | deterministic diagnostic |

## Hybrid Poly BRep Bridge

When a part uses mesh-only ops (`wall-pattern`, `polyhedron`, or `heightfield`)
followed by BRep-required ops (`difference`, `chamfer`, `fillet`), render dispatch uses the
hybrid poly BRep bridge:

1. **Partition analysis** classifies the part as `Hybrid`.
2. **Exact prelude**: OCCT tessellates chamfer/fillet inputs when a mesh-only
   op consumes them.
3. **Mesh phase**: each independent mesh island emits a validated,
   engine-independent `MeshAsset` STL. Internal displacement, imported STL,
   and typed LLM-generated `polyhedron` use the same contract.
4. **OCCT phase**: mesh output nodes become
   `solidify(import-stl(asset.stl))`; post-boundary booleans execute on the
   solidified poly BRep.

This avoids the 30k+ non-manifold edges the mesh renderer produces on CSG over
displaced meshes, while preserving exact BRep boolean precision from OCCT.

The `solidify` op extracts the shared shell produced by `StlAPI_Reader`, then
uses `BRepBuilderAPI_MakeSolid`. Sewing is intentionally avoided because it
collapsed dense faceted inputs during the iPhone regression proof.

Pure OCCT models (no mesh ops) and pure mesh models (no post-boundary BRep ops)
are unaffected — they use existing paths with zero regression.

## Open Gaps

- FreeCAD-only interop operations outside Direct OCCT path: `text`, `xor`.
- Typed `hole` placeholders still must be filled before Direct OCCT planning.
- Unsupported runner plans fail explicitly. No generated-C++ fallback exists.

## Current Runner Subset

Runner-first dispatch is enabled only when each command matches the current
proven runner subset:

| Operation | Runner status | Notes |
| --- | --- | --- |
| `box` | runner-supported | solid primitive |
| `sphere` | runner-supported | solid primitive |
| `cylinder` | runner-supported | solid primitive |
| `cone` | runner-supported | solid primitive |
| `circle` | runner-supported | sketch primitive |
| `rectangle` | runner-supported | sketch primitive |
| `rounded-rect` | runner-supported | sketch primitive |
| `rounded-polygon` | runner-supported | sketch primitive |
| `polygon` | runner-supported | sketch primitive |
| `profile` | runner-supported | positional outer profile or `:outer` / `:holes` arg keywords |
| `make-face` | runner-supported | face creation |
| `extrude` | runner-supported | keyword-free profile/face extrude |
| `revolve` | runner-supported | keyword-free profile revolve |
| `loft` | runner-supported | keyword-free profile loft |
| `sweep` | runner-supported | keyword-free profile/path sweep |
| `twist` | runner-supported | keyword-free profile twist |
| `taper` | runner-supported | keyword-free profile taper |
| `offset` | runner-supported | keyword-free sketch offset |
| `path` | runner-supported | polyline path |
| `bezier-path` | runner-supported | cubic Bezier path |
| `bspline` | runner-supported | sketch profile |
| `plane` | runner-supported | keyword-free frame primitive |
| `location` | runner-supported | keyword-free frame placement |
| `path-frame` | runner-supported | keyword-free path placement |
| `place` | runner-supported | keyword-free frame placement |
| `clip-box` | runner-supported | `:x`, `:y`, `:z` numeric arg keywords |
| `fillet` | runner-supported | all edges keyword-free, `:edges "all"`, exact `:edges` target ids, and coarse edge clauses |
| `chamfer` | runner-supported | all edges keyword-free, `:edges "all"`, exact `:edges` target ids, and coarse edge clauses |
| `shell` | runner-supported | keywordless default shell, exact `:faces` target ids, and face clauses using `boundary` / `planar` / `normal` / `area` |
| `linear-array` | runner-supported | transform array |
| `radial-array` | runner-supported | transform array |
| `grid-array` | runner-supported | transform array |
| `arc-array` | runner-supported | transform array |
| `union` | runner-supported | BREP boolean |
| `difference` | runner-supported | BREP boolean |
| `intersection` | runner-supported | BREP boolean |
| `translate` | runner-supported | transform |
| `rotate` | runner-supported | transform |
| `scale` | runner-supported | transform |
| `mirror` | runner-supported | transform |
| `compound` | runner-supported | grouping output |
| `draft` | runner-supported | keyword-free or `:neutral-z`/`:neutral_z` numeric keyword |
| `hull` | runner-supported | variadic shape refs; incremental 3-D convex hull added 2026-07-09 |

Every other Direct OCCT op is rejected by the runner with an explicit unsupported-plan error.

## Parity Against Exact Lowerings

This section answers a narrower question than “can Direct OCCT plan it?”:
whether the current `EckyRust -> runner-first` path covers forms that FreeCAD
interop can render.

| Form | FreeCAD | Direct OCCT runner | Notes |
| --- | --- | --- | --- |
| primitives, profiles, booleans, transforms, arrays, frames | yes | covered | direct BREP |
| selector `fillet` / `chamfer` / `shell` | yes | covered | runner-first proven |
| `sampled-radial-loft`, `helical-ridge` | yes | covered | normalized into runner operations |
| `hull` | no | covered | native-only |
| `text`, `xor` | yes | unsupported | FreeCAD interop only |
| typed `hole` placeholders | rejected until filled | rejected until filled | authoring placeholder, not runtime op |

### Practical reading

- For the shared BREP subset, `EckyRust -> Direct OCCT -> precompiled runner`
  now covers the common primitives, booleans, transforms, arrays, frames,
  profile/SVG workflows, and the supported selector-driven `fillet` /
  `chamfer` / `shell` flows.
- The runner is the only native BREP executor. Unsupported plans are rejected;
  they do not compile generated C++.

## Runner Boundary

- `Core IR -> OcctPlan/plan.json -> precompiled direct-occt-runner -> OCCT` is
  the sole native BREP path.
- UI/MCP must surface unsupported-plan errors instead of selecting a hidden
  fallback.
