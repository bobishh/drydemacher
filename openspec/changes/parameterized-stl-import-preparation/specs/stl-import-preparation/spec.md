# STL Import Preparation Specification

## ADDED Requirements

### Requirement: Backward-Compatible Parameterized STL Import

The system SHALL accept optional explicit triangle-target and maximum-error
preparation keywords on `import-stl` while preserving current behavior when the
keywords are absent.

#### Scenario: Existing import has no preparation policy

- **GIVEN** canonical source contains `(import-stl "part.stl")`
- **WHEN** the model evaluates
- **THEN** Ecky imports the original indexed mesh without simplification
- **AND** output and cache identity remain backward compatible

#### Scenario: Import has a bounded preparation policy

- **GIVEN** target triangle count and maximum error are both valid
- **WHEN** `import-stl` evaluates
- **THEN** Ecky deterministically prepares a derived indexed mesh
- **AND** measured maximum deviation does not exceed the authored error
- **AND** original STL bytes remain unchanged

#### Scenario: Preparation policy is incomplete

- **WHEN** only target triangles or only maximum error is authored
- **THEN** compilation fails with the exact missing paired keyword
- **AND** no derived asset is cached

### Requirement: Hard Error Bound and Soft Triangle Target

The system SHALL treat maximum geometric error as a hard constraint and target
triangle count as a desired bound.

#### Scenario: Target is reachable

- **WHEN** simplification reaches the requested count within topology and error
  constraints
- **THEN** achieved count is at or below target
- **AND** achieved maximum error is at or below the authored maximum

#### Scenario: Target is not reachable

- **WHEN** further reduction would violate error, boundary, component, or
  manifold constraints
- **THEN** Ecky returns the valid higher-count mesh
- **AND** reports typed `targetNotReached` with requested and achieved metrics
- **AND** never increases the authored error bound

### Requirement: Immutable Raw Source and Derived Provenance

The system SHALL preserve raw imported source identity separately from prepared
mesh identity.

#### Scenario: Prepared import succeeds

- **THEN** artifact provenance includes raw digest, prepared digest, original and
  achieved counts, requested and achieved error, protected counts, algorithm
  version, and cache state
- **AND** the prepared cache artifact is indexed mesh data, not a rewritten
  source STL

#### Scenario: Policy changes

- **WHEN** target, error, boundary policy, protected set, or algorithm version
  changes
- **THEN** prepared cache identity changes
- **AND** raw source digest remains unchanged

### Requirement: Source Anchors Remain Raw-Source Bound

The system SHALL resolve Crop, Guides, and Surface Trim mesh anchors against the
raw imported source even when preparation is enabled.

#### Scenario: Surface Trim wraps a prepared import

- **GIVEN** stored anchors reference raw source digest and triangle indices
- **WHEN** the model evaluates Surface Trim and import preparation
- **THEN** anchors reconstruct against the raw indexed mesh
- **AND** trimming/capping completes before preparation materializes
- **AND** inserted boundary and cap vertices are protected during preparation

#### Scenario: Raw source changes

- **WHEN** source bytes no longer match the stored raw digest
- **THEN** anchor replay and preparation fail with source-digest conflict
- **AND** no stale prepared asset or alternate mesh is used

### Requirement: Explicit Solidify Boundary

The system SHALL keep import preparation separate from `solidify`.

#### Scenario: Mesh-only output

- **WHEN** prepared imported mesh feeds mesh Boolean or STL/3MF export
- **THEN** Ecky uses prepared indexed geometry without BRep conversion

#### Scenario: BRep operation follows import

- **WHEN** canonical source explicitly wraps prepared import with `solidify`
- **THEN** the prepared closed mesh crosses into faceted BRep
- **AND** downstream OCCT Boolean may consume that solid
- **AND** artifact provenance does not claim analytic source geometry

### Requirement: External Shapes Import Detail Controls

The system SHALL expose preparation as canonical parameters of the exact selected
`import-stl` node in External Shapes Import.

#### Scenario: User previews prepared detail

- **GIVEN** one bound imported source is selected
- **WHEN** user chooses Prepared, target triangles, and maximum deviation
- **THEN** Viewer shows a derived preview and achieved metrics
- **AND** canonical source remains unchanged until Apply
- **AND** visible task state reports import, validation, preparation, and preview

#### Scenario: User applies prepared detail

- **WHEN** user applies a green prepared preview
- **THEN** backend AST-patches the exact import node using thread, message, source
  digest, and node guards
- **AND** canonical source contains the normalized import keywords
- **AND** reopening External Shapes derives controls from that source

#### Scenario: User resets to original

- **WHEN** user selects Original and applies
- **THEN** backend removes only preparation keywords from the exact import node
- **AND** original STL path and surrounding operations remain unchanged

#### Scenario: Preparation fails

- **WHEN** backend rejects source, policy, topology, error, cache, or cancellation
- **THEN** raw backend error is visible in the active task
- **AND** canonical source and last green artifact remain unchanged

### Requirement: No External Preparation Dependency

The system SHALL execute STL preparation in the Ecky runtime and SHALL NOT
require an agent-side Python, Blender, Meshlab, or rewritten derivative STL
workflow.

#### Scenario: Dense STL is prepared in-app

- **WHEN** user previews and applies a valid preparation policy
- **THEN** all derived geometry is produced by the versioned Ecky runtime
- **AND** progress, cancellation, cache, provenance, and failure remain visible
  through normal application surfaces
