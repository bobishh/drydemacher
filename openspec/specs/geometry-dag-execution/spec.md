# geometry-dag-execution Specification

## Purpose
TBD - created by archiving change geometry-dag-execution. Update Purpose after archive.
## Requirements
### Requirement: Direct OCCT dependency graph

The system SHALL derive a deterministic dependency graph from every positional
and keyword source reference in a Direct OCCT part plan.

#### Scenario: Nested and keyword references become dependencies

- **WHEN** a command references producer slots through positional arguments,
  nested lists, or keyword source arguments
- **THEN** every referenced producer is a dependency
- **AND** reachability retains those producers.

#### Scenario: Invalid graph rejects before execution

- **WHEN** a plan contains a missing dependency or dependency cycle
- **THEN** graph construction fails before native execution
- **AND** the error identifies the involved slot.

### Requirement: Safe difference-tool flattening

The system SHALL flatten keyword-free union producers used in difference tool
positions into ordered direct tools while preserving source semantics.

#### Scenario: Repeated cutter union becomes one multi-tool difference

- **WHEN** a difference tool is produced only by nested keyword-free unions of
  shape references
- **THEN** the optimized difference contains those ordered shape references as
  direct tools
- **AND** bypassed unreachable union commands are removed.

#### Scenario: Semantic union consumer remains

- **WHEN** a flattened union output is also referenced by a selector, keyword,
  visible result, transform, or other reachable command
- **THEN** the union command remains in the optimized plan
- **AND** its non-difference consumer keeps the original reference.

### Requirement: OCCT-owned boolean parallelism

The system SHALL execute an optimized multi-tool difference as one OCCT boolean
operation and SHALL leave boolean-internal parallel work to OCCT.

#### Scenario: One builder receives all tools

- **WHEN** the native runner executes an optimized multi-tool difference
- **THEN** one boolean builder receives the base and every ordered tool
- **AND** the builder uses the existing parallel OCCT configuration.

### Requirement: Repeated-cut performance evidence

The system SHALL provide structural and timing evidence for a real repeated-cut
model before claiming improved performance.

#### Scenario: Toothbrush Holder benchmark

- **WHEN** the opt-in Toothbrush Holder benchmark runs
- **THEN** it records fixture source digest, commands before and after, boolean
  commands, and elapsed time
- **AND** it verifies valid artifact bounds, volume tolerance, and topology
  contract against the unoptimized execution.

#### Scenario: Topology barrier parity

- **WHEN** a multi-tool difference feeds fillet or chamfer
- **THEN** the modifier remains downstream of the complete difference result
- **AND** optimized and unoptimized artifacts satisfy the same topology
  contract.
