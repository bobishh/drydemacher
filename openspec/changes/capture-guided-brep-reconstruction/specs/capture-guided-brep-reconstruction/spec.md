# Capture-Guided BRep Reconstruction Specification

## ADDED Requirements

### Requirement: Capture Mesh Remains Reference Geometry

The system SHALL keep the selected raw or cropped capture mesh immutable and
reference-only during guided BRep reconstruction. Generated manufacturing
geometry SHALL come from validated `.ecky` source and SHALL NOT be a solidified,
mirrored, patched, or otherwise disguised copy of the reference triangle mesh.

#### Scenario: User starts guided reconstruction

- **GIVEN** a durable capture run has a valid raw or successfully previewed
  cropped mesh
- **WHEN** user chooses guided CAD reconstruction
- **THEN** system binds guide to exact capture-run and mesh content digests
- **AND** mesh renders as reference geometry
- **AND** no `.ecky` source, history, STL, or STEP artifact changes

#### Scenario: Reference mesh is partial or open

- **GIVEN** selected mesh contains only a half or quarter of physical part
- **WHEN** guided reconstruction begins
- **THEN** system allows it as incomplete evidence
- **AND** does not claim the mesh is a closed BRep or manufacturing solid
- **AND** unknown regions remain explicit until authored from user constraints or
  confirmed assumptions

#### Scenario: User retains ordinary mesh workflow

- **WHEN** user chooses existing mesh Apply instead of guided reconstruction
- **THEN** existing raw/cropped mesh lifecycle remains available
- **AND** no reconstruction guide is required or silently generated

### Requirement: Digest-Bound Surface Landmarks

The system SHALL record each picked scan point as a surface anchor bound to exact
source mesh content digest, triangle index, barycentric coordinates, source
position, and source normal. Camera and display transforms SHALL NOT become
canonical anchor coordinates.

#### Scenario: User clicks reference mesh

- **WHEN** raycaster hits a valid source triangle in landmark mode
- **THEN** frontend records triangle and barycentric hit evidence before display
  transforms
- **AND** backend recomputes and validates source position and normal against
  exact mesh bytes
- **AND** persistent numbered landmark appears at validated position

#### Scenario: Hit evidence is invalid

- **WHEN** mesh digest differs, triangle is missing/degenerate, barycentric values
  are invalid, or recomputed position exceeds tolerance
- **THEN** backend rejects anchor with exact reason
- **AND** failed candidate source and raw reason are appended as the new `head`
- **AND** last valid guide/model remain the successful projection

#### Scenario: User orbits camera

- **GIVEN** one or more landmarks exist
- **WHEN** camera moves without guide edit
- **THEN** landmark source and local coordinates remain unchanged
- **AND** overlays continue to track same 3D anchors

### Requirement: Explicit Metric Calibration

The system SHALL derive physical scale from one or more user-supplied known
distances or explicitly accepted trustworthy metric capture metadata. Provider
unit labels and initial preview multipliers SHALL NOT silently establish
manufacturing scale.

#### Scenario: User calibrates known distance

- **GIVEN** two distinct digest-bound landmarks
- **WHEN** user enters finite positive physical distance in millimetres
- **THEN** backend derives uniform source-to-millimetre scale
- **AND** guide records endpoints, entered distance, scale, residual, and revision
- **AND** calibrated bounds and landmark coordinates update without modifying
  mesh bytes

#### Scenario: Multiple measurements disagree

- **GIVEN** multiple known-distance measurements
- **WHEN** their fitted scale residual exceeds configured tolerance
- **THEN** guide remains unready
- **AND** each residual and accepted tolerance are visible
- **AND** system does not silently average contradictory evidence into success
- **AND** the candidate containing contradictory evidence remains an appended
  version at `head`

#### Scenario: Native capture supplies metric provenance

- **GIVEN** client provides versioned trustworthy metric calibration provenance
- **WHEN** user explicitly accepts it
- **THEN** guide records method and provenance
- **AND** Safari/provider unit labels without equivalent provenance remain
  untrusted

### Requirement: Deterministic Reconstruction Frame And Constraints

The system SHALL construct a finite right-handed orthonormal local frame from
explicit evidence and SHALL support named axes and planes with deterministic fit
residuals. Degenerate or over-tolerance evidence SHALL NOT become an exact
constraint.

#### Scenario: User defines local frame

- **GIVEN** calibrated origin, X-direction, and XY-plane evidence is valid
- **WHEN** backend constructs reconstruction frame
- **THEN** frame is right-handed and orthonormal
- **AND** all local landmark coordinates derive deterministically in millimetres
- **AND** frontend displays authoritative origin and axes

