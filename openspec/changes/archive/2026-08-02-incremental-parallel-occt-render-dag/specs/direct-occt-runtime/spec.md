# Delta for direct-occt-runtime

## ADDED Requirements

### Requirement: Selective immutable BRep cache

The system SHALL cache successful immutable Direct OCCT part roots and admitted
expensive command results by resolved execution identity beneath the complete
bundle cache.

#### Scenario: Complete part hit

- **GIVEN** a validated cached analytic BRep has the current part identity and
  runtime metadata
- **WHEN** a new render requires that part
- **THEN** the runner loads the cached root
- **AND** executes zero kernel commands for the part.

#### Scenario: Expensive command hit

- **GIVEN** a cache-admitted command result has current identity
- **WHEN** a missed part root depends on that command
- **THEN** the runner loads the command result
- **AND** does not execute its dependency closure unless another miss requires
  those dependencies.

### Requirement: Localized parameter invalidation

The system SHALL invalidate only parts and command closures whose resolved
execution identities change after a parameter edit.

#### Scenario: One parameter affects one part

- **GIVEN** a three-part model whose middle part alone references parameter P
- **AND** all three parts rendered and entered the selective cache
- **WHEN** P changes
- **THEN** first and third parts are cache hits with zero executed commands
- **AND** only the middle part dirty closure executes
- **AND** the final bundle contains the updated middle part and unchanged clean
  parts.

#### Scenario: Structural parameter changes graph

- **WHEN** a parameter changes a normalized branch or repeat expansion
- **THEN** fingerprints change for the affected resolved graph
- **AND** obsolete cached nodes are not reused by positional coincidence.

### Requirement: Atomic success-only cache publication

The system SHALL publish selective cache entries only after complete successful
execution, representation validation, metadata creation, and artifact digest
creation.

#### Scenario: Kernel failure

- **WHEN** a cache-admitted command or part fails during execution or validation
- **THEN** no cache entry becomes visible
- **AND** the raw kernel failure is returned.

#### Scenario: Corrupt cache entry

- **WHEN** binary BRep bytes, metadata, runtime identity, or stored digest is
  missing or inconsistent
- **THEN** the entry is rejected as a cache miss
- **AND** normal recomputation produces a fresh atomic entry
- **AND** corrupt geometry never reaches the bundle.

### Requirement: Bounded and versioned selective cache

The system SHALL bound selective cache residency by bytes and SHALL include
cache schema, runner ABI, OCCT runtime, tolerances, imports, and tessellation
policy in admission identity.

#### Scenario: Byte budget exceeded

- **WHEN** successful entries exceed the configured byte budget
- **THEN** least-recently-used entries are evicted until within budget
- **AND** active entries remain pinned until their render completes.

#### Scenario: Backend version changes

- **WHEN** cache schema, runner ABI, or OCCT runtime identity changes
- **THEN** prior selective entries become misses
- **AND** authored source and committed model history remain untouched.

### Requirement: Clean-part tessellation reuse

The system SHALL avoid remeshing a clean cached part solely because another part
changed.

#### Scenario: Multipart localized rerender

- **GIVEN** unchanged parts have validated cached triangulation or per-part
  preview meshes
- **WHEN** another part is rebuilt
- **THEN** clean preview data is reused
- **AND** only changed geometry incurs tessellation work
- **AND** the final preview still represents every part.

### Requirement: Explicit hybrid part representation

The runner SHALL execute a planner-declared mesh-domain Boolean boundary and
cache its canonical indexed result without presenting it as analytic BRep.

#### Scenario: Decorated bracelet lid

- **GIVEN** a planner-declared `decorated-dome` mesh-domain group
- **WHEN** imported indexed relief and analytic dome are united
- **THEN** the Boolean executes in the mesh domain
- **AND** the mesh-domain partial and final lid are eligible for immutable cache
- **AND** representation participates in every cache identity.

#### Scenario: Mixed STEP export

- **GIVEN** one mesh-domain part and one or more analytic BRep parts
- **WHEN** STEP export is requested
- **THEN** the mesh-domain part is emitted as an AP242 tessellated surface set
- **AND** analytic parts retain their native BRep
- **AND** the artifact reports both representations truthfully
- **AND** no mesh-domain part is presented as analytic or faceted BRep.

### Requirement: Localized rerender performance evidence

The system SHALL prove localized parameter reuse with kernel counters and
release timing.

#### Scenario: Three-part localized benchmark

- **WHEN** only the middle-part parameter changes after a cold successful render
- **THEN** clean parts report cache hits and zero command executions
- **AND** localized rerender median is no more than 50 percent of cold full
  render median
- **AND** topology, bounds, volume, STEP, and STL contracts remain valid.
