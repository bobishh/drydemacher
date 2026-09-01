# Design: Native FEM Structural Analysis

## Context And Existing Seams

Ecky already has most of the geometry-side prerequisites:

- `src-tauri/native/direct_occt_runner.cpp` uses
  `BRepMesh_IncrementalMesh`, emits face topology, assigns face target ids, and
  resolves exact/stable face targets.
- `src-tauri/src/topology_target_ids.rs` and
  `src-tauri/src/ecky_cad_host/direct_occt_runtime.rs` preserve canonical,
  stable, and durable face aliases.
- `src-tauri/src/ecky_ir/mesh_asset.rs` owns a deterministic indexed triangle
  surface with manifold evidence and content digest.
- `csgrs`, its `nalgebra`/`parry3d-f64` stack, mesh literals, STL/3MF, and the
  hybrid BRep/mesh partition already cover surface geometry.

Those are not a volume FEM stack. `IndexedMeshAsset` contains vertices and
triangles but no tetrahedral cells, material regions, boundary-condition sets,
or per-triangle CAD-face provenance. Reusing it as a FEM mesh would erase the
mapping required to apply loads and constraints safely.

FreeCAD FEM is used as a conceptual reference for the study container,
materials, solver, mesh, constraints, loads, and result objects. Ecky does not
embed or automate the FreeCAD workbench.

## Goals / Non-Goals

### Goals

- Make one useful structural slice trustworthy before broadening physics.
- Keep authored geometry parametric and exact while treating FEM mesh/results
  as derived artifacts.
- Bind every face load and constraint through existing durable topology
  evidence rather than coordinate guesses.
- Keep core FEM contracts backend-neutral and isolate experimental/native
  dependencies behind narrow adapters.
- Provide deterministic diagnostics, progress, cancellation, cache identity,
  and reproducible reference validation.
- Enable a later optimizer to vary existing `.ecky` parameters and consume FEM
  metrics without allowing result-dependent geometry cycles.

### Non-Goals

- API compatibility with FreeCAD document objects or CalculiX input decks.
- A general PDE language or arbitrary weak-form authoring.
- Silent solver or mesher fallback.
- Automatic claims that a design is safe, certified, fatigue-resistant, or
  converged.

## Architecture

```text
.ecky source
  -> typed analysis declarations + geometry Core IR
  -> normal Direct OCCT geometry artifact and topology manifest
  -> AnalysisBoundarySurface
       vertices
       oriented triangles
       triangle -> durable CAD face group
       closed/manifold/provenance evidence
  -> external Gmsh HXT worker (optional Netgen exact-BRep fallback)
  -> FemVolumeMesh
       nodes
       Tet4 connectivity
       boundary triangles/groups
       quality + source/runtime digest
  -> ecky-fem assembler
       K, f, Dirichlet reduction
  -> sparse direct solve
  -> FemResultAsset
       u, strain, stress, von Mises, reactions, extrema, mass
  -> verification metrics + result-view surface + optional VTU
```

Normal geometry preview compiles analysis declarations but does not run a
potentially expensive solve. Mesh/solve is an explicit UI or MCP action. A FEM
check in authored verification is `missing` or `stale` until a successful
result with the current analysis identity exists.

## Layered Engineering Analysis Stack

The future structural-analysis product has 15 logical stages. They are evidence
and responsibility boundaries, not necessarily 15 software services.

