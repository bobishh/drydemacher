## Why

Ecky can already author exact BRep CAD from text, tessellate it for printing,
displace meshes from images, and reconstruct bounded solids from vector
orthographic sketches. It cannot yet express an arbitrary trusted triangle mesh
in `.ecky`, and its image workflows remain separate paths rather than one
truthful image-to-geometry authoring surface.

Adding mesh-native Core IR closes that representation gap without executing
untrusted LLM-written Blender Python. Connecting deterministic heightfields and
raster orthographic tracing to existing preview, verification, and poly-BRep
systems gives image-to-3D useful semantics while keeping approximate inference
distinct from accepted CAD.

## What Changes

- Add `.ecky` surface forms for bounded triangle-mesh authoring:
  `mesh` for open or closed triangle surfaces and `polyhedron` for closed,
  orientable, manifold solids.
- Add one typed mesh-literal Core IR representation shared by both forms, with
  deterministic validation, topology analysis, resource budgets, mesh-runtime
  execution, AST visibility, and language-reference coverage.
- Add a planar `heightfield` operation that converts a referenced raster into a
  dimensioned, closed relief mesh using existing image/mesh infrastructure.
- Route mesh-native results through existing STL/3MF paths. Gate STEP on a
  successful watertight `solidify` bridge and label it faceted poly-BRep, never
  analytic CAD.
- Add per-view raster references to Sketch Workspace. Trace high-contrast
  front/top/side contours into an editable, provenance-bearing
  `SketchDocument`; require review before existing candidate reconstruction and
  accepted-CAD gates run.
- Keep vision-assisted reference-image generation as source inference: the LLM
  authors `.ecky`, then normal compile, preview, and verification decide artifact
  truth.
- Expose raw topology and image-decoding failures. Never replace them with
  generic image/API guidance.
- Keep literal mesh payloads bounded and encourage generated point/triangle
  lists for repeated or formula-driven geometry.

No existing syntax changes. No breaking API change.

## Capabilities

### New Capabilities

- `mesh-native-authoring`: Trusted triangle-mesh and closed-polyhedron authoring,
  validation, rendering, verification, hybrid solidification, and export truth.
- `image-geometry-authoring`: Deterministic raster heightfields, truthful
  vision-reference inference, and raster orthographic contour extraction.

### Modified Capabilities

- `sketch-preview-draft`: Raster-derived orthographic contours participate in
  the existing stable sketch draft lifecycle with explicit provenance, review,
  pending, and failure states.

## Impact

- Language/compiler: `src-tauri/src/ecky_scheme`, `ecky_ir`,
  `ecky_language_surface`, backend capability manifests, AST/source maps.
- Geometry runtime: Rust mesh evaluator, topology verification, render dispatch,
  poly-BRep partition/solidify bridge, artifact manifests and export gating.
- Image runtime: existing displacement/lithophane image decoding and mesh
  refinement, plus bounded planar heightfield generation.
- UI: Sketch Workspace per-view reference images, contour review/editing, raw
  validation evidence, and current Tactical Midnight controls.
- MCP/agents: language reference and inspect/validate/preview/commit guidance;
  no new arbitrary-code execution tool.
- Tests: compiler and topology units, backend live renders, differential bridge
  checks, MCP artifact assertions, and Playwright happy plus pending/failure UI
  flows.
- Coordination: builds on `poly-brep-bridge`; does not replace or duplicate its
  `import-stl -> solidify` boundary.
