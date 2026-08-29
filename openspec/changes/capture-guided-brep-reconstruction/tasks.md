# Tasks: Capture-Guided BRep Reconstruction

## 1. Acceptance Fixtures And Evidence Baseline

- [x] 1.1 Add a small deterministic partial-scan fixture representing one half or
  quarter of a symmetric mechanical insert, with known source triangles,
  physical dimensions, landmarks, and expected BRep envelope.
- [x] 1.2 Add an outer failing Playwright scenario that opens the durable capture,
  picks calibration/frame/symmetry/profile evidence, and requests guided CAD.
- [x] 1.3 Add outer failure scenarios for degenerate frame evidence and a guide
  made stale by crop/source-mesh change.
- [x] 1.4 Record baseline proof that ordinary raw/cropped mesh Apply still follows
  existing `solidify(import-stl(...))` behavior when guided reconstruction is not
  selected.

## 2. Reconstruction Guide Contracts

- [x] 2.1 Add failing serialization/validation tests for versioned
  `CaptureReconstructionGuide`, `CaptureSurfaceAnchor`, landmarks,
  feature expectations, measurements, frames, axes, planes, profiles, evidence
  correspondences, and guide result provenance.
- [x] 2.2 Implement backend-owned snake_case Rust contracts with
  `#[serde(rename_all = "camelCase")]` and generated camelCase TypeScript
  contracts.
- [x] 2.3 Add deterministic canonical digest and revision identity tests,
  preserving profile order while canonicalizing unordered evidence where safe.
- [x] 2.4 Reject non-finite/unbounded coordinates, duplicate IDs, unknown roles,
  invalid references, invalid profile order, and unsupported schema versions.
- [x] 2.5 Keep guide and overlay types separate from `IndexedMeshAsset`, BRep,
  STL, STEP, and source AST contracts.
- [x] 2.6 Add discriminated opaque refs for capture anchor, BRep target, preview
  render vertex, analysis-boundary vertex, and FEM volume node. Require owner
  artifact digest and reject cross-kind fields/decoding.

## 3. Digest-Bound Surface Picking

- [x] 3.1 Add failing frontend unit test converting an exact Three.js triangle
  intersection into triangle index, barycentric weights, source position, and
  source normal before display transforms.
- [x] 3.2 Add failing backend test reloading source mesh and validating anchor
  digest, triangle, barycentric sum/range, recomputed position, triangle area,
  and normal consistency.
- [x] 3.3 Add Capture viewer landmark mode using existing raycaster; camera orbit
  and crop gizmo interaction must not create points accidentally.
- [x] 3.4 Render persistent numbered point overlays and selected-item focus
  without treating mesh vertices/faces as source-backed CAD entities.
- [x] 3.5 Add edit/delete/undo-within-draft behavior without creating accepted
  model history versions; each distinct draft state still appends a candidate
  version before validation.

## 4. Metric Calibration And Local Frame

- [x] 4.1 Add failing analytical tests for two-point known-distance uniform scale,
  multiple-measurement least-squares scale, and residual reporting.
- [x] 4.2 Reject coincident endpoints, non-positive distance, contradictory
  measurements above tolerance, and untrusted provider unit labels.
- [x] 4.3 Add failing analytical tests for deterministic right-handed orthonormal
  frame construction and source-to-local-mm transforms.
- [x] 4.4 Reject coincident, collinear, ill-conditioned, non-finite, or
  left-handed frame evidence with exact reason.
- [x] 4.5 Add Capture controls for selecting calibration endpoints, entering known
  millimetres, choosing frame origin/directions, and inspecting derived bounds.
- [x] 4.6 Keep raw/cropped STL immutable; calibration updates guide revision and
  derived coordinates only.

## 5. Axes, Planes, Profiles, And Mechanical Roles

- [x] 5.1 Add failing unit tests for named axis and plane fitting from minimum and
  overdetermined landmark sets, including RMS/max residuals and deterministic
  orientation.
- [x] 5.2 Add failing rejection tests for degenerate fits and residuals above
  configured tolerance.
- [x] 5.3 Add UI actions for named axis, symmetry plane, mating surface, bore,
  extent, clearance boundary, ignored damage, and generic labeled reference.