| Stage | Owner | Output | MVP state |
| --- | --- | --- | --- |
| 1. Engineering question | user + typed authoring | named question, decision, acceptance metrics, consequence class | typed question/criteria implemented; consequence class not yet authored |
| 2. Geometry identity | exact CAD runtime | source/parameter/solid/topology identity | available |
| 3. Analysis idealization | deterministic backend + explicit user approval | included/removed features, domain choice, characteristic dimensions | exact-solid identity/approval implemented; defeatured geometry artifact not implemented |
| 4. Physics applicability | deterministic rule gates | admitted linear-static/small-strain/Tet4 assumptions and rejected alternatives | deterministic pre/post gates implemented for MVP scope |
| 5. Material evidence | user/evidence store + validation | property values, source, condition, uncertainty, applicability range | typed properties, provenance authority, and uncertainty implemented; applicability range remains authored evidence |
| 6. Interfaces and connections | explicit authoring + exact topology | bonded/contact/joint/fastener model and provenance | explicitly unsupported; one connected solid only |
| 7. Load cases and combinations | explicit authoring | magnitude, distribution, frame, duration, combination, provenance, uncertainty | surface force/traction/pressure and provenance implemented; combinations/duration not implemented |
| 8. Supports and boundary realism | explicit authoring + deterministic audit | constrained DOFs, physical rationale, over/underconstraint evidence | durable selectors, component constraints, rigid-mode and support-area audit implemented |
| 9. Analysis boundary representation | direct OCCT runtime | closed grouped surface with CAD-face provenance | implemented with BRep incidence and selected-face area checks |
| 10. Volume discretization | external exact-BRep mesher | Tet4 mesh, quality, group coverage, mesh identity | implemented through Gmsh HXT with optional Netgen fallback |
| 11. Equation assembly and solve | deterministic FEM core | K/f/constraints, displacement, stress, reactions | implemented for linear-static isotropic Tet4 |
| 12. Numerical verification | deterministic FEM core | patch/oracle proof, residual, equilibrium, energy, finite-value gates | implemented; independent offline solver golden remains open |
| 13. Discretization error and singularity analysis | convergence service | per-metric convergence, hotspot movement, suspected singularities | displacement/stress status and partial-failure evidence implemented; hotspot movement evidence remains open |
| 14. Sensitivity, uncertainty, and physical validation | deterministic orchestration + external evidence | response ranges, dominant assumptions, test/reference correlation | sensitivity/uncertainty contracts and gates implemented; physical/qualified validation remains external evidence |
| 15. Decision and traceability | verification + user | pass/fail/pending decision with applicability, uncertainty, convergence, and provenance | typed acceptance evaluation implemented; unresolved evidence or required convergence stays pending |

Stages 1, 3–8 define the engineering model. An agent may help author or explain
them, but cannot infer a material, load, support, connection, idealization, or
acceptance threshold from geometry appearance and mark it authoritative.
Stages 9–13 verify the numerical model. Stage 14 validates sensitivity and
physical relevance. Stage 15 makes a bounded decision.

```text
question + acceptance criterion
  -> explicit idealization + assumption ledger
  -> materials + interfaces + load cases + supports, all with provenance
  -> applicability gate
  -> boundary/volume discretization
  -> assemble/solve
  -> residual/equilibrium + convergence/singularity proof
  -> sensitivity/uncertainty + physical/reference validation
  -> accept | revise | unsupported | needs evidence
```

Solver success is only one intermediate fact. Mesh convergence cannot prove
loads, supports, material data, or model idealization are physically correct.
Offline CalculiX agreement verifies implementation, not the real product.

## Authoring Model

`analysis` is a top-level model clause. It references a part and existing face
selectors; it is not a geometry operation and returns no shape.

Illustrative normative surface syntax:

```scheme
(model
  (params
    (number load-n 1000 :min 10 :max 10000 :step 10 :unit "N")
    (number mesh-size 2.0 :min 0.25 :max 10 :step 0.25 :unit length))

  (part bracket
    (tag-face mounting :face "bottom"
      (tag-face load-pad :face "top"
        (difference
          (box 80 30 12)
          (translate 12 15 0 (cylinder 5 12))))))

  (analysis bracket-static
    (linear-static :part bracket)
    (material aluminum-6061
      :young-modulus 68900MPa
      :poisson-ratio 0.33
      :density 2700kg-per-m3
      :yield-strength 276MPa)
    (volume-mesh :element tet4 :size mesh-size
      (refine :faces (tag load-pad) :size 1mm))
    (fixed :faces (tag mounting))
    (surface-force :faces (tag load-pad)
      :total [0N 0N (- load-n)])
    (solve :method sparse-direct))

  (verify
    (metric bracket-stress (fem-max bracket-static von-mises))
    (metric bracket-deflection
      (fem-max bracket-static displacement-magnitude))
    (check bracket-stress (< 138MPa))
    (check bracket-deflection (< 0.5mm))))
```

The implementation SHALL define vector/unit syntax that preserves this meaning
without weakening existing parser guarantees. Zero vector components carry the
same force dimension. A scalar without required dimension is rejected in
strict FEM fields; permissive CAD-unit compatibility does not silently apply to
loads or material properties.

Canonical internal dimensions are:

