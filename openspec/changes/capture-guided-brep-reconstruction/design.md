# Design: Capture-Guided BRep Reconstruction

## Goal

Let a user turn a partial, noisy capture mesh into constrained reconstruction
evidence that an agent can use to author a useful parametric BRep without
pretending the mesh itself contains CAD design intent.

## Current Boundary

The existing capture pipeline owns source photos and reconstructs an immutable
triangle mesh. Capture preview currently supports uniform scale, box crop,
preview, Apply, and Commit. Applying the mesh inserts source-backed
`solidify(import-stl(...))` geometry. This change does not remove that path.

Guided reconstruction is an optional branch before mesh Apply. The reference
mesh may be open, partial, non-manifold, noisy, or include damaged regions. It is
therefore not solidified, mirrored, or exported as the generated CAD part in
this branch.

The existing Three.js viewer already owns raycasting. Guided reconstruction
extends that interaction boundary to emit exact mesh-hit evidence. Backend owns
validation, coordinate transforms, fitting, guide identity, persistence, and
agent handoff. Frontend never becomes geometry authority.

## Decisions

- Capture mesh remains immutable reference geometry.
- `.ecky` remains the only canonical source for generated parametric geometry.
- A `CaptureReconstructionGuide` is a separate versioned artifact, not a hidden
  prompt string and not a mesh mutation.
- Frontend raycasts and presents handles; backend validates anchors and derives
  calibrated coordinates.
- Every surface anchor is guarded by exact source mesh content digest.
- Triangle index plus barycentric coordinates identify a hit only within that
  digest. They are never claimed stable across crop or reconstruction changes.
- Fit-critical measurements require trustworthy metric metadata or explicit
  point-to-point calibration.
- Named landmark roles and constraints carry intent. Unlabeled point clouds do
  not.
- Capture anchors, BRep topology targets, preview-render vertices,
  analysis-boundary vertices, and FEM volume nodes keep separate identities.
  Typed digest-bound correspondences connect them when needed.
- A right-handed local reconstruction frame makes existing axis-aligned `.ecky`
  transforms and `mirror` useful for arbitrary scan orientation.
- Symmetry completes authored BRep features, not reference triangles.
- Agent generation reuses the normal inspect -> validate -> preview -> commit
  lifecycle and cannot bypass source/thread digest guards.
- Scan and BRep overlays are display-only diagnostics and never enter production
  STL or STEP exports.
- Deviation is bounded to observed mesh regions. Missing regions are classified
  as inferred, not verified.
- Rust boundary structs use snake_case with
  `#[serde(rename_all = "camelCase")]`; frontend uses camelCase.
- Major Capture/viewer/overlay containers retain `overflow: hidden` and Tactical
  Midnight square-border styling.

## Layered Reconstruction Stack

Product direction contains 13 logical stages. These are pipeline ownership
boundaries, not necessarily 13 runtime services.

| Stage | Owner | Output | Current state |
| --- | --- | --- | --- |
| 1. Capture acquisition | Capture backend | photos, poses, immutable reconstructed STL | implemented |
| 2. Artifact normalization and identity | artifact services | selected raw/crop mesh, topology summary, content/artifact digests | implemented |
| 3. Exact evidence anchoring | viewer + backend validation | triangle/barycentric anchor, position, normal, semantic role | implemented |
| 4. Neighborhood and uncertainty extraction | deterministic geometry backend | bounded adjacent samples, curvature/noise/coverage estimates, uncertainty | implemented |
| 5. Metric calibration and local frame | deterministic geometry backend | millimetre scale, right-handed frame, local coordinates | implemented |
| 6. Analytic primitive fitting | deterministic geometry backend | plane/line/circle/cylinder/cone/sphere/axis candidates with domains and residuals | partial: line/plane/circle/cylinder candidates wired; cone/sphere fit kernels not yet wired into guide candidates |
| 7. Surface segmentation and adjacency | deterministic geometry backend | evidence regions, boundaries, adjacency graph, ignored/damaged masks | missing |
| 8. Curve and profile reconstruction | deterministic geometry backend | ordered line/arc/spline loops with continuity and fit residuals | partial: user-ordered points only |
| 9. Dimension and constraint graph | deterministic geometry backend | named dimensions, symmetry/coaxial/parallel/tangent/equal relations, tolerances | partial contracts only |
| 10. Feature-plan synthesis | deterministic planner | bounded extrude/revolve/sweep/boolean/mirror candidates with evidence score | missing |
| 11. Semantic selection and `.ecky` authoring | agent bounded by computed candidates | parametric source, named bindings/tags, explicit assumptions | implemented handoff; too much geometric choice remains here |
| 12. Exact BRep execution and correspondence proof | compiler + direct OCCT runtime | valid solids, exact vertices/edges/faces, guide correspondences | implemented |
| 13. Deviation, acceptance, and refinement | deterministic validation + user | observed-region metrics, outliers, inferred regions, accept/revise decision | implemented comparison; iterative fit refinement missing |

