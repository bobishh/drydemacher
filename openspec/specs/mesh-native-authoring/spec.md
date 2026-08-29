# mesh-native-authoring Specification

## Purpose
TBD - created by archiving change mesh-native-image-authoring. Update Purpose after archive.
## Requirements
### Requirement: Typed mesh literals

The `.ecky` language SHALL provide `mesh` and `polyhedron` surface forms that
compile to one typed Core IR mesh-literal node containing finite 3D vertices and
indexed triangles. `mesh` SHALL permit open surfaces; `polyhedron` SHALL require
a closed orientable manifold.

#### Scenario: Generated closed polyhedron compiles

- **GIVEN** `.ecky` source whose `polyhedron` vertices and triangles are produced
  by finite list helpers
- **WHEN** source compilation runs
- **THEN** one typed Core IR mesh-literal node is produced
- **AND** source spans and stable AST identity remain inspectable.

#### Scenario: Open mesh compiles without pretending to be a solid

- **GIVEN** `.ecky` source containing a valid open triangle surface through
  `mesh`
- **WHEN** source compilation runs
- **THEN** compilation succeeds as mesh geometry
- **AND** its topology is not classified as a closed solid.

#### Scenario: Invalid triangle index fails before rendering

- **GIVEN** a mesh literal with a triangle index outside its vertex list
- **WHEN** source compilation or static validation runs
- **THEN** validation fails with the offending triangle and index
- **AND** no render process starts.

### Requirement: Deterministic mesh validation

The system SHALL validate mesh literals deterministically for finite
coordinates, integer index bounds, repeated triangle indices, degenerate area,
duplicate triangles, boundary edges, non-manifold edges, connected components,
and orientability before classifying their topology.

#### Scenario: Closed tetrahedron passes polyhedron validation

- **GIVEN** four consistently wound triangular faces forming one tetrahedron
- **WHEN** polyhedron validation runs
- **THEN** boundary-edge and non-manifold-edge counts are zero
- **AND** the result is classified as one closed orientable component.

#### Scenario: Missing face rejects polyhedron

- **GIVEN** a tetrahedron triangle set with one face missing
- **WHEN** polyhedron validation runs
- **THEN** validation fails with non-zero boundary-edge evidence
- **AND** the error identifies the model part and `polyhedron` operation.

#### Scenario: Same open surface remains valid mesh

- **GIVEN** the same triangle set authored through `mesh`
- **WHEN** mesh validation runs
- **THEN** rendering may continue
- **AND** structural verification reports its boundary-edge evidence as a
  printability failure rather than rewriting topology.

### Requirement: Mesh authoring resource budgets

The compiler and runtime SHALL enforce explicit configurable budgets before
allocating or evaluating mesh-literal vertices and triangles. A rejected payload
SHALL report observed and allowed counts.

#### Scenario: Literal mesh exceeds budget

- **GIVEN** source whose literal or evaluated mesh list exceeds the active
  vertex or triangle budget
- **WHEN** bounded evaluation runs
- **THEN** evaluation stops before mesh allocation exceeds the budget
- **AND** the diagnostic reports observed and allowed counts.

#### Scenario: Procedural list stays within budget

- **GIVEN** a finite formula-generated vertex and triangle list within budget
- **WHEN** bounded evaluation runs
- **THEN** it produces the same mesh digest as an equivalent literal list.

### Requirement: Mesh-native render dispatch

The system SHALL classify mesh literals as mesh-native Core IR geometry. Pure
mesh parts SHALL use the Rust mesh renderer; exact-only consumers above a closed
mesh boundary SHALL use the existing hybrid poly-BRep partition and solidify
path when that path validates successfully.

#### Scenario: Pure mesh model renders

- **GIVEN** a `.ecky` model containing one valid mesh literal and mesh-safe
  transforms
- **WHEN** preview renders
- **THEN** the Rust mesh runtime produces a viewer artifact and model STL
- **AND** no arbitrary Python or Blender process executes.

#### Scenario: Closed polyhedron enters hybrid boolean

- **GIVEN** a watertight polyhedron consumed by a supported post-boundary BRep
  boolean
- **WHEN** hybrid dispatch runs
- **THEN** the polyhedron is handed to `import-stl -> solidify`
- **AND** the boolean result must pass existing hybrid topology gates before it
  replaces the last good preview.

#### Scenario: Open mesh cannot enter solidification

- **GIVEN** an open mesh consumed by a BRep-required operation
- **WHEN** partition validation runs
- **THEN** rendering fails before OCCT boolean execution
- **AND** the diagnostic reports boundary/non-manifold evidence and the consumer
  operation.

### Requirement: Mesh artifact truth

Mesh-native renders SHALL expose topology evidence and SHALL export STL and 3MF
when corresponding viewer assets exist. STEP SHALL be offered only after a
successful closed poly-BRep solidification and SHALL be labeled faceted rather
than analytic.

#### Scenario: Pure mesh export choices

- **GIVEN** a successful pure mesh render
- **WHEN** export options are built
- **THEN** STL is available
- **AND** multipart 3MF or STL zip is available when multiple mesh parts exist
- **AND** STEP remains unavailable without a successful poly-BRep artifact.

#### Scenario: Solidified mesh exposes faceted STEP

- **GIVEN** a closed mesh successfully solidified and exported by OCCT
- **WHEN** artifact metadata is returned to UI or MCP
- **THEN** STEP path and format are present
- **AND** metadata identifies polyhedral/faceted provenance
- **AND** no response describes its faces as analytic source CAD.

### Requirement: Mesh operations remain structurally editable

Mesh-native forms SHALL participate in source maps, AST reads, guarded AST
patches, dependency inspection, and normal preview/commit flow without requiring
full-source rewrites.

#### Scenario: Agent patches one mesh binding

- **GIVEN** mesh vertices or triangles are produced from a named binding
- **WHEN** an agent inspects and validates a guarded AST replacement for that
  binding
- **THEN** affected stable node keys and dependency impact are returned
- **AND** preview occurs only after successful validation.
