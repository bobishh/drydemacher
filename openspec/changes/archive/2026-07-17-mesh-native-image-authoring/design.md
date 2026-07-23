## Context

Ecky source currently lowers through Surface Scheme -> Core IR -> native OCCT,
build123d, FreeCAD, or Rust mesh execution. Exact authoring covers profiles,
solids, booleans, sweeps, lofts, arrays, hull, and formula-driven radial lofts.
Mesh execution already exists for tessellation, wall patterns, displacement,
lithophanes, structural STL verification, and export. `import-stl -> solidify`
already bridges closed triangle shells back into faceted OCCT BRep.

Missing seam: source cannot carry trusted arbitrary 3D vertices and faces.
`polygon` is a 2D sketch. Image references reach vision models, and image
post-processing changes meshes, but no unified Core IR capability represents
mesh literals or a dimensioned heightfield. Three-view reconstruction accepts
vector `SketchDocument`, not raster contours.

Constraints:

- Keep `.ecky` source deterministic, inspectable, bounded, and AST-patchable.
- Do not execute LLM-authored `bpy` or other arbitrary Python.
- Preserve exact BRep path for normal manufacturing CAD.
- Preserve raw backend/image errors and truthful artifact claims.
- Keep repeated structures procedural; do not normalize generated meshes into
  copied source blocks.
- Use current attachment staging and MCP inspect -> validate -> preview ->
  commit flow. No direct SQLite writes.

## Goals / Non-Goals

**Goals:**

- Author open triangle surfaces and closed polyhedra in `.ecky`.
- Generate large-enough meshes from bounded pure list/formula expressions.
- Produce deterministic topology evidence before render/export claims.
- Generate printable planar heightfields from images.
- Convert clean orthographic raster references into reviewed editable sketch
  intent, then reuse existing reconstruction.
- Reuse existing mesh runtime, verifier, exporters, and poly-BRep bridge.

**Non-Goals:**

- Blender scene authoring, modifiers, UVs, materials, lighting, rigging, or
  animation.
- Arbitrary LLM-generated Python execution.
- Learned single-image depth/mesh generation, NeRF, Gaussian splatting, or
  photogrammetry.
- OBJ/PLY/GLB/FBX authoring or export in this change.
- Automatic exact CAD recovery from one perspective image.
- Analytic STEP reconstruction from arbitrary triangles.
- Cylindrical/spherical heightfield surface syntax; existing displacement and
  lithophane projections remain available.

## Decisions

### 1. Trusted mesh Core IR, not Blender Python

LLM writes a small data/formula form. Compiler and runtime own allocations,
validation, rendering, and export. This preserves source guards, stable AST
patches, backend diagnostics, and deterministic replay.

Rejected: optional raw `bpy` script field. It grants file/process access,
depends on Blender operator context, bypasses typed validation, and cannot
participate honestly in current source/AST/MCP contracts.

### 2. One Core node, two surface contracts

V1 syntax:

```scheme
(mesh
  :vertices ((0 0 0) (10 0 0) (0 10 0))
  :triangles ((0 1 2)))

(polyhedron
  :vertices vertices
  :triangles triangles)
```

Both lower to one `MeshLiteral`-like Core node with `closureRequirement` equal
to `surface` or `closedSolid`. Frontend/Tauri boundary fields use camelCase;
Rust structs use snake_case plus `#[serde(rename_all = "camelCase")]` where
serialized.

Triangles only in v1. N-gons create planarity, self-intersection, and
triangulation policy before core value is proven. Callers can triangulate
procedurally. Later n-gon syntax can lower into the same node.

### 3. Bounded list evaluation precedes allocation

Vertices accept finite point3 lists; triangles accept finite integer triples.
Literal and generated lists share configurable vertex and triangle budgets.
Evaluation counts items incrementally and aborts before building oversized mesh
buffers. Error includes part, operation, observed count, and active limit.

No fixed source expansion occurs. `map`, `range`, helpers, `repeat`, and named
bindings remain source representation for generated meshes.

### 4. Validation has static and topology phases

Static phase, before render:

- required keywords and list item types;
- finite coordinates;
- integer indices in bounds;
- three distinct indices per triangle;
- non-zero triangle area;
- no duplicate oriented/unoriented triangle.

Topology phase:

- edge incidence;
- boundary and non-manifold edges;
- connected components;
- winding consistency/orientability;
- signed volume for closed components.

`mesh` can continue with boundaries; evidence flows into structural
verification. `polyhedron` rejects any boundary/non-manifold edge, inconsistent
winding, zero volume, or multiple components in v1. Runtime does not silently
repair or weld authored topology. Existing STL quantization remains export
behavior, not source mutation.

### 5. Mesh runtime owns direct execution

Pure mesh-literal parts route to existing Rust `IrMesh` evaluation and normal
viewer/STL asset production. Mesh-safe transforms and grouping operate on that
geometry. Mesh literal becomes a mesh-boundary op for partition analysis.