Stages 1–10 transform captured evidence into a typed reconstruction problem.
Stage 11 may choose semantic intent among bounded candidates and express it as
`.ecky`; it must not estimate raw coordinates, silently choose unsupported
primitives, or invent missing fit-critical dimensions. Stages 12–13 prove the
authored result against exact runtime topology and observed evidence.

```text
deterministic evidence bundle
  anchors + neighborhoods + uncertainty
  primitive/profile candidates + residuals
  region adjacency
  named dimensions + constraint graph
  bounded feature plans
        |
        v
agent semantic selection + .ecky authoring
        |
        v
exact OCCT proof + observed deviation
```

Missing deterministic stages are visible capability gaps. A guide may support a
simple reconstruction when explicit user evidence fully specifies its profile
and operations, but readiness must state which stages were bypassed and why the
result remains unambiguous. “Agent can probably infer it” is not readiness.

## Rejected Paths

### Direct partial STL solidification

An open or incomplete photogrammetry mesh does not become an analytic design by
wrapping it in `solidify`. That path remains available for ordinary accepted
mesh use but is not reconstruction.

### Mirrored mesh as final geometry

Mirroring can make a visual reference look complete while preserving noise,
holes, thickness errors, and unknown topology. Only authored BRep features may
become manufacturing output.

### Prompt-only point descriptions

Screen coordinates and prose such as "this corner" are not reproducible after
camera movement. Canonical evidence uses source triangle anchors and calibrated
3D coordinates.

### Anonymous points

Four to six arbitrary samples can still describe many incompatible parts. Each
landmark must have a role or belong to a named axis, plane, measurement, or
ordered profile.

### Frontend-owned fitting or source generation

Browser calculations may draw immediate ghosts, but backend validation and
canonical guide serialization are authoritative. Svelte does not write `.ecky`
source or persistence directly.

### Hidden completion heuristics

The agent may propose a reconstruction from visual evidence, but every inferred
region and symmetry assumption remains explicit. It cannot silently manufacture
missing geometry.

## Artifact Model

```text
CaptureReconstructionGuide
  schemaVersion
  guideId
  revision
  captureRunId
  targetThreadId + targetMessageId?
  sourceMesh
    artifactDigest
    contentDigest
    selection: raw | crop
    cropDigest?
    triangleCount
    sourceBounds
  calibration
    sourceUnits
    millimetresPerSourceUnit
    method: knownDistance | trustedMetricMetadata
    measurements[]
    residualMm
  reconstructionFrame
    originMm
    xAxis
    yAxis
    zAxis
    sourceLandmarkIds[]
  landmarks[]
  featureExpectations[]
  measurements[]
  axes[]
  planes[]
  profiles[]
  ignoredRegions[]
  instruction
  evidenceViews[]
  canonicalDigest
```

```text
CaptureSurfaceAnchor
  sourceMeshContentDigest
  triangleIndex
  barycentric[3]
  sourcePosition[3]
  sourceNormal[3]
```

```text
CaptureLandmark
  landmarkId
  label
  role
  anchor
  localPositionMm[3]
  localNormal[3]
  uncertaintyMm?
```

```text
CaptureFeatureExpectation
  expectationId
  guideItemIds[]
  label
  expectedGeometryKind: point | curve | plane | cylinder | profile
  requiredBrepTopologyKind: vertex | edge | face | orderedEdges
  cardinality: one | oneOrMore
  partId
  instancePath?
  expectedAuthoredSelector?: { kind: binding | tag, name }
  requiredForAcceptance
  positionToleranceMm?
  normalToleranceDeg?
  radialToleranceMm?
```

