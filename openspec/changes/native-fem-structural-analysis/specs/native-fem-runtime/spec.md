# Delta for native-fem-runtime

## ADDED Requirements

### Requirement: Analysis boundary meshes preserve CAD-face provenance

The runtime SHALL derive a dedicated closed analysis boundary surface directly
from the current Direct OCCT solid and SHALL retain one durable CAD-face group
for every triangle. It MUST NOT use STL as the boundary handoff or treat the
manufacturing `IndexedMeshAsset` as a volume mesh.

The runtime SHALL also record source BRep vertex/edge/face/loop incidence and
prove corresponding boundary-mesh adjacency. Tessellation tolerance controls
geometric error only; it MUST NOT merge, invent, or drop semantic face groups.

#### Scenario: Tagged bracket boundary is emitted

- **GIVEN** a valid closed Direct OCCT bracket with tagged support and load faces
- **WHEN** the analysis boundary is tessellated
- **THEN** every oriented triangle references one recorded face group
- **AND** each group carries canonical/durable ids, aliases, source provenance,
  area evidence, tessellation policy, and source geometry digest.

#### Scenario: Boundary provenance is incomplete

- **GIVEN** selected CAD face area has missing, ambiguous, or ungrouped boundary
  triangles beyond tolerance
- **WHEN** analysis-boundary validation runs
- **THEN** volume meshing is rejected with expected and observed group evidence
- **AND** no coordinate guess, STL round-trip, or hidden regrouping occurs.

### Requirement: Model applicability is checked before and after solve

The runtime SHALL evaluate deterministic applicability gates separately from
mesh quality, algebraic solve success, and mesh convergence. It SHALL record
characteristic dimensions and configured thresholds for thin/slender Tet4 risk,
near-incompressible locking risk, rigid/support realism, concentrated
load/support singularities, displacement-to-size ratio, and elastic-range
validity. A failed applicability gate SHALL prevent a green engineering check
without deleting the numerical result evidence.

#### Scenario: Tet4 solid model is inappropriate for thin bending domain

- **GIVEN** characteristic thickness and span indicate unresolved thin/slender
  behavior under current element sizes
- **WHEN** pre-solve applicability audit runs
- **THEN** study is rejected or marked unsupported for decision use with
  measured ratios and required capability/refinement
- **AND** agent cannot silence risk by changing display or convergence metric

#### Scenario: Linear solution leaves admitted regime

- **GIVEN** solve residual and equilibrium pass
- **WHEN** maximum displacement ratio exceeds small-displacement threshold or
  stress exceeds declared elastic applicability range
- **THEN** numerical result remains inspectable
- **AND** applicability status is failed with values, locations, and thresholds
- **AND** result cannot produce a green linear-elastic safety decision

#### Scenario: Hotspot is caused by boundary idealization

- **GIVEN** stress maximum remains attached to a constrained edge, point-load
  surrogate, or shrinking support/load footprint during refinement
- **WHEN** singularity analysis runs
- **THEN** hotspot is classified as suspected boundary-condition singularity
- **AND** peak stress remains unusable for green acceptance
- **AND** displacement and reaction metrics retain independent statuses

### Requirement: Sensitivity and validation are distinct from convergence

The system SHALL distinguish solver verification, mesh convergence, parameter
sensitivity, input uncertainty, and physical/reference validation. Agreement
between meshes or with another implementation SHALL NOT imply that authored
loads, supports, material properties, connections, or idealizations match the
physical product.

#### Scenario: Mesh converges but load uncertainty dominates

- **GIVEN** selected displacement/stress metrics are mesh-converged
- **AND** admitted load magnitude or material modulus has non-zero uncertainty
- **WHEN** study decision evidence is assembled
- **THEN** system reports bounded response sensitivity/range and dominant inputs
- **AND** nominal result alone cannot hide decision-changing uncertainty

#### Scenario: Offline solver comparison agrees