- [x] 5.4 Add ordered open/closed profile editing with visible vertex numbers,
  support plane, and explicit operation hint.
- [x] 5.5 Ensure fit-critical requested offsets/clearances are named constraints or
  parameters, never anonymous guide metadata.
- [x] 5.6 Support one- and two-plane half/quarter reconstruction evidence without
  mirroring triangle mesh as final geometry.
- [x] 5.7 Add target-kind/cardinality editing for validation-critical landmarks,
  profiles, axes, planes, mating faces, and cylindrical evidence. Keep scan,
  BRep, preview-render, analysis-boundary, and FEM-volume-node identities
  distinct.
- [x] 5.8 Separate expected analytic geometry kind from required BRep topology
  kind. Define cylinder/axis -> supporting face/edge, plane -> face, and profile
  -> ordered edges extraction.

## 6. Persistence, Staleness, And Historical Capture

- [x] 6.1 Add failing backend tests saving/loading guide through capture-run
  service with expected revision and mesh digest guards; never write SQLite
  outside owned persistence services.
- [x] 6.2 Persist guide metadata with capture run while keeping source mesh/photos
  in managed filesystem artifact storage.
- [x] 6.3 Restore guide and overlays when historical capture reopens; pairing-token
  rotation must not change guide identity.
- [x] 6.4 Mark guide stale when reconstruction, selected raw/crop artifact, crop
  bounds, or mesh digest changes.
- [x] 6.5 Add explicit remap proposal contract with old/new anchors and residual;
  require confirmation before remapped landmark becomes authoritative.
- [x] 6.6 Preserve last successful guide/model as the render projection on failed
  save, stale revision, missing mesh, or remap rejection; retain any appended
  candidate and its raw reason when persistence is available.
- [ ] 6.7 Migrate guide/source/draft persistence to append every distinct
  candidate before validation; retain failed/stale candidates, exact source, and
  raw evidence, serialize appends, and advance `head` to the latest candidate.
- [ ] 6.8 Add migration tests proving stale expected revisions and changed mesh
  digests append without conflict/refusal; successful-version filtering remains
  a projection only.

## 7. Canonical Agent Handoff

- [x] 7.1 Add failing service test producing bounded
  `CaptureGuidedReconstructionRequest` from exact guide revision, target source
  digest/version, mesh artifact identity, instruction, and evidence views.
- [x] 7.2 Generate deterministic local-frame front/right/top/isometric evidence
  views with landmark/profile labels and no unrelated viewport state.
- [x] 7.3 Add `BUILD CAD FROM GUIDE` only when calibration, frame, source identity,
  and required references validate.
- [x] 7.4 Queue request into owning thread through existing agent workflow; no
  hidden provider, direct database write, or whichever-thread-is-visible patch.
- [x] 7.5 Require agent output to be parametric `.ecky` BRep with guide/mesh
  provenance, explicit assumptions/inferred regions, named fit constraints, and
  repeated/mirrored source structures. Require named authored binding/tag for
  every validation-critical feature expectation.
- [x] 7.6 Keep ambiguous reconstruction pending for targeted user confirmation
  instead of generating arbitrary geometry.

## 8. BRep Preview And Symmetry Proof

- [x] 8.1 Add failing integration fixture proving a quarter guide produces one
  authored quarter and explicit X/Y symmetry operations, not four copied shape
  blocks or mirrored STL.
- [x] 8.2 Validate generated source through normal compiler, exact OCCT preview,
  structural verification, and source/version digest provenance checks.
- [x] 8.3 Reject open/invalid expected-solid result, compile failure, unresolved
  assumptions, or missing fit-critical bindings without accepting the candidate.
- [x] 8.4 Preserve ordinary `.ecky` source and artifact lifecycle; reconstruction
  result is not a new geometry kind.
- [x] 8.5 Keep production source projection unchanged until explicit Apply/Commit
  and surface raw compiler/runtime errors beside responsible guide/build state.
- [x] 8.6 Add failing exact-runtime tests resolving each feature expectation from
  authored binding/tag to expected BRep target kind and cardinality. Reject
  missing, ambiguous, wrong-kind, stale, or coordinate-only matches.