#### Scenario: Frame evidence is degenerate

- **WHEN** frame evidence is coincident, collinear, ill-conditioned, non-finite,
  or cannot form right-handed frame
- **THEN** backend rejects frame with exact geometric reason
- **AND** invalid candidate version becomes `head` with raw geometric reason
- **AND** previous valid frame remains the successful projection

#### Scenario: User fits symmetry plane

- **GIVEN** three or more named surface landmarks
- **WHEN** user defines symmetry plane
- **THEN** backend derives deterministic plane orientation and reports RMS/max
  residuals
- **AND** plane becomes exact symmetry constraint only when fit satisfies
  configured tolerance

#### Scenario: User defines half or quarter completion

- **GIVEN** one or two valid symmetry planes
- **WHEN** user requests completion from visible half or quarter
- **THEN** guide records explicit planes and intended completion
- **AND** system does not mirror reference triangles into manufacturing output

### Requirement: Semantic Mechanical Evidence

The system SHALL let each landmark participate in a named mechanical role,
measurement, axis, plane, ignored region, or ordered profile. It SHALL NOT treat
an unlabeled point collection as sufficient design intent.

#### Scenario: User marks mating evidence

- **WHEN** user labels points or samples as mating surface, bore/axis, outer
  extent, clearance boundary, profile vertex, damage marker, or generic named
  reference
- **THEN** guide persists supported role and label
- **AND** generated source provenance can trace resulting feature or assumption
  to those guide items

#### Scenario: User creates profile

- **GIVEN** valid calibrated landmarks and support plane
- **WHEN** user explicitly orders profile points and selects open/closed kind
- **THEN** viewport renders numbered order and connecting segments
- **AND** guide preserves explicit order and operation hint
- **AND** backend does not reorder profile by nearest-neighbor inference

#### Scenario: Fit requires clearance

- **WHEN** user requests mating clearance or offset
- **THEN** guide records its role and requested value
- **AND** generated `.ecky` represents it through named parameter, binding, or
  constraint
- **AND** anonymous fit-critical geometry offset is rejected

### Requirement: Typed Evidence-To-BRep Correspondence

The system SHALL keep capture anchors, BRep topology targets, preview-render
vertices, analysis-boundary vertices, and FEM volume nodes as distinct opaque
digest-bound entity kinds. Cross-kind decoding SHALL fail. Every
validation-critical guide item SHALL declare expected geometry kind, required
BRep topology kind, cardinality, part/instance scope, and discriminated authored
binding-or-tag selector. Coordinate equality or nearest whole-shape distance
MUST NOT substitute for identity.

#### Scenario: Scan point expects exact mating face

- **GIVEN** calibrated landmark declares one mating-face expectation
- **WHEN** generated `.ecky` preview resolves its named authored tag
- **THEN** result records guide item, authored binding/tag, exact canonical and
  durable face target IDs, source stable-node provenance, and geometry digest
- **AND** validation reports point-to-face and normal-angle residuals
- **AND** scan landmark ID is not reused as face, mesh-vertex, or FEM-node ID

#### Scenario: Analytic expectation resolves supporting topology

- **WHEN** guide expects cylinder/axis, plane, or profile geometry
- **THEN** correspondence resolves respectively through supporting exact
  face/edge, face, or ordered edge-set topology
- **AND** analytic residual and topology provenance are both recorded

#### Scenario: Ordered profile expects exact edge

- **GIVEN** ordered guide profile declares one-or-more edge expectation
- **WHEN** exact preview is validated
- **THEN** result records ordered profile-to-edge correspondence and residuals
- **AND** backend does not satisfy profile from unrelated nearest whole-shape
  surfaces

#### Scenario: Expected target is missing or ambiguous

- **WHEN** binding/tag resolves zero targets, wrong entity kind, multiple targets
  for `one`, stale geometry, or over-tolerance residual
- **THEN** correspondence status names exact failure and candidate provenance
- **AND** reconstruction remains red and cannot be accepted as complete

#### Scenario: Parameter edit rebuilds topology

- **WHEN** accepted source parameter changes and exact BRep rebuilds
- **THEN** every validation-critical correspondence re-resolves against new
  geometry digest
- **AND** coordinate proximity alone cannot preserve prior target identity
- **AND** old durable target IDs are evidence only for old geometry digest

#### Scenario: Entity reference crosses artifact kind

