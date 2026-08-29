# Delta for render-snapshot-authority

## MODIFIED Requirements

### Requirement: Authoring actor owns draft transitions

The system SHALL serialize every changed draft append through one authoring
actor mailbox while render and verification workers remain parallel and
stateless. Worker results SHALL attach to the version that requested them and
SHALL NOT change append order or move head backward.

#### Scenario: Newer revision wins out-of-order completion

- GIVEN draft versions 7 and 8 were appended in that order
- AND render workers process both versions
- WHEN version 8 completes before version 7
- THEN version 8 remains head
- AND late version 7 evidence attaches only to version 7
- AND neither version is discarded from history.

#### Scenario: Different draft actors run concurrently

- GIVEN thread A and thread B have independent authoring actors
- WHEN both append changed drafts and request rendering
- THEN their render workers may execute concurrently
- AND each thread head follows only its serialized append order.

#### Scenario: Worker failure stays scoped

- GIVEN one render worker fails for the current version in thread A
- WHEN supervisor records the failure
- THEN that version remains thread A head with raw failure evidence
- AND thread A may project its last successful snapshot separately
- AND thread B continues processing messages.

### Requirement: Verification outcome is bound to the same snapshot

The system SHALL append changed snapshot content before verification. A green
explicit verification record naming the same snapshot and artifact digest SHALL
mark that version successful. Missing, red, or stale verification SHALL mark the
already-appended version failed and SHALL NOT erase it or reject its history
append.

#### Scenario: Verified snapshot updates its version automatically

- GIVEN snapshot A was appended and has a green explicit verification record
- WHEN verification targets snapshot A
- THEN version A is marked successful
- AND its saved inputs and outputs remain unchanged
- AND no content-identical duplicate version is appended.

#### Scenario: Changed snapshot invalidates verification

- GIVEN snapshot A has green verification
- AND changed source, parameters, backend, or artifact produce appended snapshot
  B
- WHEN a consumer requests successful projection for B without new verification
- THEN B is marked failed with stale-verification evidence
- AND B remains head and inspectable history
- AND A remains available through the successful-version filter.

### Requirement: Runtime cache is bound to durable version inputs

The system SHALL address a version runtime by durable version ID plus a
canonical digest of every effective render input. Artifact identity SHALL remain
separate from version-input identity. A cache hit SHALL require both version ID
and digest to match; `modelId`, source digest, or artifact path alone SHALL NOT
authorize reuse.

#### Scenario: Any effective parameter changes cache identity

- GIVEN durable version A contains a complete effective parameter map
- WHEN any parameter value changes and version B is appended
- THEN A and B have different version-input digests
- AND B cannot reuse A's version runtime association
- AND unchanged parameters serialized in another map insertion order retain the
  same canonical digest.

#### Scenario: Post-processing cannot mutate another version cache

- GIVEN versions A and B have equal source and base geometry parameters
- AND their post-processing inputs differ
- WHEN both versions render
- THEN their version-input digests and runtime ownership differ
- AND rendering B does not overwrite or post-process A's cached artifacts.

#### Scenario: Artifact store may deduplicate without aliasing versions

- GIVEN two durable versions produce byte-identical immutable artifacts
- WHEN the artifact store deduplicates those bytes
- THEN each version retains its own `(durableVersionId, versionInputDigest)`
  runtime association
- AND loading either version verifies that association before exposing the
  shared artifact.

### Requirement: Frontend state is a projection of active snapshot

The workbench SHALL expose the latest appended version as head. Source and
failure state SHALL project from head; render-dependent controls and viewport
MAY project the latest successful snapshot when head has no valid artifact, but
that fallback SHALL be labeled separately and SHALL NOT redefine head.

#### Scenario: Agent preview updates every projection together

- GIVEN successful snapshot A is visible
- WHEN a valid agent draft appends and publishes successful snapshot B
- THEN code, parameters, viewport artifact, manifest, and head project B
- AND no projection retains values from A.

#### Scenario: Invalid preview preserves render fallback without hiding head

- GIVEN successful snapshot A is visible
- WHEN invalid changed snapshot B is appended
- THEN B is head and its exact source plus raw validation detail are visible
- AND viewport may continue rendering A as latest-successful fallback
- AND the UI does not label A as head or delete B.
