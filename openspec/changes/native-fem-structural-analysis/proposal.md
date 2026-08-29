# Proposal: Native FEM Structural Analysis

## Why

Ecky can author parametric solids, preserve stable face selectors, produce exact
OCCT BRep, and validate/export surface meshes, but it cannot answer structural
questions such as displacement, stress, reaction force, safety factor, or mass
under an authored load case. The existing triangle mesh is a manufacturing and
preview surface; it is not a volume finite-element mesh and no stiffness
assembly or linear-system solve exists.

Users designing lightweight load-bearing parts need a deterministic loop inside
Ecky:

```text
.ecky parameters -> exact solid -> volume mesh -> load case -> solve
  -> stress/displacement checks -> parameter edit -> repeat
```

That loop must not depend on the FreeCAD UI, arbitrary Python, or a hidden cloud solver,
and it must not present an unconverged linear-elastic result as engineering
certification.

## What Changes

- Add top-level `.ecky` analysis declarations for one 3D linear-static solid
  study, isotropic material data, Tet4 mesh controls, stable face-selected
  constraints, and stable face-selected loads.
- Add an explicit analysis-intent and assumption ledger: engineering question,
  acceptance metrics, geometry idealization, material/load/support provenance,
  uncertainty, and linear-static/Tet4 applicability gates. Agent-authored study
  values are proposals until backed by user or recorded engineering evidence.
- Add dimension-checked structural units for force, stress/modulus, density,
  and FEM result metrics while retaining millimetres as canonical CAD length.
- Add a provenance-preserving boundary mesh from Direct OCCT and a separate
  validated tetrahedral volume-mesh artifact. The FEM mesh never replaces
  manufacturing BRep/STL/STEP geometry.
- Add an external Gmsh HXT Tet4 meshing adapter, with an optional Netgen exact-
  BRep fallback, isolated behind bounded cancellable worker processes without
  FreeCAD or an STL round-trip.
- Add a Rust FEM crate for Tet4 small-strain isotropic elasticity, sparse
  assembly, Dirichlet elimination, sparse solution, reactions, strain, stress,
  von Mises stress, displacement, mass, and safety-factor results.
- Add immutable, digest-bound FEM result artifacts and stale-result detection
  when source, parameters, geometry, selectors, material, mesh controls, or
  runtime identity changes.
- Add workbench and MCP actions to generate/inspect the FEM mesh, run a study,
  run a mesh-convergence study, and inspect extrema with source/selector
  provenance.
- Add viewport-only mesh, deformation, and scalar-field views. FEM display
  state and debug geometry never enter STL or STEP exports.
- Expose FEM extrema and mass as authored verification metrics so a parametric
  model can enforce stress, displacement, and safety-factor requirements.
- Separate equation verification, discretization convergence, model
  applicability, and physical validation. A numerically green solve alone never
  establishes a trustworthy engineering decision.

## MVP Scope

The first supported study is intentionally narrow:

- three-dimensional, static, small-displacement, small-strain elasticity;
- one connected closed solid domain;
- one homogeneous isotropic material;
- first-order four-node tetrahedra (`tet4`);
- fixed and component-wise prescribed-displacement boundary conditions;
- total surface force, global traction, and scalar normal pressure;
- displacement, strain, Cauchy stress, von Mises stress, reaction force, mass,
  and yield-based safety-factor post-processing;
- explicit mesh quality and mesh-convergence evidence.

## Out of Scope

- Contact, friction, fastener pretension, bonded multi-body interfaces, shell,
  beam, truss, or composite elements.
- Plasticity, hyperelasticity, large deformation, nonlinear geometry, fracture,
  fatigue, creep, impact, thermal coupling, modal, harmonic, transient, or
  buckling analysis.
- Automatic topology/generative optimization or solver-driven mutation of
  geometry during the same compilation. Parameter search may orchestrate the
  existing preview/edit flow in a later change.
- Engineering certification, automatic acceptance of stress singularities, or
  replacement for physical testing and qualified review.