- length: millimetre;
- force: newton;
- stress and elastic modulus: megapascal (`N/mm²`);
- mass: kilogram;
- density: `kg/mm³` after normalization;
- displacement: millimetre;
- strain and Poisson ratio: dimensionless.

Material validation requires finite positive Young's modulus and density,
`-1 < poissonRatio < 0.5`, and finite positive yield strength when safety factor
is requested.

`surface-force :total` is a global vector whose exact total is distributed by
consistent integration over selected boundary triangles. `traction` is a
force-per-area global vector. Positive `pressure` acts inward against the
outward boundary normal. Prescribed displacement may constrain any subset of
x/y/z; `fixed` constrains all three.

A face selector that resolves to zero, multiple ambiguous durable targets, the
wrong part, or only partial volume-boundary coverage is a hard pre-solve error.
Coordinate-plane selection may be used only through an authored selector that
resolves to recorded target ids.

Scan landmarks, render vertices, surface-mesh vertices, and FEM node numbers are
not valid boundary-condition identities. Capture reconstruction may establish a
typed correspondence to an exact BRep face; FEM consumes that BRep target, not
the upstream scan point or downstream discretization index.

## No Analysis-to-Geometry Cycle

FEM values are post-render metrics. Geometry expressions in a model version
MUST NOT read displacement, stress, or other results from that same version.
Shape optimization is an outer orchestration loop:

```text
choose params -> preview geometry -> run FEM -> inspect metrics
  -> choose new params -> repeat
```

This preserves a directed dependency graph, version history, and reproducible
cache identity. A later optimization change may automate parameter selection;
it must not mutate anonymous mesh nodes as authored CAD.

## Boundary Surface And Provenance

A new `AnalysisBoundarySurface` is distinct from `IndexedMeshAsset` and stores:

```rust
struct AnalysisBoundarySurface {
    vertices: Vec<[f64; 3]>,
    triangles: Vec<[u32; 3]>,
    triangle_face_groups: Vec<FaceGroupId>,
    face_groups: Vec<FemFaceGroup>,
    topology: BoundaryTopologyEvidence,
    source_geometry_digest: String,
    tessellation_policy: FemBoundaryTessellationPolicy,
    content_digest: String,
}
```

`FemFaceGroup` carries part id, requested selector/tag when applicable,
canonical target id, durable target id, aliases, source stable-node key, area,
and triangle range/set. Direct OCCT emits triangles while traversing each
`TopoDS_Face`; shared vertices use the existing named weld policy. Orientation
is corrected against the solid so pressure normals are outward.

Admission requires one connected closed orientable boundary, finite vertices,
non-degenerate triangles, zero boundary/non-manifold edges, positive enclosed
volume, and exact group array cardinality. Self-intersection policy is explicit.
No STL round-trip is permitted because STL loses face-group provenance.

`BoundaryTopologyEvidence` also records BRep vertex/edge/face/loop incidence and
the corresponding mesh adjacency. Admission requires topology equivalence for
the supported closed-manifold solid: no BRep face group disappears, merges with
another group, or gains an adjacency absent from source topology. Tessellation
tolerance controls geometric approximation only, not semantic topology.

## Volume Mesher Decision

### Selected MVP Backend: Gmsh HXT

Invoke a separately installed Gmsh executable through a dedicated bounded worker.
The adapter exports the exact Direct OCCT BRep to a temporary STEP input, asks
Gmsh's OCC/HXT path for Tet4 output, and parses bounded ASCII MSH2 output. Face
signatures, durable face groups, source/STEP digest, mesh controls, and runtime
identity are checked before publication. No STL round-trip or untagged remesh is
allowed.

Gmsh HXT is the primary mesher. If it fails after producing no accepted mesh,
the service may use an explicitly probed Netgen OCC exact-BRep fallback within
the remaining runtime budget. Netgen receives the same STEP, face count,
durable-group mapping, size controls, and digest-bound request. A missing or
changed executable/module, malformed output, timeout, cancellation, or failed
face mapping remains a raw stage error; no TetGen, FreeCAD, Python fallback,
network, or cloud service is attempted beyond the explicit Netgen adapter.