- [x] 8.7 Add binding-only and tag-only resolution tests using discriminated
  selector ref, part ID, and optional instance path. Reject wrong-part/instance,
  cross-artifact IDs, and old durable IDs after geometry digest change.

## 9. Reference Overlay And Honest Deviation

- [x] 9.1 Add failing viewer test rendering immutable scan ghost and generated
  BRep in same calibrated frame with independent visibility/opacity controls.
- [x] 9.2 Add backend tests for deterministic target-kind residuals, bounded
  observed-mesh-to-BRep sampling, max/RMS/percentile metrics, and outlier counts.
- [x] 9.2a Prove vertex, edge/profile, face/normal, plane, and axis/cylinder
  expectations use exact target metric; nearest whole-shape distance cannot
  satisfy validation-critical evidence.
- [x] 9.3 Label sampled metrics as observed-region evidence and symmetry-completed
  or otherwise missing regions as inferred/unverified.
- [x] 9.4 Render landmarks, axes, planes, profiles, inferred-region cues, and
  deviation colors as display-only diagnostics.
- [x] 9.5 Prove overlays and diagnostics do not alter BRep, STL, STEP, or
  manufacturing artifact digests.

## 10. Product And Failure Verification

- [x] 10.1 Run frontend unit tests for anchor generation, guide editing, and
  interaction arbitration.
- [x] 10.2 Run Playwright happy path plus degenerate, stale mesh, insufficient
  evidence, and source-divergence scenarios.
- [x] 10.3 Run backend contract, geometry math, persistence, handoff, and deviation
  tests.
- [x] 10.4 Run the partial symmetric insert fixture end-to-end and record guide,
  source, BRep, overlay, and residual evidence.
- [x] 10.5 Run `cd src-tauri && cargo check` after Rust changes.
- [x] 10.6 Run `git diff --check` and
  `openspec validate capture-guided-brep-reconstruction --strict`.
- [x] 10.7 Verify Tactical Midnight, square borders, `overflow: hidden`, raw error
  reporting, and absence of a new status bar or terminal-output dump.
- [ ] 10.8 Add migration BDD coverage: invalid compile, validation, stale-source,
  and source-divergence candidates append first, become `head`, retain exact
  source/raw evidence, and never produce a version conflict or loss.

## 11. Deterministic Reconstruction Stack

- [x] 11.1 Add digest-bound neighborhood extraction around anchors with bounded
  sample counts, adjacency provenance, coverage, noise, curvature, and uncertainty.
- [x] 11.2 Add deterministic line/circle/plane/cylinder/cone/sphere candidate fits
  with robust residuals, parameter domains, degeneracy rejection, and competing
  hypothesis reporting.
- [x] 11.3 Add source-surface segmentation and region-adjacency graph; keep
  damaged/ignored masks explicit and preserve source evidence IDs.
- [x] 11.4 Replace polyline-only profile evidence with bounded line/arc/spline
  reconstruction, continuity/closure checks, support-plane validation, and fit
  residuals.
- [x] 11.5 Build typed named dimension/constraint graph for symmetry, coaxial,
  coplanar, parallel, perpendicular, tangent, equal-radius, thickness, extent,
  clearance, and tolerance relations.
- [x] 11.6 Add deterministic bounded feature-plan candidates for supported
  extrude/revolve/sweep/boolean/mirror structures with evidence score and exact
  reasons for rejection.
- [x] 11.7 Extend guide readiness to report missing/bypassed stages and affected
  evidence. Permit bypass only under explicit constraints proving no material
  ambiguity.
- [x] 11.8 Restrict agent handoff to computed candidates and explicit user
  constraints. Reject agent-authored unbound fit-critical dimensions, primitives,
  or feature operations not supported by recorded evidence/confirmation.
- [x] 11.9 Add outer BDD cases for supported deterministic plan, ambiguous plan
  requiring confirmation, noisy over-tolerance fit, and unsupported primitive.
- [x] 11.10 Add exact-runtime proof that chosen feature plan traces through
  authored nodes/bindings to BRep targets and observed-deviation evidence.