- Silent geometry defeaturing, support stiffening, load redistribution,
  material lookup, load-case invention, or uncertainty suppression by an agent.
- Runtime dependence on FreeCAD FEM, CalculiX, TetGen, arbitrary Python, or a
  remote service. Gmsh is invoked only as a separately installed external
  executable; an explicitly probed Netgen adapter may use its own interpreter.
  Gmsh is never linked or bundled without approved GPL/commercial licensing.
- Treating a surface STL as a tetrahedral FEM mesh.

## Capabilities

### New Capabilities

- `fem-analysis-authoring`: Typed `.ecky` structural studies, units, materials,
  mesh controls, boundary conditions, loads, and verification metrics.
- `native-fem-runtime`: Provenance-preserving volume meshing, Tet4 assembly,
  sparse solve, validation, caching, progress, cancellation, and result
  artifacts.
- `fem-result-inspection`: Workbench/MCP inspection, convergence evidence,
  deformation/field visualization, and truthful stale/error states.

### Modified Capabilities

None. FEM artifacts consume existing geometry/topology evidence but do not
change the manufacturing semantics of `direct-occt-runtime`,
`mesh-native-authoring`, or `workbench-viewport`.

## Impact

- `.ecky` parser, surface verifier, Core program metadata, source maps, and
  dependency graph.
- New isolated `src-tauri/crates/ecky-fem` domain crate.
- Direct OCCT boundary tessellation output with per-triangle face-target
  provenance.
- External-runtime probes and a dedicated Gmsh HXT adapter with optional Netgen
  exact-BRep fallback.
- Artifact bundle/model manifest additive FEM summaries and derived-asset
  routing.
- Render/verification services, MCP capability group, and Tauri commands using
  camelCase frontend DTOs and `#[serde(rename_all = "camelCase")]` backend
  boundary structs.
- Workbench control dock and viewport result-material path; no separate agent
  status bar and no solver stdout/stderr copied into general app logs.
- Runtime/license evidence for separately installed Gmsh and Netgen; neither is
  linked or bundled by the default product. Gmsh's GPL and TetGen's AGPL paths
  are excluded from the default bundle.

## Proof Gates

- A parameterized bracket authored in `.ecky` runs end to end without FreeCAD,
  arbitrary Python, CalculiX, network access, or hidden fallback.
- Constant-strain patch, uniaxial bar, load/reaction equilibrium, rigid-body
  rejection, and offline CalculiX differential fixtures pass recorded
  tolerances.
- Every constrained/loaded FEM boundary facet maps to the authored durable face
  selector; missing, ambiguous, or partially covered mappings fail before solve.
- BRep face/edge/loop incidence and surface-boundary adjacency remain equivalent
  under tessellation policy; geometric proximity alone cannot replace group
  identity.
- Invalid/inverted/degenerate tetrahedra, inadequate constraints, non-finite
  values, and solver failure publish no result artifact and surface the exact
  stage error.
- A three-level mesh study reports element counts, quality, metric deltas, and
  convergence status; unconverged results remain visibly unconverged.
- Study admission records engineering question, acceptance metric, assumptions,
  idealizations, and provenance for every material, load, and support value.
  Missing decision-critical evidence cannot be filled from model appearance or
  agent prose.
- Pre/post-solve applicability checks cover one-solid scope, thin/slender Tet4
  risk, near-incompressible locking risk, small-displacement ratio, elastic/yield
  range, support/load singularities, and unsupported interface physics.
- Sensitivity/uncertainty evidence remains separate from mesh convergence and
  numerical residual. Absence of physical validation is visible.
- FEM result views do not alter BRep, STL, STEP, or manufacturing artifact
  digests.
- Parameter or geometry changes mark prior FEM results stale and prevent them
  from satisfying authored verification.
- Cancellation leaves no worker process or partial cache/result artifact.
- Relevant Rust/unit/integration/browser tests, `cd src-tauri && cargo check`,
  and `openspec validate native-fem-structural-analysis --strict` pass.