```text
CaptureEvidenceCorrespondence
  expectationId
  guideItemIds[]
  partId
  instancePath?
  authoredSelector: { kind: binding | tag, name }
  selectorCardinality
  brepTargetKind
  canonicalTargetIds[]
  durableTargetIds[]
  sourceStableNodeKeys[]
  sourceGeometryDigest
  relation: observes | constrains | profiles | definesAxis | definesSurface
  residual
  status: satisfied | ambiguous | missing | wrongKind | overTolerance
```

```text
TypedGeometryRef
  captureAnchor: { sourceMeshContentDigest, triangleIndex, barycentric }
  brepTarget: { sourceGeometryDigest, partId, instancePath?, kind, targetId }
  previewRenderVertex: { renderArtifactDigest, vertexIndex }
  analysisBoundaryVertex: { boundaryDigest, vertexIndex }
  femVolumeNode: { volumeMeshDigest, nodeIndex }
```

`TypedGeometryRef` is discriminated union. Decoder rejects fields from another
variant. Every index is meaningful only under owner artifact digest.

These records form correspondence chain, not shared vertex table. Scan point can
observe analytic cylinder whose supporting exact topology is face without
becoming CAD vertex. Profile maps to ordered edge set. Plane maps to supporting
face. BRep face can later discretize into many analysis-boundary vertices and
FEM nodes without derived indices becoming authored identity. Durable target ID
is durable evidence only within recorded `sourceGeometryDigest`; rebuild always
re-resolves it.

Initial roles include:

- calibration endpoint;
- frame origin or direction;
- symmetry sample;
- rotation-axis endpoint;
- profile vertex;
- mating-surface sample;
- bore or cylindrical-feature sample;
- outer extent;
- clearance boundary;
- ignored damaged-region marker;
- generic named reference.

Roles are extensible strings only through a versioned enum/unknown-role failure;
the first implementation does not silently accept an unsupported role.

## Surface Anchors

Frontend obtains the nearest mesh triangle hit from Three.js raycasting. It
records triangle index, normalized barycentric weights, interpolated source
position, and geometric or interpolated normal before display transforms.
Preview scale and camera transforms must not be baked into this source anchor.

Backend reloads the selected raw or cropped mesh and validates:

- content digest matches;
- triangle index is in range;
- barycentric components are finite, within tolerance, and sum to one;
- recomputed source position matches supplied position within numeric tolerance;
- triangle has finite non-zero area;
- normal is finite and consistent with selected triangle orientation;
- calibrated local coordinates remain finite and bounded.

An anchor is durable only while source mesh digest remains unchanged. A changed
crop, reconstruction, or source STL marks the guide stale. Optional nearest-
surface remapping may propose candidates, but each remapped landmark requires
explicit confirmation and records old/new anchors and residual distance.

## Metric Calibration

Current capture scale is a manually adjustable uniform multiplier. Guided BRep
reconstruction makes calibration evidence explicit.

### Known-distance calibration

User selects two distinct landmarks and enters physical distance in millimetres.
Backend derives:

```text
millimetresPerSourceUnit = knownDistanceMm / |sourcePointB - sourcePointA|
```

Multiple measurements use a bounded least-squares uniform scale and report each
measurement residual. Contradictory measurements above configured tolerance
block ready state rather than being averaged silently.

### Trusted metric metadata

A future native depth/pose client may declare metric calibration provenance.
User must see and explicitly accept that provenance before manufacturing-oriented
BRep generation. Provider units without provenance are not trusted metric data.

Changing calibration increments guide revision and deterministically recomputes
all local millimetre positions. Raw mesh bytes never change.

## Local Frame, Axes, And Planes

A reconstruction frame is right-handed and orthonormal:

```text
origin = selected origin evidence
x = normalize(x direction evidence)
y0 = direction toward XY-plane evidence
y = normalize(y0 - dot(y0, x) x)
z = normalize(cross(x, y))
```

Near-zero, coincident, collinear, or non-finite evidence fails with exact reason.
Frontend may render a provisional frame; backend result is authoritative.

Named axes use two or more landmarks. Named planes use three or more landmarks.
For more than minimum samples, backend performs deterministic least-squares fit
and reports maximum and RMS residual. A plane or axis above configured fit
tolerance remains visible but cannot be used as an exact symmetry constraint
until user adjusts evidence or explicitly changes tolerance in guide metadata.