- **WHEN** request supplies preview-render vertex, analysis-boundary vertex, or
  FEM node where BRep target is required, or omits owner artifact digest
- **THEN** typed decoding/validation rejects reference before geometry matching

### Requirement: Append-Only Versioned And Persistent Reconstruction Guide

The system SHALL persist an append-only deterministic reconstruction guide and
every distinct source/draft/candidate change through Ecky-owned capture-run
services before validation. Valid, invalid, failed, and stale candidates SHALL
remain addressable versions. Appends SHALL be serialized; stale expected
revision or changed source digest SHALL never reject an append or create a
conflict. `head` SHALL always identify the latest appended version. Successful
versions MAY be filtered for rendering/application as a projection only.
Frontend and agents SHALL NOT write SQLite directly.

#### Scenario: Historical capture reopens

- **GIVEN** capture run has a saved guide
- **WHEN** user reopens it from task history
- **THEN** guide, selected mesh identity, calibration, landmarks, constraints,
  profiles, and overlays restore
- **AND** rotated pairing credentials do not change guide identity

#### Scenario: Concurrent guide edit occurs

- **WHEN** append carries stale expected revision
- **THEN** backend serializes and appends the candidate as a new version
- **AND** `head` advances to that version
- **AND** no version conflict or refusal is returned

#### Scenario: Failed candidate is retained

- **WHEN** a changed guide/source/draft fails validation, compile, or exact BRep
  verification
- **THEN** backend appends it before validation and advances `head`
- **AND** exact candidate source plus raw backend/agent evidence are retained
- **AND** successful-version filtering may keep the last successful render as a
  projection without deleting or rewinding the failed version

#### Scenario: Source mesh changes

- **GIVEN** guide anchors reference mesh digest A
- **WHEN** reconstruction, crop, or selected mesh changes to digest B
- **THEN** the new candidate appends and becomes `head`
- **AND** old/new source identities and stale/remap evidence are retained
- **AND** no conflict or version loss occurs; optional remap still requires
  explicit confirmation before treating anchors as remapped

### Requirement: Deterministic Reconstruction Evidence Stack

Before agent authoring, the system SHALL transform exact anchors into a typed,
digest-bound reconstruction evidence bundle. For every supported shape class,
backend computation SHALL own neighborhood extraction, uncertainty, analytic
primitive/profile fits, dimensions, constraint relations, candidate domains,
and residuals. Sparse point coordinates or prose alone SHALL NOT establish
reconstruction readiness.

#### Scenario: Surface neighborhood supports analytic candidates

- **GIVEN** one or more semantic anchors reference a valid source mesh digest
- **WHEN** guide requests plane, line, circle, cylinder, cone, sphere, or profile
  evidence supported by current capability
- **THEN** backend extracts a bounded deterministic neighborhood around source
  anchors
- **AND** records sample provenance, coverage, uncertainty, fit domain,
  candidate parameters, maximum/RMS residuals, and rejected hypotheses
- **AND** agent does not recompute those parameters from screenshots or prose

#### Scenario: Ordered profile contains curved evidence

- **GIVEN** ordered profile landmarks and source neighborhoods
- **WHEN** backend reconstructs supported profile candidates
- **THEN** result distinguishes line, arc, circle, and spline segments
- **AND** records closure, continuity, support plane, parameter ranges, residuals,
  and source evidence IDs
- **AND** a raw polyline is not silently promoted to exact design intent

#### Scenario: Evidence yields several feature plans

- **GIVEN** primitive, profile, region-adjacency, and constraint evidence permits
  materially different extrude/revolve/sweep/boolean plans
- **WHEN** deterministic planner scores candidates
- **THEN** handoff contains bounded candidates with supporting/rejecting evidence
- **AND** semantic agent may select only a supported candidate or request user
  confirmation
- **AND** it cannot invent a new fit-critical dimension or hidden feature plan

#### Scenario: Deterministic stage is unavailable

- **WHEN** required neighborhood, primitive, segmentation, profile, constraint,
  or feature-plan capability is missing or ambiguous
- **THEN** guide readiness names missing stage and affected evidence IDs
- **AND** system allows bypass only when explicit user constraints fully specify
  result and recorded proof shows no material ambiguity
- **AND** “agent inference” alone cannot satisfy readiness

### Requirement: Canonical Guided Agent Handoff

The system SHALL build a bounded canonical reconstruction request from exact
guide revision, target source/version, source mesh artifact identity, user
instruction, and deterministic evidence views. The request SHALL require
parametric `.ecky` BRep output and SHALL NOT request automatic mesh
solidification.

