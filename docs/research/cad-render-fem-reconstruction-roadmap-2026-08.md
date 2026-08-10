# CAD, Reconstruction, Rendering, And FEM Research Roadmap

Date: 2026-08-09

## Decision Summary

Ecky should treat exact BRep topology and authored design intent as authority.
Scan meshes, render meshes, and FEM meshes are derived evidence with separate
identities.

The highest-value shared seam is a typed correspondence chain:

```text
scan surface anchor
  -> semantic landmark / profile / axis / plane
  -> expected authored feature
  -> exact BRep vertex / edge / face target
  -> topology-preserving boundary group
  -> volume-mesh boundary facet
  -> FEM nodes and elements
```

Each arrow needs relation kind, source/target digest, tolerance, cardinality,
and residual. Equal coordinates never imply equal identity.

Two existing OpenSpec changes own the first useful slices:

- `capture-guided-brep-reconstruction` owns scan evidence through exact BRep
  feature correspondence and observed-region validation.
- `native-fem-structural-analysis` owns exact BRep face targets through boundary
  and volume-mesh provenance, solve, convergence, and result inspection.

No third overlapping change is needed now. General lineage-query syntax becomes
a separate change only after parameter-sweep fixtures expose gaps that existing
tags plus `:created-by` provenance cannot express.

## Identity Model

| Entity | Authority | Stability | Valid use |
| --- | --- | --- | --- |
| `CaptureSurfaceAnchor` | Exact hit on one immutable scan-mesh digest | Stable only for that digest | Calibration and observed evidence |
| `CaptureLandmark` | Semantic role attached to scan evidence | Stable across camera motion; stale after source change | Reconstruction intent |
| Authored binding/tag | `.ecky` source intent | Stable across rebuilds while source semantics remain | Feature naming and selector input |
| BRep vertex/edge/face target | Exact OCCT topology for one geometry digest | Durable through recorded lineage/rebinding evidence | CAD operations, validation, FEM boundary selection |
| Preview-render vertex/triangle | Approximation under one render policy | Derived and digest-bound | Preview and visual deviation |
| Analysis-boundary vertex/triangle | Topology-checked approximation under one FEM boundary policy | Derived and digest-bound | Volume-mesh handoff |
| FEM node/Tet4 | Analysis discretization under one mesher identity | Derived and mesh-digest-bound | Assembly and post-processing only |

Forbidden shortcuts:

- scan landmark ID reused as BRep target ID;
- nearest whole-shape surface accepted when guide expected a named edge, face,
  vertex, axis, or cylindrical feature;
- preview-render or analysis-boundary vertex used as durable CAD selector;
- FEM node number persisted as authored load/support identity;
- STL round-trip used between exact face selection and FEM boundary grouping.

## Research Findings

### 1. Lineage-based CAD references

The PLDI 2023 lineage DSL makes references queries over operation history,
including generated, modified, split, merged, and deleted topology. Main Ecky
lesson: geometric predicates refine identity; they do not define it alone.
Selectors expecting one entity must reject zero or multiple results.

Action:

- keep authored tags and `:created-by` provenance as current surface;
- record operation lineage and explicit selector cardinality in reconstruction
  correspondences and FEM analysis identity;
- add parameter-sweep survival tests before expanding selector syntax.