One or two named symmetry planes commonly describe half- or quarter-captured
parts. Plane orientation is expressed in local frame. The generated `.ecky`
model constructs the unique half/quarter feature set and applies explicit mirror
operations. If a plane does not align with local X/Y/Z, the authored geometry
uses a named rigid frame transform before and after axis-aligned mirror rather
than approximating the plane.

## Ordered Profiles And Mechanical Intent

A profile is an ordered list of landmark IDs with:

- stable profile ID and label;
- open or closed kind;
- support plane ID;
- order chosen by user;
- intended operation hint such as extrude, revolve, sweep, reference-only, or
  agent-decide;
- optional feature label and fit role.

The system does not infer profile order from nearest-neighbor distance. The
viewport shows numbered vertices and connecting segments so accidental order is
visible before generation.

For a mating part, the guide should distinguish functional evidence from visual
silhouette: mating planes, bore/axis samples, retaining features, extents, and
clearance boundaries. Requested clearances become named `.ecky` parameters or
constraints. They are never anonymous offsets inserted by the agent.

## Evidence-To-BRep Correspondence

Validation-critical guide evidence declares expected geometry kind and required
BRep topology kind. Axis/cylinder resolves through supporting face/edge analytic
geometry, plane through supporting face, and profile through ordered edge set.
Agent output maps expectation to named authored binding or selector tag. After exact OCCT
preview, backend resolves binding/tag to canonical and durable BRep targets and
records selector cardinality and source provenance.

Residual evaluation follows target kind:

- vertex: calibrated point-to-exact-BRep-vertex distance;
- edge or profile: ordered point/curve-to-exact-edge distance plus endpoint and
  order evidence where applicable;
- face or mating surface: point-to-face distance and normal-angle residual;
- axis/cylinder: point-to-axis radial residual, axis-angle residual, and analytic
  radius residual;
- plane: signed point-to-plane and normal residual.

Nearest whole-shape distance remains broad observed-region diagnostic. It cannot
satisfy validation-critical expectation. Zero targets, wrong kind, multiple
targets for `one`, or over-tolerance residual keeps reconstruction red.
Part ID and optional instance path scope every resolution. Parameter edits
re-resolve correspondences and bind result digest to new target set. Coordinate
similarity and old durable ID cannot silently preserve identity across geometry
digest change.

## User Workflow

1. Open a durable Capture run and choose `GUIDE CAD RECONSTRUCTION`.
2. Select raw or successfully previewed cropped mesh as immutable reference.
3. Calibrate scale with known distance or accept trustworthy metric metadata.
4. Establish local frame or a named rotation axis.
5. Add and label feature landmarks.
6. Create one or more symmetry planes and inspect fit residuals.
7. Order visible profile points where useful.
8. Enter reconstruction instruction, for example:
   "This is a joint insert. Build one quarter, mirror across X and Y, preserve
   the cylindrical mating surface, and expose clearance as a parameter."
9. Review a deterministic guide summary and choose `BUILD CAD FROM GUIDE`.
10. Agent authors `.ecky`, validates, and renders a preview in the owning thread.
11. Capture viewer overlays reference mesh and BRep in the calibrated frame.
12. User inspects residuals and inferred regions, adjusts guide/source, and
    explicitly applies/commits an accepted BRep preview.

Camera orbit never changes guide coordinates. Landmark edits, reorder, delete,
and guide reset are explicit actions. Guide edits do not create model versions;
accepted generated source follows normal history.

## Agent Handoff

`BUILD CAD FROM GUIDE` produces a canonical bounded request:

```text
CaptureGuidedReconstructionRequest
  guideDigest
  guide
  targetSourceDigest
  sourceMesh artifact reference + digest
  canonical orthographic evidence views[]
  userInstruction
  requiredOutput: parametricEckyBrep
```

Evidence views include fixed local-frame front/right/top/isometric cameras and
landmark labels. They are generated from the selected guide revision so labels
and coordinates agree. Source photos may be attached only through explicit user
selection and bounded file rules.

The agent must:

- inspect current target source/version;
- treat mesh as reference-only;
- preserve explicit symmetry and fit constraints in source;
- author named binding/tag for every validation-critical feature expectation;
- name fit-critical dimensions;
- use repeated/mirrored structures rather than copied shape blocks;
- validate and preview before commit;
- record guide digest, source mesh digest, and assumptions in result provenance;
- report unsupported or insufficient evidence instead of emitting arbitrary
  geometry.

