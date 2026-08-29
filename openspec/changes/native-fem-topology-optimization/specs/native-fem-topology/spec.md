## ADDED Requirements

### Requirement: Authored study defines topology inputs

The topology runtime SHALL derive geometry, material, mesh controls, supports, loads, passive regions, load-case weights, and topology targets from the selected authored `.ecky` study. Production Rust SHALL NOT contain model-specific dimensions, source templates, semantic role names, mounting layouts, or load cases.

#### Scenario: Any admitted authored study runs

- **GIVEN** an admitted exact Tet4 design domain and authored linear-static topology study
- **WHEN** topology optimization runs
- **THEN** the runtime resolves every model-specific input from that study
- **AND** the same runtime accepts another admitted study without Rust changes

#### Scenario: Runtime config is model-independent

- **WHEN** application FEM settings are applied
- **THEN** they may select or bound resources, algorithms, backends, executable paths, numeric tolerances, and safety limits
- **AND** they do not supply geometry, dimensions, tags, materials, supports, loads, passive regions, or product semantics

### Requirement: SIMP compliance minimization

The runtime SHALL minimize weighted multi-load compliance over free Tet4 design densities using authored volume fraction, penalization, minimum density, filter radius, move limit, and convergence controls.

#### Scenario: Generic cantilever improves

- **GIVEN** a bounded Tet4 cantilever design domain with authored support, load, and keepout regions
- **WHEN** topology optimization converges
- **THEN** final compliance is lower than initial compliance
- **AND** physical volume satisfies the authored target within tolerance

### Requirement: Passive regions are generic authored constraints

Passive-solid and passive-void cells SHALL be derived from authored durable face tags and depths. They SHALL be disjoint, immutable during optimization, and included correctly in physical-volume accounting.

#### Scenario: Overlap rejects before solve

- **GIVEN** one Tet4 cell classified into both passive-solid and passive-void sets
- **WHEN** topology preprocessing runs
- **THEN** the study is rejected before the first linear solve with the exact conflicting-input diagnostic

### Requirement: Deterministic filtering and sensitivity

The runtime SHALL use a deterministic volume-aware compact-support density filter and its transpose for sensitivity propagation.

#### Scenario: Constant field remains constant

- **WHEN** a constant design-density field is filtered on a nonuniform Tet4 mesh
- **THEN** the physical field remains constant within numeric tolerance

#### Scenario: Analytic derivative matches finite difference

- **WHEN** one free density in a tiny admitted mesh is perturbed by bounded epsilon
- **THEN** analytic and central finite-difference compliance derivatives agree within declared tolerance

### Requirement: Bounded MMA/GCMMA convergence

Production optimization SHALL use bounded deterministic MMA/GCMMA updates. Convergence SHALL require sustained filtered physical-density stability and bounded KKT residual. Objective delta or iteration count alone SHALL NOT establish convergence.

#### Scenario: Conservative inner approximation

- **WHEN** an inner MMA candidate is evaluated
- **THEN** exact objective and volume constraint are bounded by their accepted approximations
- **AND** a non-conservative candidate is rejected or retried within declared limits

### Requirement: One factorization serves authored load cases

Each topology iteration SHALL assemble and factor one SPD stiffness matrix and solve every admitted authored right-hand side against that factorization.

#### Scenario: Multi-load iteration

- **GIVEN** an authored study with multiple load cases sharing support constraints
- **WHEN** one topology iteration solves
- **THEN** evidence reports one factorization and all right-hand-side solves
- **AND** each residual meets the declared tolerance

### Requirement: Bounded resumable execution

The optimizer SHALL emit immutable iteration records and stop at declared iteration, solve, element, memory, result-size, cancellation, or wall-clock bounds.

#### Scenario: Step and resume equals uninterrupted run

- **WHEN** optimization pauses after an iteration and resumes from its emitted state
- **THEN** its remaining trace and final digest equal an uninterrupted run

### Requirement: Evidence boundary

Density fields, thresholded previews, tetrahedral edges, and reconstructed support graphs SHALL be labeled analysis evidence, not exact production geometry.

#### Scenario: Preview cannot masquerade as BRep

- **WHEN** optimization finishes
- **THEN** result metadata reports no exact BRep, production STEP, or engineering acceptance claim

### Requirement: Generic density support reconstruction

Reconstruction SHALL retain the dominant anchor-connected density component, report discarded active volume, derive a deterministic sparse support graph, and optionally apply authored generic smoothing and symmetry controls.

Reconstruction SHALL NOT infer semantic roles from anchor names, generate model-specific `.ecky` source, or publish exact geometry.

#### Scenario: Connected density becomes support evidence

- **GIVEN** a converged density artifact, immutable Tet4 mesh, and authored named anchor regions
- **WHEN** generic reconstruction runs
- **THEN** the result contains deterministic graph nodes, edges, anchor bindings, discarded-island evidence, and declared smoothing/symmetry metadata

#### Scenario: Density noise is rejected

- **GIVEN** active threshold islands outside the dominant anchor-connected component
- **WHEN** reconstruction runs
- **THEN** those islands are excluded and their active-volume fraction is recorded

### Requirement: Independent verification remains separate

Any exact geometry authored from topology evidence SHALL be remeshed and independently solved against its current authored studies before engineering acceptance.

#### Scenario: Authored geometry changes

- **WHEN** source geometry or fit-critical parameters change after topology evidence was produced
- **THEN** stale topology or convergence evidence cannot publish as current engineering acceptance

## MODIFIED Requirements

### Requirement: Exact-BRep meshing is automatic and identity complete

For an admitted analytic BRep, the runtime SHALL invoke external Gmsh HXT automatically, validate durable required-face mapping and mesh quality, and bind every downstream solve/result to the immutable checked mesh digest.

#### Scenario: Authored study automatically produces a checked mesh

- **GIVEN** synchronized exact BRep and authored durable FEM tags
- **WHEN** topology optimization requires a mesh
- **THEN** Gmsh HXT runs without a caller-supplied mesh artifact
- **AND** evidence records executable, controls, threads, face mapping, quality, and checked mesh identity

### Requirement: Current FEM evidence restores

The Structural Analysis window SHALL restore immutable evidence matching current source digest, model identity, authored study, mesh sequence, controls, and result digests. Restore SHALL be read-only.

#### Scenario: Matching evidence restores

- **WHEN** the panel reopens with matching immutable evidence
- **THEN** it shows the stored status and levels without starting meshing or solving

#### Scenario: Stale evidence cannot hydrate

- **WHEN** source identity, study, mesh sequence, or controls differ
- **THEN** prior evidence is reported stale or missing and is not displayed as current
