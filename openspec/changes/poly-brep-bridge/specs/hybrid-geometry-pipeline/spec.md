# Delta for hybrid-geometry-pipeline

## ADDED Requirements

### Requirement: Part partition analysis

The system SHALL analyze each Core IR part tree and identify the boundary
between exact-BRep-capable operations and mesh-only operations, producing a
partition plan that drives hybrid rendering.

#### Scenario: Part with mesh-only op followed by boolean

- GIVEN a Core IR part tree containing `wall-pattern` followed by `difference`
- WHEN partition analysis runs
- THEN a boundary is identified at the `wall-pattern` node
- AND the pre-boundary sub-tree is classified as exact-BRep-capable
- AND the post-boundary ops are classified as BRep-required

#### Scenario: Pure OCCT part (no mesh ops)

- GIVEN a Core IR part tree with no mesh-only operations
- WHEN partition analysis runs
- THEN no boundary is found
- AND the part is flagged for pure OCCT rendering

#### Scenario: Pure mesh part (no post-boundary BRep ops)

- GIVEN a Core IR part tree with mesh-only ops where all subsequent ops are
  mesh-safe (translate, rotate, scale)
- WHEN partition analysis runs
- THEN a boundary is found
- AND the part is flagged with a short-circuit indicator (skip poly BRep)

#### Scenario: Mesh-only op in multiple sub-branches

- GIVEN a Core IR part tree where different sub-branches each contain a
  mesh-only operation
- WHEN partition analysis runs
- THEN multiple boundaries are identified
- AND each boundary produces a separate poly shell for fusion

### Requirement: Hybrid render dispatch

The system SHALL dispatch rendering through the hybrid pipeline when partition
analysis identifies a boundary with post-boundary BRep-required operations,
falling back to pure-OCCT or pure-mesh paths when no hybrid processing is needed.

#### Scenario: Hybrid dispatch for textured boolean part

- GIVEN a part with `wall-pattern` followed by `difference`
- WHEN render dispatch runs
- THEN the pre-boundary sub-tree renders through OCCT
- AND the mesh-only op chain renders through the mesh renderer
- AND the displaced mesh is wrapped as poly BRep
- AND the post-boundary boolean executes through OCCT

#### Scenario: Pure OCCT dispatch (no regression)

- GIVEN a part with no mesh-only operations
- WHEN render dispatch runs
- THEN the entire part renders through the existing OCCT path
- AND no hybrid pipeline overhead is incurred

#### Scenario: Pure mesh dispatch (no regression)

- GIVEN a part with mesh-only ops but no post-boundary BRep ops
- WHEN render dispatch runs
- THEN the entire part renders through the existing mesh renderer path
- AND no OCCT round-trip is incurred

### Requirement: Mesh asset interface

The system SHALL provide a unified `MeshSource` interface so that any triangle
mesh — from wall-pattern displacement, file import, or future image/AI
generation — enters the OCCT poly BRep bridge through the same path.

#### Scenario: Wall-pattern mesh through interface

- GIVEN a `wall-pattern` operation producing a displaced mesh
- WHEN the mesh is resolved through the `MeshSource` interface
- THEN a `MeshAsset` is produced containing the displaced triangle mesh
- AND the mesh asset can be wrapped as poly BRep

#### Scenario: Imported mesh through interface

- GIVEN an STL or OBJ file path
- WHEN the mesh is resolved through `MeshSource::ImportedMesh`
- THEN a `MeshAsset` is produced containing the imported triangle mesh
- AND the mesh asset can be wrapped as poly BRep

### Requirement: Non-manifold result guard

The system SHALL reject hybrid rendering results that exceed a non-manifold
edge threshold, reporting the failure with actionable diagnostics instead of
silently producing unprintable geometry.

#### Scenario: Hybrid render produces clean mesh

- GIVEN a hybrid render of a textured boolean part
- WHEN structural verification runs
- THEN the non-manifold edge count is below the threshold (< 100)
- AND the result passes verification

#### Scenario: Hybrid render produces excessive non-manifold edges

- GIVEN a hybrid render that produces > 100 non-manifold edges
- WHEN structural verification runs
- THEN verification fails with a diagnostic naming the affected part
- AND the error suggests increasing tessellation density or simplifying the
  displacement pattern

### Requirement: Representation-aware hybrid execution

The system SHALL preserve exact BRep and indexed manifold mesh as distinct
representations and SHALL choose a Boolean kernel from the required operation
and export contract instead of converting every mesh to faceted BRep.

#### Scenario: Mesh island exported as STL or 3MF

- GIVEN a validated indexed manifold mesh participates in a hybrid Boolean
- AND the requested output is STL or 3MF
- WHEN the hybrid plan executes
- THEN the local exact operands are tessellated for the mesh island
- AND the Boolean runs as one batch mesh operation
- AND the result remains an indexed manifold mesh

#### Scenario: Analytic STEP remains exact

- GIVEN a part contains only exact BRep operations
- WHEN STEP export is requested
- THEN the part remains on the OCCT path
- AND no mesh Boolean or faceted conversion occurs

#### Scenario: Faceted STEP exceeds budget

- GIVEN an imported mesh requires a faceted STEP result
- AND the projected faceted BRep face count exceeds the configured budget
- WHEN the hybrid plan is validated
- THEN execution is rejected with the projected face count and budget
- AND no hidden kernel fallback occurs

### Requirement: N-ary Boolean execution

The system SHALL submit all operands of union and difference expressions to
one kernel builder while preserving ordered head-minus-tail difference
semantics. N-way intersection SHALL retain intersection-of-all semantics and
MUST NOT be lowered as intersection with the union of tail operands.

#### Scenario: Multi-operand union

- GIVEN a union with three or more operands
- WHEN the OCCT or mesh plan executes
- THEN all operands are submitted to one n-ary builder
- AND the system does not evaluate a sequential pairwise left fold

#### Scenario: Multi-tool difference

- GIVEN a difference with one target and multiple tools
- WHEN the plan executes
- THEN the target is the sole argument
- AND all remaining operands are submitted as one ordered tool group

#### Scenario: Multi-operand intersection

- GIVEN an intersection with three or more operands
- WHEN the OCCT plan executes
- THEN the result equals the region common to every operand
- AND the operands are not grouped as `head ∩ union(tail)`

### Requirement: Deterministic hybrid reuse

The system SHALL reuse successful immutable hybrid artifacts by content and
coalesce identical concurrent work without caching failures.

#### Scenario: Warm identical render

- GIVEN a verified artifact exists for the same source, parameters, operation
  plan, mesh digests, and backend/runtime versions
- WHEN the model is rendered again
- THEN no geometry kernel process starts
- AND the verified artifact bundle is returned

#### Scenario: Concurrent identical render

- GIVEN two subscribers request the same uncached hybrid artifact
- WHEN both requests overlap
- THEN one kernel job executes
- AND both subscribers receive the same result or raw failure

### Requirement: Hybrid progress and cancellation

The system SHALL expose typed stage progress and cancellation for long-running
kernel jobs without emitting interactive kernel output into general app logs.

#### Scenario: Long Boolean reports progress

- GIVEN a hybrid Boolean is running
- WHEN the kernel advances through its stages
- THEN subscribers receive typed progress for import, validation, Boolean,
  verification, and export

#### Scenario: Last subscriber cancels

- GIVEN a shared kernel job has one remaining subscriber
- WHEN that subscriber cancels
- THEN cooperative kernel cancellation is requested
- AND an uncooperative child process is terminated
- AND no partial artifact enters the cache