- **GIVEN** Ecky result agrees with versioned CalculiX reference within tolerance
- **WHEN** validation status is reported
- **THEN** agreement is labeled implementation/differential verification
- **AND** physical validation remains absent unless test or qualified reference
  evidence with matching setup is attached

### Requirement: Analysis boundary admission is strict

An analysis boundary SHALL contain finite non-degenerate consistently oriented
triangles forming exactly one closed orientable non-self-intersecting component
with positive enclosed volume and valid face-group cardinality.

#### Scenario: Closed manifold boundary passes

- **GIVEN** all triangles are finite and form one outward-oriented closed solid
- **WHEN** boundary admission runs
- **THEN** boundary-edge and non-manifold-edge counts are zero
- **AND** positive volume, component count, face-group areas, and content digest
  are recorded.

#### Scenario: Open or non-manifold boundary fails

- **GIVEN** the boundary has a missing face, non-manifold edge, degenerate
  triangle, winding mismatch, invalid group index, or non-positive volume
- **WHEN** admission runs
- **THEN** meshing fails before the Gmsh HXT/Netgen mesher starts
- **AND** the diagnostic includes observed topology counts and offending ids.

#### Scenario: Tessellation changes semantic topology

- **GIVEN** output boundary is geometrically close but drops a BRep face group,
  changes group adjacency, or merges two source groups
- **WHEN** analysis-boundary admission runs
- **THEN** boundary is rejected with source/output incidence evidence
- **AND** coordinate proximity cannot satisfy topology equivalence

### Requirement: Gmsh HXT tetrahedralization is external and isolated

The product SHALL run an explicitly probed external Gmsh HXT executable through
a dedicated bounded worker to produce first-order tetrahedra from an exact BRep.
If HXT fails, an explicitly probed Netgen OCC exact-BRep worker MAY run within
the remaining budget. The default path MUST NOT invoke FreeCAD, CalculiX,
TetGen, arbitrary Python, network, cloud service, or untagged STL fallback;
Python is allowed only through the explicitly probed Netgen adapter.

#### Scenario: Available Gmsh HXT runtime meshes a bracket

- **GIVEN** Gmsh executable identity matches its probed path, version, digest,
  platform, architecture, adapter protocol, and supported Tet4 capability
- **WHEN** a valid tagged boundary and mesh controls are submitted
- **THEN** the worker returns typed nodes, Tet4 cells, tagged boundary facets,
  runtime identity, insertion/approximation evidence, and meshing diagnostics
- **AND** no compatibility process is invoked unless HXT fails and the explicit
  Netgen fallback is available.

#### Scenario: Gmsh HXT runtime is unavailable or crashes

- **GIVEN** the configured Gmsh executable is missing/mismatched or the worker
  exits unsuccessfully
- **WHEN** capability probing or meshing runs
- **THEN** the operation fails with raw probe/stdout/stderr/exit detail
- **AND** Netgen starts only when its interpreter/module identity was explicitly
  probed and HXT failed before the remaining budget was exhausted
- **AND** no partial volume mesh enters cache.

#### Scenario: Exact BRep face identity is not preserved

- **GIVEN** Gmsh or Netgen output omits, reorders, or ambiguously maps an exact
  OCC face group
- **WHEN** boundary reconciliation runs
- **THEN** the adapter rejects the mesh with source/output face evidence
- **AND** no numeric proximity or lossy tag remap can satisfy the mapping.

### Requirement: Volume meshes retain boundary groups and pass quality gates

`FemVolumeMesh` SHALL store finite nodes, oriented Tet4 connectivity, boundary
triangles, source face groups, quality evidence, source boundary digest,
mesher identity, and content digest. Every admitted exterior facet SHALL belong
to exactly one tetrahedron and one valid boundary group.

#### Scenario: Locally refined mesh preserves groups

