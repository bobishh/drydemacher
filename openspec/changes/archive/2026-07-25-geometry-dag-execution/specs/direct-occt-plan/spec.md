## ADDED Requirements

### Requirement: Graph-optimized direct OCCT planning

The system SHALL optimize planned command dependencies before serializing a
Direct OCCT plan without changing the part root or observable source semantics.

#### Scenario: Difference consumes repeated tools directly

- **WHEN** normalized Core IR produces a difference whose tool is a union of
  repeated cutters
- **THEN** the serialized plan contains one difference with the repeated
  cutters as direct tool operands
- **AND** no unreachable intermediate union is serialized.

#### Scenario: Topology reference prevents dead-command removal

- **WHEN** a keyword or selector source references an intermediate union
- **THEN** that union remains serialized
- **AND** the keyword or selector reference remains unchanged.

### Requirement: Toothbrush benchmark runner compatibility

The precompiled native runner SHALL admit the narrow frame and union forms
needed by the Toothbrush Holder benchmark without changing plan ABI.

#### Scenario: Plane vectors accept point3 or numeric three-item lists

- **WHEN** a runner plan uses `plane :origin`, `plane :x`, or `plane :normal`
  with a point3 or a list of three numeric values
- **THEN** the runner accepts the value as the corresponding frame vector
- **AND** any other value shape rejects with a raw argument diagnostic.

#### Scenario: Location transforms accept point3 or numeric three-item lists

- **WHEN** a runner plan uses `location :offset` or `location :rotate` with a
  point3 or a list of three numeric values
- **THEN** the runner applies the requested location transform
- **AND** generated frame placement can continue through the runner path.

#### Scenario: Singleton union is identity

- **WHEN** a runner plan evaluates a union with exactly one shape input
- **THEN** the runner returns that shape as the union result
- **AND** the plan exports through the normal native artifact path.
