## ADDED Requirements

### Requirement: Every changed authoring source or persisted model draft is a version

The system SHALL append one version for every observed version-owned authoring
source file or persisted model draft content change, regardless of validation,
compilation, preview, render, or artifact success. The version SHALL retain the
exact changed content and provenance. Observing unchanged content SHALL not
append a duplicate. Binary assets, capture frames, caches, telemetry, and local
scratch SHALL NOT create model versions unless they become persisted model-draft
content.

#### Scenario: Invalid file change is retained

- GIVEN a bound file whose bytes differ from its last appended snapshot
- WHEN the watcher observes and processes the file
- THEN exactly one version is appended with those exact bytes
- AND validation/render failure status and raw diagnostic are attached
- AND the version remains available in history.

#### Scenario: Unchanged observation is idempotent

- GIVEN a file or draft whose content digest equals its latest appended snapshot
- WHEN it is observed again
- THEN no new version is appended.

### Requirement: Head is the latest append

The system SHALL advance thread head to every appended version, independent of
status, artifact availability, or validation result. Head SHALL never select an
older successful version merely because a newer version failed.

#### Scenario: Failed edit becomes head

- GIVEN a successful version is current head
- WHEN a changed draft is appended and render fails
- THEN the failed version is head
- AND the prior successful version remains available through history.

### Requirement: Successful versions are a separate filter

The system SHALL expose successful-version filtering/querying independently of
head resolution. Successful filtering MAY require successful validation and a
render artifact, but SHALL NOT hide or delete other appended versions.

#### Scenario: Printable filter excludes red head

- GIVEN head is an error version and an earlier version succeeded
- WHEN successful versions are requested
- THEN the earlier successful version is returned
- AND the error head remains returned by head/history queries.

### Requirement: Stale writers append without conflicts

The system SHALL serialize concurrent or stale source/draft writers into
versions. It SHALL NOT reject an append as `conflict`, `threadAdvanced`, or
require a force flag because another writer advanced head. The last serialized
append SHALL be head and every accepted snapshot SHALL remain recoverable.

#### Scenario: Concurrent edits have no loss

- GIVEN two writers start from the same earlier snapshot
- WHEN each writes different changed content
- THEN both contents are appended as separate versions
- AND the later append is head
- AND no conflict/refusal is emitted for either append.

### Requirement: Results attach after persistence

The system SHALL persist the changed snapshot before validation/render gates and
attach success, failure, pending, evidence, artifact, and raw error data to that
version. MCP SHALL use inspect -> append -> validate -> preview -> verify
semantics, with persistence preceding validation and all outcomes attached
automatically. Generic commit/finalize tools SHALL NOT exist.

#### Scenario: Preview failure does not lose the attempt

- GIVEN a changed draft is submitted through MCP
- WHEN validation or preview fails
- THEN the snapshot version already exists
- AND the version records the exact failure outcome
- AND a later repair creates a new version instead of overwriting it.

### Requirement: Existing history migrates losslessly

The system SHALL load existing history without deleting records or source data,
derive append order deterministically, and preserve successful-version queries.

#### Scenario: Existing database opens after migration

- GIVEN a database containing successful and failed messages under the legacy
  model
- WHEN the new history model is opened
- THEN all legacy records remain inspectable
- AND head uses latest append order
- AND successful filtering returns the same successful records.

### Requirement: Missing head runtime rebuilds from durable version inputs

The system SHALL repair a missing runtime through one backend intent containing
only thread identity, version identity, and an optional expected artifact
content hash. The backend SHALL load the exact stored source and complete effective
parameter map, render them, validate coherent artifact/model identities, attach
the runtime to the same durable head version, and return its bounded workspace
projection. A caller SHALL NOT submit replacement source, parameters, artifact
bundles, or manifests during repair.

#### Scenario: Head artifact cache is missing

- GIVEN the current head retains source and parameters but its runtime file is missing
- WHEN runtime repair is requested with the observed artifact identity
- THEN the backend rebuilds from that head's exact stored inputs
- AND atomically attaches the validated artifact and manifest
- AND returns the repaired version as the selected workspace projection.

#### Scenario: Repair evidence is stale

- GIVEN the caller observed an older artifact identity
- WHEN runtime repair reaches a head with a different artifact identity
- THEN repair returns a conflict before rendering or persistence.

#### Scenario: Durable source cannot reproduce the runtime

- GIVEN a legacy imported head has no stored reproducible source
- OR the renderer returns a backend failure
- WHEN runtime repair is requested
- THEN no runtime metadata is replaced
- AND the raw source/render error is returned.
