# Delta for fem-analysis-authoring

## ADDED Requirements

### Requirement: Structural studies are typed non-geometry declarations

The `.ecky` language SHALL support named top-level `analysis` declarations for
3D linear-static structural studies. An analysis SHALL reference authored
geometry but SHALL NOT produce a shape, participate in a Boolean, or alter
manufacturing geometry.

#### Scenario: Bracket study compiles with its model

- **GIVEN** a model contains one closed solid part and one named linear-static
  analysis referencing that part
- **WHEN** source compilation and semantic validation run
- **THEN** geometry Core IR and typed analysis metadata are both produced
- **AND** the analysis retains source spans and stable node identity
- **AND** removing only the analysis leaves geometry and export digests
  unchanged.

#### Scenario: Analysis is used as geometry

- **GIVEN** an analysis form is supplied to a shape operation or part result
- **WHEN** type checking runs
- **THEN** compilation fails with expected shape versus analysis kinds
- **AND** no render or solver process starts.

### Requirement: MVP physics and element scope is explicit

A linear-static analysis SHALL admit exactly one connected closed 3D solid
region, one homogeneous isotropic material, and first-order four-node
tetrahedral elements. Unsupported physics, domains, materials, or elements
MUST fail before meshing.

#### Scenario: Supported Tet4 study validates

- **GIVEN** one closed solid, one isotropic material, Tet4 mesh controls, at
  least one displacement constraint, and at least one supported load
- **WHEN** analysis validation runs
- **THEN** the study is admitted as small-strain small-displacement linear
  elasticity.

#### Scenario: Unsupported contact or nonlinear study is authored

- **GIVEN** source requests contact, shells, beams, Tet10, plasticity, large
  deformation, modal, buckling, thermal, fatigue, transient, or topology
  optimization behavior
- **WHEN** analysis validation runs
- **THEN** the study is rejected with the unsupported form and source span
- **AND** it is not silently approximated by the linear Tet4 solver.

### Requirement: Structural quantities are dimension checked

The compiler SHALL normalize force, stress/modulus, mass density, displacement,
and dimensionless strain/material quantities into canonical FEM units and
SHALL reject dimensionally invalid structural inputs even in otherwise
permissive CAD-unit mode.

#### Scenario: Valid material and load units normalize

- **GIVEN** Young's modulus and yield strength use stress units, density uses
  mass-per-volume units, displacement uses length, and a total surface load uses
  force
- **WHEN** analysis values compile
- **THEN** they normalize to MPa, kg/mm³, mm, and N respectively
- **AND** normalized dimension metadata remains available to diagnostics,
  manifests, UI, and verification.

#### Scenario: Pressure receives force instead of stress

- **GIVEN** a pressure boundary condition is assigned a force-valued scalar
- **WHEN** strict FEM dimension validation runs
- **THEN** compilation fails with the study, field, source span, expected stress
  dimension, and actual force dimension
- **AND** no implicit area or unit conversion is invented.

### Requirement: Analysis intent, assumptions, and evidence are explicit

Every structural study SHALL identify its engineering question, decision
metrics, acceptance criteria, geometry idealization, and assumption ledger.
Every material, load, support, connection, and decision-critical tolerance SHALL
carry authored provenance and uncertainty or an explicit `unknown` state. Agent
proposals SHALL NOT become authoritative engineering inputs without recorded user
or evidence acceptance.

#### Scenario: User authors a supported bracket question

- **GIVEN** study asks for displacement and elastic stress under one named load
  case
- **WHEN** analysis validation runs
- **THEN** study records acceptance metrics, exact included solid, declared
  idealizations, linear-static assumptions, material source/condition, load
  source/frame/distribution, support rationale, and uncertainty
- **AND** every record retains source span and identity in study digest

#### Scenario: Agent guesses a missing load or material

- **GIVEN** source geometry has no authoritative load magnitude, support model,
  or material property evidence
- **WHEN** agent proposes a study using values inferred from appearance or a
  generic material name
- **THEN** values remain proposed/unknown and cannot support a green verification
- **AND** system requests targeted evidence or explicit user acceptance
- **AND** no hidden default enters assembly

#### Scenario: Geometry is idealized for analysis

- **GIVEN** study removes a fillet, hole, cosmetic feature, or other exact CAD
  region from analysis domain
- **WHEN** idealization is admitted
- **THEN** it records source and analysis geometry identities, affected topology,
  justification, dimensional threshold, expected influence, and user approval
- **AND** manufacturing BRep remains unchanged
- **AND** result cannot be attributed to unmodified geometry without the
  idealization record

#### Scenario: Unsupported connection physics is required

- **GIVEN** load path depends on contact, friction, fastener, adhesive, weld, or
  multi-body interface behavior
- **WHEN** MVP one-solid study is validated
- **THEN** applicability fails as unsupported before meshing
- **AND** agent cannot replace interface with fixed faces or bonded behavior
  silently

### Requirement: Isotropic material data is physically validated

The system SHALL require finite positive Young's modulus and density, SHALL
require `-1 < poissonRatio < 0.5`, and SHALL require finite positive yield
strength whenever yield-based safety factor is requested.