The agent may ask one targeted question when ambiguity changes product shape,
for example whether a damaged tab should exist. It does not ask for information
already encoded by guide.

## BRep Reconstruction Result

```text
CaptureGuidedBrepResult
  guideDigest
  sourceMeshContentDigest
  sourceDigest
  parameterDigest
  geometryDigest
  modelId
  artifactDigest
  assumptions[]
  inferredRegions[]
  evidenceCorrespondences[]
  landmarkResiduals[]
  observedDeviationSummary?
  previewState
```

The result is an ordinary generated `.ecky` target with exact BRep/runtime
artifacts. It is not a new geometry kind. If no valid closed solid is produced,
the preview remains failed/red and cannot be committed as completed
reconstruction.

## Overlay And Deviation Evidence

Reference mesh renders as a separately colored translucent ghost. Generated BRep
renders through normal preview. Both use the guide's calibrated frame.

First verification slice reports:

- target-kind residual for every validation-critical evidence correspondence;
- broad nearest-BRep distance only for non-critical generic landmarks;
- axis/plane residuals;
- bounding extents in calibrated frame;
- sampled nearest-surface distance from observed mesh vertices to BRep;
- observed sample count and rejected/outlier count;
- regions declared ignored or damaged;
- regions inferred from symmetry or instruction.

Deviation sampling is deterministic and bounded. It is one-way from observed
mesh to BRep because missing scan surface provides no reverse evidence. The UI
must not label it whole-part accuracy. Unaveraged maximum, RMS, percentile, and
tolerance are shown; smoothing cannot hide outliers.

Overlay meshes, landmarks, labels, axes, planes, and deviation colors are debug/
reference geometry. They are excluded from STL/STEP and cannot alter
manufacturing artifact digests.

## Persistence And Concurrency

Guide metadata is stored through an Ecky-owned capture-run service. No frontend
or agent writes SQLite directly. Mesh/photos remain filesystem artifacts.
Boundary structs use camelCase serialization.

Each save includes expected guide revision, capture run ID, and source mesh
digest. Stale revision or changed mesh rejects update and returns current
identity. Reopening capture restores guide and overlays. Pairing token rotation
does not change guide identity.

`BUILD CAD FROM GUIDE` also guards target source digest and target version. If
source advances while generation runs, preview is retained as a conflict and is
not applied to the newer source automatically.

## Failure Surface

Distinct errors include:

- source mesh missing or digest changed;
- invalid triangle or barycentric anchor;
- uncalibrated physical scale;
- contradictory measurement calibration;
- degenerate frame/axis/plane;
- plane/axis fit above tolerance;
- unsupported landmark role or profile operation hint;
- missing, ambiguous, wrong-kind, or over-tolerance BRep correspondence;
- empty or self-inconsistent profile;
- insufficient evidence for requested reconstruction;
- target source/version divergence;
- generated source compile failure;
- invalid/open BRep result;
- overlay/deviation artifact mismatch.

Raw actionable backend/agent errors remain visible beside responsible guide
item. Last good guide, model, capture mesh, and history remain unchanged.

## BDD Strategy

### Browser happy path

```gherkin
Given a durable partial capture mesh is open
When user calibrates a known distance, defines local frame and two symmetry
planes, labels ordered profile points, and chooses BUILD CAD FROM GUIDE
Then exact digest-bound guide is sent to the owning thread
And agent-produced parametric BRep preview overlays the unchanged reference mesh
And source/history remain uncommitted until explicit Apply/Commit
```

### Degenerate evidence

```gherkin
Given selected frame points are coincident or collinear
When user attempts to mark guide ready
Then backend rejects the frame with exact geometric reason
And previous valid guide and model remain unchanged
```

### Stale mesh

```gherkin
Given guide anchors reference cropped mesh digest A
When user changes crop or reconstruction produces mesh digest B
Then guide becomes stale
And BUILD CAD FROM GUIDE is blocked until explicit remap or confirmation
```

### Honest observed deviation

```gherkin
Given scan observes only one quarter of generated symmetric part
When deviation analysis completes
Then metrics describe only observed samples and landmark residuals
And missing mirrored regions are labeled inferred rather than verified
```

Backend tests cover anchor validation, calibration, frame/plane fitting, canonical
digests, stale revisions, and persistence. Browser tests cover picking, overlay,
profile ordering, happy generation handoff, and stale/degenerate failures. Rust
changes require `cd src-tauri && cargo check`.
