## MODIFIED Requirements

### Requirement: Exact-BRep meshing is automatic, bounded, and identity complete

The FEM runtime SHALL invoke external Gmsh HXT automatically for admitted analytic BRep studies. It SHALL apply explicit size, quality, memory, wall-time, and thread controls; validate imported OCC surfaces against durable topology faces; and assign the checked mesh an immutable content digest.

#### Scenario: Generic authored study meshes

- **GIVEN** any synchronized analytic model with a valid authored FEM study
- **WHEN** FEM or topology execution requires a volume mesh
- **THEN** the runtime invokes Gmsh HXT without a separate caller mesh step
- **AND** every required support, load, refinement, passive-solid, and passive-void face maps exactly once
- **AND** downstream evidence binds to the checked mesh digest

#### Scenario: Runtime is unavailable

- **WHEN** no compatible Gmsh HXT executable can be resolved
- **THEN** execution fails before solve with the raw runtime diagnostic
- **AND** no triangle-soup fallback is silently substituted

### Requirement: Sparse solve is parallel, measured, and reusable

For each admitted linear-static or topology matrix, the runtime SHALL factor the SPD matrix once and reuse that factorization for all right-hand sides. Evidence SHALL record backend/version, matrix digest, right-hand-side count, thread-control mode, observed worker use, factor time, solve time, peak memory, and residuals.

#### Scenario: Generic multi-RHS fixture

- **GIVEN** a representative SPD matrix and multiple immutable right-hand sides
- **WHEN** available solver backends are compared
- **THEN** each backend receives identical numerical work
- **AND** numerical equivalence and residual gates pass before performance ranking

#### Scenario: Required parallel execution is unavailable

- **WHEN** admitted runtime policy requires parallel execution but the selected backend cannot provide or observe it
- **THEN** the run fails explicitly
- **AND** a sequential solve is not relabeled as parallel

### Requirement: Runtime config contains no model inputs

Application configuration SHALL contain only model-independent runtime policy and safety bounds. Geometry, parameters, material, supports, loads, face tags, passive regions, and topology targets SHALL come from the authored study.

#### Scenario: Two unrelated models use one runtime config

- **GIVEN** two admitted authored studies with different geometry and loads
- **WHEN** both run under the same application FEM settings
- **THEN** no runtime configuration change or Rust code change is required