If a validated closed polyhedron feeds a BRep-required consumer, existing
hybrid slicing writes the mesh phase STL, replaces boundary with
`solidify(import-stl(...))`, then runs supported OCCT operations. Open/invalid
meshes fail before OCCT. Hybrid output still must pass current non-manifold and
artifact checks.

No new mesh/BRep bridge abstraction lands until current `poly-brep-bridge`
contracts are reconciled; this change plugs a second source into its proven
handoff.

### 6. STEP truth follows representation

Pure mesh: STL, plus 3MF/multipart STL when part assets exist. No STEP.

Successful solidify: STEP allowed, marked `facetedPolyBrep` with source mesh
digest and topology summary. UI/MCP must not call this analytic, exact-source,
or reconstructed parametric CAD. Unaffected analytic geometry in a hybrid
artifact remains exact, matching current bridge semantics.

### 7. Heightfield is geometry, not free-form post-processing

V1 syntax:

```scheme
(heightfield image_path
  :width 100
  :depth 70
  :relief-height 4
  :base-thickness 1.2
  :invert #f)
```

Raster resolves through existing staged file paths. Decoder converts to
luminance; deterministic grid sampling creates top vertices; triangles connect
grid; side walls and bottom close mesh. Existing lithophane refinement/safe
writer logic should be extracted/reused instead of duplicated.

Empty image parameter is frontend `pending`, not backend success and not fake
geometry. Decode failure returns raw path/decoder context and preserves last
good preview.

### 8. Image routes remain semantically separate

```text
reference photo -> vision LLM -> inferred .ecky -> normal validation
raster heightmap -> heightfield -> deterministic closed mesh
orthographic raster -> contour candidates -> reviewed SketchDocument
                    -> existing preview hull/candidate BRep acceptance
```

No route labels vision output as reconstruction. No raster contour bypasses
review or accepted-CAD gates.

### 9. Orthographic extraction feeds SketchDocument

Each Front/Top/Side pane gains optional reference image, physical calibration,
threshold/inversion controls, extraction evidence, and contour selection.
Extraction is bounded classical image processing suitable for clean line art:
grayscale -> threshold -> connected contours -> simplify -> closed candidate
validation. Selected contour becomes a polyline primitive with provenance:

```text
kind: rasterTrace
asset identity/digest
view
calibration
threshold/invert
contour id
extractor version
```

User edits normal sketch points after selection. Existing candidate-cell,
projection replay, exact candidate acceptance, and hidden-line validation stay
unchanged consumers.

### 10. Stable draft owns image extraction state

Raster selection and extraction update existing sketch draft. No new thread,
version, or temporary history record. Save/restore persists asset identity and
settings through existing backend draft command; frontend never writes SQLite.
Failed extraction preserves reviewed sketch and last good preview.

### 11. UI stays shell, Rust owns geometry truth

Frontend owns file selection, preview overlays, calibration input, contour
choice, and status rendering. Rust owns image decoding, contour extraction
contract, mesh generation, topology validation, render dispatch, and artifact
metadata. Major Sketch Workspace containers retain `overflow: hidden`; controls
follow Tactical Midnight square-border theme.

## Risks / Trade-offs

- **Large LLM mesh payloads** -> enforce pre-allocation budgets; prefer formula
  generation and image operations.
- **Triangle syntax less ergonomic than Blender n-gons** -> deterministic v1;
  add n-gon lowering only with explicit triangulation proof.
- **Poly-BRep STEP can be large and slow** -> gate solidification, preserve STL,
  report faceted provenance and counts.
- **Raster contours depend on threshold quality** -> expose controls and raw
  evidence; require human review; preserve last good sketch.
- **Single images remain ambiguous** -> label vision result inferred; require
  scale/dimensions for manufacturing claims.
- **Mesh topology validation cost** -> linear edge maps with bounded input;
  no quadratic repair pass.
- **Open meshes visible but unprintable** -> allow preview, fail structural
  printability and BRep consumers explicitly.

## Migration Plan

1. Add syntax/Core node behind backend capability manifest entries.
2. Land compiler/topology validation with rendering disabled until integration
   red/green proof passes.
3. Enable pure mesh dispatch and STL artifacts.
4. Enable poly-BRep handoff only for closed validated polyhedra.
5. Add heightfield using existing image infrastructure.
6. Add orthographic raster references and stable-draft persistence.
7. Update LLM/MCP prompts and language reference after runtime capabilities
   report support.

Rollback: remove capability entries and dispatch admission. Existing `.ecky`
source remains unchanged; new forms fail with deterministic unsupported-op
diagnostics rather than falling back silently.

## Open Questions

- Default vertex/triangle budgets: choose from live memory/render benchmarks,
  then expose resolved values in capability metadata.
- Whether multiple closed components should become valid `polyhedron` compound
  in v2; v1 requires one component.
- Whether raster asset identity uses existing attachment digest directly or a
  project-mirror asset id once filesystem mirroring lands.
- Whether contour extraction belongs in a new Rust module or shared sketch
  command module; boundary contract remains same.