Source: [A Lineage-Based Referencing DSL for Computer-Aided Design](https://dl.acm.org/doi/10.1145/3591238)

### 2. Editability must be tested after parameter changes

HistCAD separates executable reconstruction from preserved constraints and
editability. An Ecky model rebuilding once is insufficient.

Action:

- sample nominal, min/max, near-topology-transition, and coordinated parameter
  edits;
- record rebuild success, selector survival, correspondence survival, verify
  preservation, topology regime, and FEM boundary-group survival;
- fail silent rebinding when cardinality or lineage changes.

Source: [HistCAD](https://arxiv.org/abs/2602.19171)

### 3. Exact measurements plus diagnostic views beat render-only validation

CADSmith combines exact OCCT measurements with rendered inspection. Ecky should
use exact BRep checks for validity, volume, mass properties, topology counts,
feature residuals, and fit constraints. Rendering should direct attention to
recent operations, selected targets, sections, free edges, and deviation hot
spots.

Action:

- keep normal preview deterministic;
- add target-isolation, section, normal, curvature, and deviation passes as
  preview-only diagnostics;
- never let smoothed display fields replace checked raw extrema.

Source: [CADSmith](https://arxiv.org/abs/2603.26512)

### 4. BRep topology must survive tessellation

Topology-First B-Rep Meshing treats BRep topology as invariant and geometric
error as tolerance-controlled approximation. Ecky need not adopt the research
implementation immediately, but should adopt its contract.

Action:

- require BRep face/edge/loop incidence evidence beside boundary triangles;
- require every boundary triangle to map to one exact face group;
- compare source and mesh adjacency, coverage, orientation, and connectedness;
- reject repair that merges or invents semantic boundary groups.

Source: [Topology-First B-Rep Meshing](https://arxiv.org/abs/2604.02141)

### 5. Mesher choice: fTetWild default, Gmsh conditional

Current FEM OpenSpec incorrectly describes Gmsh as LGPL. Official Gmsh is
GPL-2-or-later; integrating it into distributed closed-source software requires
a commercial license. Its physical groups and discrete entities fit Ecky's
boundary model technically, but default bundling is not license-safe without a
project-wide GPL decision or commercial agreement.

fTetWild is MPL-2.0, runs on macOS/Linux/Windows, accepts tagged input faces,
tracks `surface_tags` through tetrahedralization, and produces valid Tet4
meshes from triangle surfaces. Its output may approximate/retriangulate the
input boundary, so Ecky must reconcile every output boundary facet to exactly
one source face group and reject ambiguity, missing coverage, or tolerance
excess. The adapter also needs a wider tag type than the upstream internal
`char` storage before Ecky can support arbitrary face-group counts.

Action:

- use pinned fTetWild in a killable native worker;
- preserve per-input-triangle face-group tags;
- publish MPL-covered adapter/fork modifications and required notices/source;
- audit libigl, geogram, GMP, oneTBB, Eigen, spdlog, fmt, and JSON dependencies;
- keep Gmsh as offline reference or separately licensed backend;
- reject TetGen/AGPL from default dependency graph.

Sources: [fTetWild paper](https://arxiv.org/abs/1908.03581),
[fTetWild implementation](https://github.com/wildmeshing/fTetWild),
[Gmsh licensing](https://gmsh.info/#License)

### 6. Keep Ecky's narrow Rust solver; use PolyFEM as oracle

PolyFEM offers broad high-order, nonlinear, material, contact, and optimization
capabilities. That breadth is valuable for offline comparison but larger than
the MVP contract. Existing `ecky-fem` code already owns versioned contracts,
budgets, Tet4 kinematics, isotropic constitutive behavior, strain/stress, and
element stiffness.

Action:

- continue backend-neutral Rust Tet4 assembly and sparse solve;
- compare recorded fixtures against closed-form solutions, CalculiX, and
  optionally PolyFEM;
- add Tet10, contact, nonlinear models, or PolyFEM runtime only through later
  explicit changes.

Source: [PolyFEM](https://polyfem.github.io/)

### 7. Scan-to-CAD needs semantic correspondences, not vertex equality

CADFit supports editable program recovery from meshes with extrusion,
revolution, fillet, chamfer, and Boolean operations. It reinforces Ecky's
choice to recover authored programs and validate them geometrically. Scan
landmarks should constrain expected features, not become copied CAD vertices.

Action:

- add `CaptureFeatureExpectation` to guide items;
- return `CaptureEvidenceCorrespondence` from guide item to authored binding/tag
  and exact BRep target;
- evaluate residual by target kind: point-to-vertex, point/profile-to-edge,
  point/normal-to-face, radial/axis residual for cylindrical evidence;
- reject nearest-target ambiguity and unbound validation-critical evidence.

Source: [CADFit](https://arxiv.org/abs/2605.01171)

### 8. Inverse editing belongs after stable fixed-topology correspondence

Differentiable CAD programs show geometry manipulation can solve for source
parameter updates, but not across topology changes. Ecky can later start with
finite-difference Jacobians over exposed parameters and exact measurements.

Action:

- defer drag-to-parameter and FEM optimization;
- require fixed topology and unchanged lineage/correspondence during an inverse
  step;
- abort and remesh/re-resolve after topology change.

Source: [Differentiable 3D CAD Programs for Bidirectional Editing](https://arxiv.org/abs/2110.01182)

## Delivery Order

### Phase 0: Correct contracts and licenses

1. Update both existing OpenSpec changes with typed correspondence boundaries.
2. Replace bundled-Gmsh assumption with fTetWild worker plus license audit.
3. Reconcile OpenSpec task state with tests already present.

### Phase 1: Capture evidence to exact BRep

1. Digest-bound scan picking and calibration.
2. Named roles, profiles, axes, planes, and feature expectations.
3. Agent-authored `.ecky` bindings/tags.
4. Exact target-kind correspondence and residual validation.
5. Observed-region deviation; inferred regions remain unverified.

### Phase 2: Topology-preserving analysis boundary

1. OCCT face/edge/loop incidence report.
2. Tessellation with exact face-group tags.
3. Source-versus-boundary topology equivalence and coverage gates.
4. Parameter-sweep selector/correspondence survival fixture.

### Phase 3: Native linear-static FEM

1. fTetWild worker and boundary reconciliation.
2. Tet4 global assembly, exact load integration, Dirichlet elimination, sparse
   solve, reactions, raw stress, mass, and safety factor.
3. Immutable result artifacts, cancellation, budgets, cache, and staleness.
4. Three-level convergence and independent numerical oracles.

### Phase 4: Diagnostic rendering and authoring loop

1. Boundary group, support, load, mesh-quality, deformation, and raw/display
   stress views.
2. Section and isolated-target views for suspicious geometry.
3. Exact verification metrics feeding agent revision.

### Phase 5: Editability and optimization

1. Parameter-perturbation benchmark.
2. Fixed-topology drag-to-parameter edits.
3. Outer-loop FEM parameter search with remesh and convergence at each accepted
   candidate.

## Proof Matrix

| Boundary | Required proof |
| --- | --- |
| Scan hit -> landmark | Mesh digest, triangle index, barycentric validation |
| Landmark -> feature expectation | Supported semantic role and target kind |
| Feature -> BRep target | Authored binding/tag, exact selector cardinality, target provenance |
| BRep -> surface boundary | Incidence equivalence, group coverage, tolerance, orientation |
| Surface -> Tet4 boundary | Tag reconciliation, one-owner boundary facets, no ambiguity |
| Tet4 -> solve | Quality, constraints, residual, equilibrium, finite fields |
| Solve -> claim | Mesh convergence, stale guard, raw extrema, scope disclaimer |
| Any display -> export | Manufacturing digests unchanged |

## Current Repository Evidence

At research time:

- `native-fem-structural-analysis` had complete proposal/design/spec/tasks
  artifacts but reported `0/98` implementation tasks.
- `ecky-fem` already had seven passing tests covering contracts, canonical
  digests, budgets, Tet4 volume/gradients, constitutive response, constant-strain
  patch behavior, rigid modes, and stiffness.
- `native_fem_bracket_contract` had two passing tests proving analysis metadata
  compiles without becoming geometry.
- `capture-guided-brep-reconstruction` already specified digest-bound scan
  landmarks, calibration, axes, planes, profiles, agent handoff, overlay, and
  observed-region deviation; exact evidence-to-BRep target correspondence was
  the missing bridge.