#### Scenario: Valid isotropic material is accepted

- **GIVEN** finite positive modulus, density, yield strength, and Poisson ratio
  in the open physical interval
- **WHEN** material validation runs
- **THEN** one normalized isotropic material record is bound to the analysis
  domain.

#### Scenario: Nearly or fully incompressible invalid value is supplied

- **GIVEN** Poisson ratio is non-finite, less than or equal to -1, or greater
  than or equal to 0.5
- **WHEN** material validation runs
- **THEN** validation fails with the value and admitted interval
- **AND** assembly does not clamp or replace it.

### Requirement: Boundary conditions bind through durable face selectors

The compiler SHALL require fixed, prescribed-displacement, surface-force,
traction, and pressure forms to target existing face selectors/tags and SHALL
resolve them to durable face evidence for the declared part before volume
meshing or assembly.

#### Scenario: Tagged support survives a parameter edit

- **GIVEN** a fixed condition references an authored face tag
- **AND** a parameter edit reorders backend face indices while the tag rebinds
  uniquely
- **WHEN** the study is resolved again
- **THEN** the condition targets the rebound durable face evidence
- **AND** the resolved ids and rebind diagnostic enter analysis identity.

#### Scenario: Face selector is missing or ambiguous

- **GIVEN** a load or constraint selector resolves to zero faces, ambiguous
  durable faces, or a face from another part
- **WHEN** study resolution runs
- **THEN** the study fails before volume meshing
- **AND** the diagnostic identifies study, condition, part, authored selector,
  and candidate target ids.

### Requirement: Surface load semantics are unambiguous

A total surface force SHALL preserve the authored global resultant independent
of boundary triangulation density. Traction SHALL be a global force-per-area
vector. Positive scalar pressure SHALL act inward against outward-oriented
boundary normals. Prescribed displacement SHALL explicitly identify constrained
components.

#### Scenario: Total force spans a refined face

- **GIVEN** a total surface force is applied to a tagged face
- **AND** local mesh refinement changes the number and area of boundary
  triangles
- **WHEN** load assembly runs
- **THEN** assembled nodal loads sum to the same authored global force within
  numerical tolerance.

#### Scenario: Pressure acts on a curved boundary

- **GIVEN** positive pressure is assigned to an outward-oriented curved face
- **WHEN** boundary loads are integrated
- **THEN** each contribution acts opposite its outward normal
- **AND** the resultant and moment are reported for equilibrium inspection.

### Requirement: Volume mesh controls are authored analysis intent

An analysis SHALL support Tet4 global size and bounded face-local refinement
controls. Mesh controls SHALL reference durable faces where local and SHALL be
part of analysis/cache identity without changing CAD tessellation or
manufacturing exports.

#### Scenario: Local refinement follows a tagged load pad

- **GIVEN** a volume mesh has a global size and a smaller size on a tagged face
- **WHEN** the tagged face resolves and volume meshing runs
- **THEN** the mesher receives a tagged local size control
- **AND** the resulting mesh records requested controls and resolved face ids.

#### Scenario: Mesh budget is invalid

- **GIVEN** size, min/max size, element order, node/cell budget, or quality
  threshold is non-finite, non-positive, inconsistent, or unsupported
- **WHEN** analysis validation runs
- **THEN** validation fails before a mesher process starts
- **AND** reports the observed value and admitted constraint.

### Requirement: FEM results cannot drive same-version geometry

Structural results SHALL be post-render values and MUST NOT be read by geometry
expressions in the same model version. Parameter optimization SHALL operate as
an outer sequence of parameter edit, geometry preview, FEM run, and metric
inspection.

#### Scenario: Geometry attempts to read stress

- **GIVEN** a part dimension references a `fem-max`, displacement, stress, or
  safety-factor result from its own model version
- **WHEN** dependency validation runs
- **THEN** compilation fails with an analysis-to-geometry cycle
- **AND** no stale prior result is substituted.

#### Scenario: External parameter sweep uses FEM metrics

- **GIVEN** an orchestrator chooses a parameter set, previews geometry, and runs
  its analysis
- **WHEN** it reads mass, stress, and displacement summaries
- **THEN** those values are bound to the exact parameter/geometry/result digest
- **AND** a subsequent parameter choice creates a new identity rather than
  mutating anonymous FEM nodes as CAD.

### Requirement: Authored verification can consume current FEM metrics

The verification language SHALL expose typed mass, displacement, reaction,
stress, and safety-factor metrics from a named study. A metric SHALL pass only
with a successful non-stale result matching the current analysis identity and
any required convergence policy.

#### Scenario: Current converged stress check passes

- **GIVEN** the current study result is successful and its required stress
  convergence criterion is green
- **AND** unaveraged maximum von Mises stress is below the authored threshold
- **WHEN** generated-model verification runs
- **THEN** the check passes with value, unit, threshold, mesh/result identity,
  and extremum location evidence.

#### Scenario: Old result exists after geometry edit

- **GIVEN** a previous study result exists
- **AND** current parameters or geometry changed its analysis identity
- **WHEN** FEM verification runs
- **THEN** the metric is stale and cannot pass
- **AND** the diagnostic identifies the old and required identities.
