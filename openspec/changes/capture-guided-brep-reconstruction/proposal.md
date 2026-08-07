# Proposal: Capture-Guided BRep Reconstruction

## Intent

Turn an incomplete photogrammetry mesh into useful, explicit design evidence for
reconstructing a parametric `.ecky` BRep.

Today Capture produces an ordinary triangle mesh and lets the user scale, box
crop, preview, apply, and commit it. That is appropriate for mesh ownership, but
an incomplete scan does not contain design intent. When an agent receives only a
partial STL and a prose request, it must guess the object's coordinate system,
symmetry, mating surfaces, dimensions, and missing regions. The resulting CAD
can therefore be unrelated to the physical part even when the visible scan is
recognizable.

The new workflow lets the user click calibrated landmarks on the capture mesh,
name their mechanical roles, define axes and symmetry planes, order profile
points, and attach reconstruction instructions. Ecky persists this evidence as a
versioned `CaptureReconstructionGuide`. An agent uses the guide, bounded visual
references, and the immutable source mesh to author ordinary parametric `.ecky`
source through the existing inspect -> validate -> preview -> commit path.

The scan remains reference geometry. It never becomes an authoritative analytic
surface, and mirroring a scan is not presented as a final BRep.

## Scope

- Add a landmark-guidance mode inside the existing Capture mesh viewport.
- Raycast exact triangle hits and store digest-bound barycentric anchors,
  positions, and normals.
- Calibrate physical scale from one or more known point-to-point distances, or
  explicitly accept trustworthy metric capture metadata when available.
- Define a right-handed local frame, named axes, and one or more symmetry planes
  from selected evidence.
- Extract bounded source-surface neighborhoods around semantic anchors and attach
  deterministic uncertainty/provenance instead of treating one triangle normal
  as sufficient shape evidence.
- Fit supported analytic primitive and profile candidates before agent handoff;
  preserve candidate scores, residuals, domains, and rejected hypotheses.
- Build a typed dimension/constraint graph and bounded feature-plan candidates
  from computed evidence. Keep unresolved topology/product-shape choices
  explicit for agent or user resolution.
- Create labeled feature landmarks and ordered open or closed profile paths.
- Let validation-critical guide items declare expected exact CAD target kind
  without pretending scan anchor is that target.
- Persist a versioned, deterministic reconstruction guide through Ecky-owned
  backend services and capture-run history.
- Produce a canonical agent handoff containing guide data, source identity,
  reconstruction instruction, and bounded visual reference views.
- Author the reconstructed part as normal parameterized `.ecky` geometry,
  including named fit-critical parameters, bindings, and symmetry operations.
- Overlay generated BRep and reference mesh in the same calibrated frame.
- Record typed correspondences from guide evidence through authored bindings/
  tags to exact BRep targets, then report target-kind residuals and sampled
  deviation only on observed scan regions.
- Preserve explicit preview, Apply, Commit, stale-source, and raw-error behavior.

## Out Of Scope

- Automatic or guaranteed STL-to-BRep conversion.
- Treating scan triangles as source-backed CAD faces, edges, or parameters.
- Reusing scan landmark IDs, capture triangle vertices, preview-render vertices,
  analysis-boundary vertices, or future FEM volume-node numbers as exact BRep
  topology identity.
- Filling unknown regions without an explicit user instruction or symmetry
  constraint.
- Using mirrored triangle mesh as manufacturing geometry.
- Free-form mesh sculpting, retopology, or a separate scan document/editor.
- Claiming metrology, certification, or accuracy beyond calibration and reported
  residuals.
- Cloud reconstruction, cloud geometry inference, or a hidden provider fallback.
- Automatically modifying a mating assembly or adding clearance not requested by
  the user.

## Product Direction

Capture becomes an evidence-acquisition surface rather than a weak mesh-to-CAD
shortcut:

```text
photos -> immutable capture mesh
       -> digest-bound anchors + local surface neighborhoods
       -> uncertainty + scale + frame calibration
       -> analytic primitive / curve / profile fits
       -> surface regions + adjacency
       -> dimensions + constraint graph
       -> bounded feature-plan candidates
       -> semantic landmarks / axes / planes / profiles
       -> expected feature kinds and authored bindings/tags
       -> CaptureReconstructionGuide
       -> agent-authored .ecky source
       -> exact BRep preview
       -> typed evidence-to-BRep correspondences
       -> reference overlay and bounded deviation evidence
       -> explicit Apply / Commit
```

A small number of semantically labeled points can be enough for a simple
symmetric mechanical insert. For example, two points calibrate a known distance,
two points define an axis, three or more samples define a symmetry plane, and a
short ordered profile describes a visible quarter. More complex or noisy parts
require more evidence; Ecky shall expose insufficiency rather than invent
precision.

The agent is not the geometry calculator. Deterministic backend stages own
coordinates, neighborhoods, primitive/curve fits, dimensions, constraints,
candidate residuals, and rejection bounds. The agent owns semantic selection
among supported candidates and authors canonical `.ecky` source. If computed
evidence permits multiple materially different feature plans, the system asks
the user instead of letting prose silently choose product shape.

## Proof Gates

- Clicking a capture mesh records the exact triangle hit in source-mesh
  coordinates and displays a persistent landmark overlay.
- Two-point known-distance calibration produces deterministic millimetre
  coordinates and rejects zero-length or non-finite evidence.
- A frame or plane built from degenerate/collinear evidence is rejected with the
  raw geometric reason.
- Supported plane, cylinder/cone/sphere, line/arc/spline, and ordered-profile
  hypotheses carry deterministic fit domains, residuals, uncertainty, and source
  evidence IDs; unsupported or ambiguous fits remain explicit.
- Agent handoff includes typed constraint graph and bounded feature-plan
  candidates. Sparse raw points alone cannot claim reconstruction ready.
- Every guide is bound to capture run, selected raw/cropped mesh digest,
  calibration revision, and source coordinate frame.
- Changing reconstruction, crop, or source mesh makes the old guide stale and
  blocks CAD generation until landmarks are remapped or confirmed.
- `BUILD CAD FROM GUIDE` sends structured evidence and reference views, not an
  instruction to solidify or mirror STL.
- Generated source uses ordinary `.ecky` operations and named fit-critical
  parameters/constraints; symmetry is represented explicitly.
- Every validation-critical guide item resolves through an authored binding/tag
  to expected exact BRep target kind and declared cardinality; missing or
  ambiguous correspondence blocks acceptance.
- Overlay and deviation diagnostics cannot enter STL or STEP export geometry.
- Deviations are reported only against observed scan regions and never imply
  correctness of reconstructed missing regions.
- Browser happy path and stale/degenerate failure paths pass; Rust changes pass
  `cd src-tauri && cargo check`.