Gmsh is GPL-2-or-later and is never linked or bundled by default. The runtime
probe resolves `ECKY_GMSH_EXECUTABLE` or `gmsh` on PATH and records executable
path, version, SHA-256, platform, architecture, and HXT adapter protocol. Netgen
is similarly probed through `ECKY_NETGEN_PYTHON` or the `netgen` launcher; its
Python/module paths and digests enter runtime identity. Product distribution
must provide the required external-runtime license evidence.

### Rejected Default: TetGen Through `tritet`

`tritet` provides useful Rust wrapping, boundary markers, and TetGen support,
but enabling `with_tetgen` changes the project license to AGPL. It SHALL NOT be
a bundled/default Ecky backend. The pure-Rust `tetgen` crate is currently WIP
and is not a production alternative.

### Not Selected As Core: Gemlab

`gemlab` provides meshes, feature search, integration, and TetGen calls, but its
default scientific stack adds OpenBLAS/SuiteSparse system dependencies and its
unstructured tetrahedral route does not remove TetGen licensing concerns. It
may serve as a test/reference library, not the product contract.

## FemVolumeMesh Contract

The backend-neutral mesh contains:

```rust
struct FemVolumeMesh {
    nodes: Vec<[f64; 3]>,
    tetrahedra: Vec<[u32; 4]>,
    boundary_triangles: Vec<[u32; 3]>,
    boundary_face_groups: Vec<FaceGroupId>,
    quality: FemMeshQuality,
    source_boundary_digest: String,
    mesher_identity: RuntimeIdentity,
    content_digest: String,
}
```

Validation rejects non-finite data, missing references, repeated cell nodes,
duplicate cells, zero/negative signed volume after orientation normalization,
facets not owned by exactly one tetrahedron, interior facets mislabeled as
boundary, incomplete source-boundary coverage, and configured budget/quality
violations. It records at least node/cell counts, min/max volume, minimum scaled
Jacobian or equivalent documented tetra quality, radius ratio, boundary area
by group, connected components, and worst element location.

MVP uses reproducible Gmsh HXT/Netgen options, fixed thread policy, no ambient
mesher config, and every option plus external-runtime identity in cache identity.
Connectivity is canonicalized before digesting. If the mesher still produces a
different topology for identical identity, the differing digest is reported;
it is never reused under the old key.

Local `refine` controls attach to durable face groups. Global/min/max size,
curvature behavior, element order, node/cell budgets, and quality threshold are
validated before worker launch.

## Rust Assembly And Sparse Solve

Create `src-tauri/crates/ecky-fem` with no Tauri/UI dependency. Public domain
contracts are internal Ecky types, not Fenris or Faer types.

Use `fenris` behind an `ElementAssembler` adapter for Tet4 element mapping,
quadrature, and sparse assembly where its API proves adequate. Pin the audited
version because Fenris declares no API stability and does not solve linear
systems. If a required elasticity operation is absent, implement the documented
constant-strain Tet4 `Bᵀ D B V` kernel inside the same adapter and validate it
against the independent patch/oracle suite; do not leak the implementation
choice into manifests.

Use `faer` behind `LinearSolver` for the reduced symmetric sparse system. The
MVP direct path is sparse Cholesky/LDLT with ordering and residual checks.
Prescribed degrees of freedom are applied by elimination, not a penalty
constant. Keep the unreduced `K` and `f` information needed to compute support
reactions. Reject a non-SPD/singular system with a diagnostic that identifies
likely unconstrained rigid-body modes; do not add hidden springs.

For every successful solve, verify:

- finite displacement/result arrays;
- normalized residual below configured tolerance;
- applied plus reaction force equilibrium within tolerance;
- strain energy non-negative within numerical tolerance;
- no unsupported element/material/load kind entered assembly.

Stress is computed per tetrahedron from small strain. Nodal display stress is a
clearly labeled volume-weighted extrapolation/average; extrema used for
verification default to unaveraged element/integration-point values so display
smoothing cannot lower the checked maximum. Yield safety factor is
`yieldStrength / vonMises`; zero-stress regions report infinity through a typed
representation, never non-standard JSON `Infinity`.

## Result And Artifact Model

`FemResultAsset` is immutable and keyed by:

- source/model and parameter digest;
- geometry artifact digest;
- analysis declaration and selector-resolution digest;
- boundary and volume-mesh digests;
- material/load/constraint digest;
- assembler/solver schema and runtime identity;
- numerical tolerances.