#### Scenario: User builds CAD from valid guide

- **GIVEN** guide has valid mesh identity, metric calibration, local frame,
  supported evidence, and reconstruction instruction
- **WHEN** user chooses `BUILD CAD FROM GUIDE`
- **THEN** backend sends exact guide digest and target source/version to owning
  thread
- **AND** request includes computed uncertainty, supported primitive/profile
  candidates, constraint graph, and bounded feature-plan candidates
- **AND** request includes bounded local-frame evidence views and reference mesh
  identity
- **AND** agent follows inspect -> validate -> preview -> explicit commit
  lifecycle

#### Scenario: Guide lacks sufficient evidence

- **WHEN** requested shape cannot be constrained by guide and visual evidence
- **THEN** system keeps request pending or asks targeted confirmation
- **AND** agent does not emit arbitrary geometry or hidden assumption

#### Scenario: Target source advances during generation

- **WHEN** target source/version differs from guarded request identity
- **THEN** generated candidate appends and becomes `head`
- **AND** old/new identities are visible as source-divergence metadata
- **AND** no conflict/refusal is emitted and no candidate is discarded

### Requirement: Parametric BRep Preserves Design Intent

Generated reconstruction SHALL be ordinary `.ecky` source lowered through exact
BRep runtime. Symmetry, repeats, mating dimensions, and clearances SHALL be
represented explicitly rather than baked into copied geometry or tessellation.

#### Scenario: Guide describes quarter-symmetric insert

- **WHEN** agent reconstructs part
- **THEN** source authors unique quarter features and explicit symmetry
  operations
- **AND** source does not contain four copy-pasted shape blocks
- **AND** generated geometry does not include mirrored reference STL

#### Scenario: Symmetry plane is not aligned to scan world axes

- **WHEN** guide frame aligns valid plane to local reconstruction axis
- **THEN** source uses named frame transform and supported mirror operation
- **AND** no approximate plane or anonymous offset is introduced

#### Scenario: Generated expected solid is invalid

- **WHEN** source fails compilation, exact runtime yields open/invalid expected
  solid, or fit-critical bindings are unresolved
- **THEN** preview remains failed/red
- **AND** system does not mark reconstruction complete or accept it as the
  successful projection
- **AND** failed candidate version remains at `head` with exact source/raw reason
- **AND** raw compiler/runtime reason is visible

### Requirement: Honest Reference Overlay And Deviation

The system SHALL overlay reference mesh and generated BRep in same calibrated
frame and SHALL report deviations only for observed scan evidence. Overlay and
diagnostic geometry SHALL never alter production artifacts or their digests.

#### Scenario: Generated BRep preview is available

- **WHEN** user opens reconstruction comparison
- **THEN** reference mesh renders as independently controllable translucent ghost
- **AND** BRep renders through normal preview path
- **AND** landmarks, axes, planes, profiles, and inferred regions remain visible
  as optional diagnostics

#### Scenario: Partial scan covers one quarter

- **WHEN** deviation analysis compares observed mesh samples to BRep
- **THEN** system reports sample count, outlier count, maximum, RMS, percentile,
  tolerance, and landmark residuals for observed evidence
- **AND** missing mirrored regions are labeled inferred/unverified
- **AND** metrics are not described as whole-part accuracy

#### Scenario: User exports reconstructed part

- **WHEN** STL or STEP export is generated
- **THEN** reference mesh, landmarks, labels, planes, axes, and deviation colors
  are absent
- **AND** enabling/disabling overlay does not change geometry or manufacturing
  artifact digests

### Requirement: Existing Capture Lifecycle Remains Safe

Guided reconstruction SHALL remain inside existing Capture/thread ownership and
explicit preview lifecycle. It SHALL NOT create a separate scan document, hidden
history authority, status bar, or provider fallback.

#### Scenario: Guided generation succeeds

- **WHEN** validated `.ecky` BRep preview is produced
- **THEN** owning thread receives ordinary preview draft
- **AND** production source projection remains uncommitted until explicit
  Apply/Commit while the candidate version is already in history
- **AND** guide digest and assumptions remain in result provenance

#### Scenario: Guided generation fails

- **WHEN** mesh is missing/stale, calibration/frame invalid, agent fails, or BRep
  verification fails
- **THEN** last good capture mesh/model may remain the successful projection
- **AND** failed candidate is appended and becomes `head`
- **AND** raw responsible error appears in Capture/agent workflow surface
- **AND** user can edit guide or retry without recapturing valid source evidence