- **GIVEN** a smaller size is requested on a tagged load face
- **WHEN** Gmsh HXT or its explicit Netgen fallback completes and validation runs
- **THEN** refined exterior facets remain assigned to that source face group
- **AND** complete boundary area coverage is within configured tolerance.

#### Scenario: Retriangulated boundary cannot be reconciled

- **GIVEN** output exterior facet is missing, maps to multiple source groups,
  crosses group adjacency, or exceeds configured envelope
- **WHEN** boundary reconciliation runs
- **THEN** mesh is rejected before assembly
- **AND** propagated tag alone cannot override failed geometric/topological proof.

#### Scenario: Invalid tetrahedron is returned

- **GIVEN** mesher output contains non-finite coordinates, missing references,
  repeated cell nodes, duplicate cells, zero/negative volume, invalid exterior
  ownership, disconnected cells, or quality below the configured threshold
- **WHEN** volume-mesh validation runs
- **THEN** the mesh is rejected before assembly
- **AND** the error reports worst element/location, metric, observed value, and
  threshold.

### Requirement: Meshing is reproducible and identity complete

The runtime SHALL set explicit Gmsh HXT/Netgen options/thread policy, SHALL
canonicalize checked connectivity before digesting, and SHALL include every
geometry, boundary, control, adapter, native runtime, and tolerance input in
mesh identity.

#### Scenario: Identical reproducible request is repeated

- **GIVEN** structured boundary, controls, runtime, and tolerances are identical
- **WHEN** the request is served from cache or recomputed
- **THEN** canonical mesh identity is identical
- **AND** a valid cache hit performs no Gmsh HXT or Netgen execution.

#### Scenario: Native runtime or local size changes

- **WHEN** the Gmsh HXT/Netgen runtime identity or one local mesh control changes
- **THEN** the prior mesh is not reused under the new request
- **AND** both identities remain distinguishable in diagnostics.

### Requirement: Tet4 assembly implements small-strain isotropic elasticity

The Rust FEM core SHALL assemble the symmetric global stiffness matrix for
three displacement DOFs per node from valid Tet4 elements and one valid
isotropic linear-elastic material. Experimental library types SHALL remain
behind internal adapters.

#### Scenario: Constant-strain patch is assembled

- **GIVEN** an affine displacement field on a valid multi-tetrahedron patch
- **WHEN** element gradients, strains, stresses, and global stiffness assemble
- **THEN** constant strain/stress match the analytical field within recorded
  tolerance
- **AND** rigid translation/rotation contribute zero strain within tolerance.

#### Scenario: Unsupported element or constitutive model reaches assembly

- **GIVEN** a non-Tet4 cell, multiple material region, invalid material, or
  unsupported nonlinear operator reaches the adapter
- **WHEN** assembly admission runs
- **THEN** it fails explicitly
- **AND** no approximation or element downgrade occurs.

### Requirement: Loads and displacement constraints are assembled exactly

The runtime SHALL apply fixed/component-wise prescribed displacement by
elimination and SHALL integrate total surface force, traction, and inward
pressure over source-grouped boundary facets. It MUST NOT use penalty supports,
hidden springs, or nodal coordinate guesses.

#### Scenario: Non-zero prescribed displacement is applied

- **GIVEN** selected boundary DOFs have finite authored displacements
- **WHEN** the reduced system is formed
- **THEN** prescribed values are satisfied exactly within solver tolerance
- **AND** matrix symmetry is retained
- **AND** reactions can be reconstructed from the unreduced system.

#### Scenario: Refined total-force boundary is assembled

- **GIVEN** the same total force and face group on two admitted refinement
  levels
- **WHEN** each load vector assembles
- **THEN** both nodal load resultants equal the authored force within tolerance
- **AND** triangle count does not multiply the total load.

### Requirement: Sparse solution rejects invalid structural systems

The runtime SHALL solve the reduced sparse symmetric system through an internal
`LinearSolver` adapter and SHALL accept a result only when displacement is
finite, normalized residual passes, applied/reaction equilibrium passes, and
strain energy is non-negative within numerical tolerance.

