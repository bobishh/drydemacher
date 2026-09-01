## MODIFIED Requirements

### Requirement: Component extraction from existing parts

The system SHALL extract an existing part subtree into a closed
`define-component` via the compiler's binding resolution, producing copy-inline
source plus a header. Explicit ports whose frames and metadata remain resolvable
inside the extracted subtree SHALL be preserved; extraction SHALL NOT invent
ports from bounds or topology without explicit acceptance.

#### Scenario: Params become signature

- GIVEN a part whose body references model params `pole_od` and `clearance`
- WHEN `component_extract` runs against that part
- THEN the produced component signature contains both params with their
  metadata (defaults, min/max/step, labels) preserved.

#### Scenario: Scalar bindings become defaults

- GIVEN a part body referencing a scalar outer `let*` binding
- WHEN extraction runs
- THEN the binding becomes a signature entry whose default is its current
  evaluated value.

#### Scenario: Blocked extraction is explicit

- GIVEN a part body referencing a non-scalar outer binding (e.g. a shape)
- WHEN extraction runs
- THEN extraction fails with a blocker report naming the binding; no partial
  component is produced.

#### Scenario: Extracted component recompiles

- GIVEN any successful extraction
- WHEN the produced source is wrapped in a minimal model and instantiated
- THEN it compiles and plans without error.

#### Scenario: Explicit local ports survive extraction

- **GIVEN** an extracted part declares a local `mount` port whose frame depends
  only on extracted parameters and bindings
- **WHEN** extraction succeeds
- **THEN** copy-inline source and header preserve the port id, type, frame, and
  fit metadata
- **AND** a re-instantiated copy solves the same mate transform

#### Scenario: World-dependent port blocks extraction

- **GIVEN** an extracted port frame depends on an unresolved parent/world binding
- **WHEN** extraction runs
- **THEN** extraction fails naming that port and binding
- **AND** it does not silently freeze the current world coordinates as local

### Requirement: Header contract

Each stored component SHALL carry a header with name, param manifest, tags,
provenance (threadId, messageId, sourceDigest), referenced named-constraint keys,
and declared local ports including their stable ids, compatibility types, frames,
and fit metadata.

#### Scenario: Provenance recorded

- GIVEN extraction from a thread version
- WHEN the header is produced
- THEN it contains the thread id, message id, and source digest of the origin.

#### Scenario: Port interface is searchable without body source

- **GIVEN** a stored component declares a `mechanical.latch.mount.v1` port
- **WHEN** library search returns its compact header
- **THEN** the result exposes the port id and compatibility type
- **AND** does not include component body source

### Requirement: Rust owns applied workbench copy-inline import

The system SHALL apply a workbench copy-inline import through one Rust intent.
Rust SHALL own canonical source loading, stale-source validation, AST insertion,
render, immutable version persistence, runtime and manifest persistence, bound
source update, and snapshot update. Frontend SHALL NOT submit replacement model
source or chain those operations.

#### Scenario: Copy-inline component import succeeds

- **GIVEN** exact package coordinate and current thread/version/source digest
- **WHEN** workbench submits component import intent
- **THEN** Rust inserts self-contained definition and one instance through the shared materializer
- **AND** appends one successful immutable version with runtime and manifest
- **AND** returns inserted part identity and canonical source digest

#### Scenario: Bound source changed

- **WHEN** expected source digest differs from bound source
- **THEN** Rust rejects before package materialization, source write, or version append
- **AND** raw conflict includes expected and actual digests

#### Scenario: Imported model cannot render

- **WHEN** AST insertion succeeds but complete model render fails
- **THEN** Rust appends one immutable error version
- **AND** returns the raw render error without updating bound source
