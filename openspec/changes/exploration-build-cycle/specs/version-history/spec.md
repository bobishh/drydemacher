## MODIFIED Requirements

### Requirement: Every changed authoring source or persisted model draft is recoverable

The system SHALL durably record every observed version-owned authoring source
file or persisted model draft content change before validation or render. Such a
record SHALL be a draft journal event or exploration attempt unless an explicit
candidate commit creates a version. Observing unchanged content SHALL not append
a duplicate. Binary assets, caches, telemetry, and local scratch SHALL NOT create
authoring records unless they become persisted draft content.

#### Scenario: Invalid draft is retained without version promotion

- **GIVEN** working draft bytes differ from the last durable draft snapshot
- **WHEN** persistence or validation processes the draft
- **THEN** the exact bytes and raw failure remain recoverable
- **AND** no candidate or version is created implicitly
- **AND** the committed-version count remains unchanged.

#### Scenario: Unchanged observation is idempotent

- **GIVEN** a file or draft content digest equals its latest durable snapshot
- **WHEN** it is observed again
- **THEN** no duplicate draft event, attempt, candidate, or version is created.

### Requirement: Working head and version head are distinct tagged references

The system SHALL expose the latest recoverable working authoring state and the
latest explicitly committed version as distinct tagged references. It SHALL NOT
use one ambiguous `head` result for both purposes. Failed or unrenderable working
state SHALL NOT move committed version head backward or forward.

#### Scenario: Failed work advances only working head

- **GIVEN** committed version A and its working draft are current
- **WHEN** changed draft B is persisted and its attempt fails
- **THEN** working head references B
- **AND** version head remains A
- **AND** both records remain inspectable.

#### Scenario: Candidate commit advances version head

- **GIVEN** candidate C references a verified attempt
- **WHEN** the user commits C
- **THEN** version head references the new version
- **AND** its provenance references C and the exact attempt snapshot.

### Requirement: Results attach after attempt persistence

The system SHALL persist exact attempt inputs before validation/render gates and
attach pending, success, failure, superseded status, evidence, artifact, and raw
error data to that attempt. A failed result SHALL NOT create or mutate a version.

#### Scenario: Preview failure does not lose work or create version noise

- **GIVEN** a changed draft is submitted for exploration build
- **WHEN** validation or preview fails
- **THEN** the attempt already exists with exact input
- **AND** it records the raw failure outcome
- **AND** primary version history remains unchanged.

### Requirement: Versions are explicit immutable candidate commits

The system SHALL create a version only from an explicit commit request naming one
verified candidate and exact attempt/render identities. The version SHALL retain
candidate and attempt provenance and SHALL NOT combine data from a moving working
head.

#### Scenario: Attempt and candidate do not count as versions

- **GIVEN** a cycle has ten attempts and two promoted candidates
- **WHEN** version history and count are requested
- **THEN** neither attempts nor candidates are counted as committed versions.

#### Scenario: Explicit commit creates one version

- **GIVEN** one candidate has green verification for its exact snapshot
- **WHEN** the user commits it with one request ID
- **THEN** exactly one version is appended
- **AND** retries with that request ID return the same version.

### Requirement: Existing history migrates losslessly

The system SHALL keep existing history records and source data readable without
destructive reclassification. Records without reliable promotion provenance
SHALL be exposed as legacy history. New attempt, candidate, and version records
SHALL use explicit lifecycle kinds.

#### Scenario: Legacy database opens after lifecycle migration

- **GIVEN** a database containing successful and failed records under the flat
  legacy model
- **WHEN** the new lifecycle model opens it
- **THEN** every legacy record remains inspectable
- **AND** no source bytes, diagnostics, or artifacts are deleted
- **AND** new version counts can distinguish explicit commits from legacy
  records through documented projections.