#### Scenario: Adequately constrained bracket solves

- **GIVEN** a valid assembled SPD reduced system within budgets
- **WHEN** sparse direct factorization and solve complete
- **THEN** finite displacement is reconstructed
- **AND** residual, equilibrium, energy, solver method, ordering, and tolerances
  are recorded.

#### Scenario: Model retains rigid-body motion

- **GIVEN** supports do not remove all rigid-body modes or a hidden mechanism
  makes the system singular
- **WHEN** pre-solve checks or factorization run
- **THEN** no result artifact is published
- **AND** the error identifies likely unconstrained modes/DOFs or raw
  factorization evidence
- **AND** no hidden stiffness is added.

### Requirement: Structural post-processing separates checked and display fields

The runtime SHALL compute per-element small strain, Cauchy stress, unaveraged
von Mises stress, displacement, reactions, volume, mass, and optional
Yield/von-Mises safety factor. Nodal-averaged display stress SHALL be labeled
and MUST NOT replace unaveraged stress for verification extrema.

#### Scenario: Stress field is published

- **GIVEN** a successful current solve
- **WHEN** post-processing runs
- **THEN** element stress/von-Mises and nodal display fields are separately
  identified
- **AND** maximum verification stress uses the unaveraged element field
- **AND** its value, element, coordinate, unit, and mesh/result identity are
  recorded.

#### Scenario: Zero stress safety factor is encoded

- **GIVEN** an element has zero von Mises stress and valid yield strength
- **WHEN** safety factor is computed and serialized
- **THEN** the result uses an explicit typed unbounded representation
- **AND** does not emit non-standard JSON `Infinity` or divide-by-zero noise.

### Requirement: FEM result artifacts are immutable, bounded, and stale-aware

A successful `FemResultAsset` SHALL be atomically bound to exact source,
parameters, geometry, selector resolution, boundary mesh, volume mesh,
material, loads, constraints, numerical settings, and runtime identities. Large
arrays SHALL use validated bounded sidecars rather than thread-message payloads.

#### Scenario: Exact result cache hit

- **GIVEN** a complete validated result exists for the exact current identity
- **WHEN** the same analysis is requested
- **THEN** no boundary meshing, Gmsh HXT/Netgen, assembly, factorization, or post-processing
  executes
- **AND** the immutable result is returned.

#### Scenario: One parameter changes

- **GIVEN** a prior result exists
- **WHEN** a geometry, load, material, selector, or mesh-affecting parameter
  changes
- **THEN** the prior result is marked stale for the current study
- **AND** cannot satisfy current verification
- **AND** remains attributable to its old identity rather than being rewritten.

#### Scenario: Result sidecar is corrupt

- **GIVEN** a sidecar has a bad digest, range, scalar type, shape, non-finite
  data, unsupported schema, or oversized byte count
- **WHEN** the artifact is read
- **THEN** it is rejected as corrupt/unavailable
- **AND** no partial field reaches verification or the viewport.

### Requirement: Long FEM work is bounded, observable, and cancellable

The runtime SHALL expose typed ordered stages for resolve, boundary mesh,
volume mesh, validation, assembly, constraints, solve, postprocess,
verification, and publication. It SHALL enforce explicit memory/count/time
budgets and cancel without orphan workers or partial artifacts.

#### Scenario: User cancels during volume meshing

- **GIVEN** a Gmsh HXT or Netgen worker is generating a mesh
- **WHEN** the final subscriber cancels
- **THEN** the worker is terminated or cooperatively stopped
- **AND** the job reports cancelled
- **AND** no partial mesh/result/cache entry becomes visible.

#### Scenario: Estimated DOFs exceed budget

- **GIVEN** an admitted mesh would create more DOFs or sparse nonzeros than the
  configured limit
- **WHEN** solve admission runs
- **THEN** execution stops before factorization allocation
- **AND** reports estimated/observed and allowed values.