It stores or routes:

- nodal displacement vectors;
- element strain/stress tensors and von Mises scalars;
- support reactions by node and authored face group;
- extrema with element/node coordinates and source face provenance when
  available;
- volume, mass, quality, equilibrium, residual, and convergence summaries;
- a boundary result surface for viewport rendering;
- optional VTU export for ParaView interoperability.

Large arrays use a versioned bounded binary sidecar plus a JSON manifest with
shape, scalar type, byte ranges, digests, and units. They are not expanded into
thread messages or normal MCP summaries. `vtkio` may produce VTU, but VTU is an
interchange/export artifact rather than the internal source of truth.

Successful complete artifacts enter cache atomically. Failures and cancelled
jobs publish neither result nor partial cache entries. Geometry/source changes
make prior results `stale`; stale results remain inspectable with their old
identity but cannot satisfy current verification.

## Convergence And Engineering Honesty

A convergence action evaluates at least three explicit mesh sizes against
selected metrics. It reports each mesh identity, node/tet count, quality,
solve residual, maximum displacement, maximum unaveraged von Mises stress, and
relative deltas.

The study is `converged` only when configured metrics meet configured relative
change thresholds on consecutive refinements and all quality/solve gates pass.
A stress maximum at a re-entrant corner, point-load surrogate, fixed-edge
singularity, or continually rising hotspot remains `unconverged/suspected
singularity`; the UI and MCP response preserve that status. No green safety
claim derives from an unconverged metric.

Verification may require an exact result identity and optionally a converged
study. Verification diagnostics identify the study, field, value, unit,
threshold, mesh size, element/location, and convergence state.

## Workbench And MCP

The existing workbench control dock gains an Analysis section; no new permanent
status bar is introduced. Actions are:

- validate study;
- generate/show volume mesh and quality;
- run/cancel solve;
- run convergence study;
- select result field and deformation scale;
- inspect extrema and reactions;
- export VTU when present.

The workbench run action submits one `run_fem_study_intent` carrying only the
current model id, source, and authored analysis name. Rust allocates the run/job
id, applies the configured compute budgets and numerical-control policy,
validates the current study, executes and publishes the immutable result, and
returns both validation and result projections. The frontend does not mint FEM
job ids or compose validate-then-run lifecycle calls.

The validate, mesh-preview, convergence-run, and cached-convergence actions use
dedicated intents carrying only current target inputs and explicit mesh sizes
where those are user-selected. Rust allocates every job id, applies configured
budgets, numerical controls, and convergence tolerances, and owns the
validate-then-preview sequence. VTU export carries the immutable result identity
and the user-selected target path; Rust loads and enforces the configured result
byte cap. The frontend may submit intent, project the returned state, display
pending/raw failure, and cancel using the Rust-emitted job id.

The viewport can show undeformed outline, deformed boundary, Tet4 edges or a
cut/clip view, and a legend for displacement magnitude, von Mises, principal
stress, or safety factor. These are display-only overlays and material buffers.
Manufacturing geometry and export digests remain unchanged.

A specialist MCP capability exposes compact operations equivalent to
`fem_analysis_validate`, `fem_mesh_preview`, `fem_analysis_run`,
`fem_analysis_result_get`, and `fem_convergence_run`. Result reads are sliced
and default to summaries/extrema; bulk arrays route as artifacts. Existing
parameter preview tools remain the mutation mechanism.

Long meshing/solve work uses typed session activity. Interactive native output
belongs in the terminal modal. Raw backend errors are preserved; the UI must
not replace them with generic solver advice.

## Progress, Cancellation, And Budgets

Fixed stages are `resolve`, `boundary-mesh`, `volume-mesh`, `validate-mesh`,
`assemble`, `apply-constraints`, `solve`, `postprocess`, `verify`, and
`publish`. Every stage reports skipped/running/succeeded/failed/cancelled,
elapsed time, and relevant counts.

Budgets cover boundary triangles, nodes, tetrahedra, degrees of freedom,
nonzeros, worker memory estimate, solve time, result bytes, and convergence-run
count. Admission reports observed/estimated and allowed values before expensive
allocation where possible.

Cancellation terminates the Gmsh HXT/Netgen worker and cooperatively interrupts Rust
assembly/solve between bounded chunks. If the sparse backend cannot interrupt a
direct factorization, it runs behind a killable worker boundary before product
exposure claims cancellation.

## Dependency And License Decisions

- Gmsh HXT: external GPL-2-or-later executable, never linked or bundled by the
  default product. Record executable/version/hash/license evidence.
- Netgen: external OCC fallback through a separately probed Python/module
  runtime. Record interpreter/module/version/hash/license evidence.
- Fenris: MIT/Apache, selected only behind a pinned experimental assembly
  adapter; its README explicitly says it has no solver and unstable API.
- Faer: selected Rust sparse linear algebra backend behind an internal adapter.
- `mshio`: optional MIT MSH 4.1 parser for fixtures/diagnostics only.
- `vtkio`: optional MIT/Apache VTK/VTU interoperability.
- `tritet` + `with_tetgen`: rejected from default/bundle because that mode is
  AGPL.
- CalculiX: offline differential oracle only, not a runtime dependency or
  fallback.

Primary upstream references:

- <https://gmsh.info/doc/texinfo/gmsh.html#Mesh-module>
- <https://ngsolve.org/downloads>
- <https://gmsh.info/#License>
- <https://github.com/elrnv/fenris>
- <https://github.com/sarah-ek/faer-rs>
- <https://github.com/w1th0utnam3/mshio>
- <https://github.com/elrnv/vtkio>
- <https://github.com/cpmech/tritet>

## Validation Strategy

### Mathematical Unit Proof

- Tet4 shape-function partition and gradient tests.
- Signed-volume/orientation and quality fixtures.
- Constant-strain patch test.
- Rigid translation produces zero strain before constraints.
- Isotropic constitutive tensor symmetry and known uniaxial response.
- Dirichlet elimination symmetry and prescribed-value proof.
- Surface load integration sums to authored total force.
- Reaction plus applied load equilibrium.

### Integration Proof

- Axial bar displacement/stress against closed-form solution.
- Cantilever displacement trend with recorded linear-Tet tolerance.
- Bracket fixture against versioned CalculiX reference output.
- Gmsh HXT surface-group preservation, exact-BRep reconciliation, Netgen fallback,
  and local sizing.
- Singular underconstrained model rejection.
- Stale-result invalidation after one parameter edit.
- Cancel during mesh and solve with no partial result.
- Cache hit performs no meshing/assembly/solve.

### Outer UI Proof

Playwright drives a `.ecky` bracket from source preview to analysis run, field
selection, stale state after parameter change, raw failure for an underfixed
case, and manufacturing-export digest equality with result overlays enabled.

## Risks / Trade-offs

- **Linear Tet4 can be overly stiff and poor near bending/incompressibility.**
  MVP labels the element, enforces quality/convergence, and does not claim
  general high-fidelity behavior. Tet10 is a later explicit capability.
- **Stable CAD face mapping can be lost during remeshing.** Boundary groups are
  first-class and complete coverage is a hard gate; no coordinate fallback.
- **External meshers can reorder or retriangulate.** Face signatures, durable
  groups, exact source digests, and complete boundary reconciliation are hard
  gates; proximity alone never transfers a load/support group.
- **Gmsh/Netgen are external license/runtime obligations.** Probe immutable
  paths and hashes, preserve raw diagnostics, and block distribution without
  required license evidence.
- **Gmsh is GPL, not LGPL.** External invocation requires compatible whole-product
  or commercial license evidence; default distribution does not bundle it.
- **Fenris API is unstable.** Pin and hide it behind `ElementAssembler`; the
  independent oracle suite is authoritative.
- **Sparse direct factorization may exceed memory.** Estimate/budget nonzeros
  and DOFs before solve; iterative/preconditioned methods are later work.
- **Peak stress may diverge at singularities.** Use unaveraged values, expose
  location and convergence trend, and refuse silent green convergence.

## Migration And Rollback

All syntax, artifact fields, commands, and UI are additive. Models without an
`analysis` clause compile/render exactly as before and load no FEM runtime.
Disabling the FEM capability hides run actions but preserves source text and
previous derived artifacts as unavailable/stale evidence. Rollback removes no
CAD geometry, model version, or manufacturing export.

## Open Questions

None for the MVP contract. Tet10, modal/buckling, contact, nonlinear material,
and automated parameter/topology optimization require separate OpenSpec
changes after the linear-static proof gates pass.
