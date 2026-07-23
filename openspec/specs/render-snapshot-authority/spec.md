# render-snapshot-authority Specification

## Purpose
TBD - created by archiving change render-snapshot-aggregate. Update Purpose after archive.
## Requirements
### Requirement: Authoring actor owns draft transitions

The system SHALL route mutable lifecycle transitions for one active draft
through one authoring actor mailbox while render and verification workers remain
parallel and stateless.

#### Scenario: Newer revision wins out-of-order completion

- **GIVEN** render worker A computes draft revision 7
- **AND** render worker B computes newer draft revision 8
- **WHEN** revision 8 completes before revision 7
- **THEN** the authoring actor publishes revision 8
- **AND** late revision 7 is marked superseded without replacing active state.

#### Scenario: Different draft actors run concurrently

- **GIVEN** thread A and thread B have independent authoring actors
- **WHEN** both request rendering
- **THEN** their render workers may execute concurrently
- **AND** each completion is ordered only against its owning actor revision.

#### Scenario: Worker failure stays scoped

- **GIVEN** one render worker fails for actor A
- **WHEN** supervisor records the failure
- **THEN** actor A keeps its last good snapshot and raw failure evidence
- **AND** actor B continues processing messages.

### Requirement: Render snapshot is one immutable aggregate

The system SHALL represent exact render inputs and outputs as one immutable
snapshot containing source identity, effective parameter identity, backend,
artifact bundle, and model manifest.

#### Scenario: Preview publishes one coherent snapshot

- **GIVEN** source and effective parameters render successfully
- **WHEN** a preview is published to MCP or frontend consumers
- **THEN** source, effective parameters, backend, artifact, and manifest share
  one snapshot identity
- **AND** consumers do not reconstruct the snapshot by merging independent
  stores.

#### Scenario: Mismatched payload is rejected

- **GIVEN** design metadata names one render input
- **AND** artifact or manifest metadata names another model or backend
- **WHEN** snapshot validation runs
- **THEN** the snapshot is rejected before replacing the last good projection
- **AND** the raw error identifies the conflicting fields.

### Requirement: Draft and saved-version references are unambiguous

The system SHALL use distinct tagged references for draft previews and saved
versions after boundary compatibility parsing.

#### Scenario: Explicit saved version remains saved

- **GIVEN** a session has an active draft based on saved message A
- **WHEN** a caller explicitly targets saved message A as a saved-version ref
- **THEN** resolution returns saved message A
- **AND** the active draft does not intercept that reference.

#### Scenario: Draft ref resolves only matching draft

- **GIVEN** a caller targets a draft preview ID
- **WHEN** session, thread, preview, or artifact identity differs
- **THEN** resolution returns a typed mismatch or not-found error
- **AND** it does not fall back to latest saved state.

### Requirement: Verification uses exact snapshot parameters

The system SHALL resolve verifier diagnostics and authored checks from the
effective parameters bound to the verified snapshot.

#### Scenario: Preview parameters override saved defaults

- **GIVEN** a saved version uses `base-height=10.5`
- **AND** a draft preview rendered `base-height=25`
- **WHEN** that draft preview is verified
- **THEN** diagnostic resolved parameters report `base-height=25`
- **AND** source or saved defaults do not replace the draft value.

#### Scenario: Cache loss does not change verification

- **GIVEN** a durable draft snapshot exists
- **AND** process preview caches are cleared
- **WHEN** the same draft is verified
- **THEN** verification resolves the same source, parameters, and artifact
- **AND** lock timing does not alter the result.

### Requirement: Commit is bound to green verification of same snapshot

The system SHALL commit a draft only when a green explicit verification record
names the same snapshot and artifact digest.

#### Scenario: Verified snapshot commits

- **GIVEN** snapshot A has a green explicit verification record
- **WHEN** commit targets snapshot A
- **THEN** the saved version references snapshot A inputs and outputs unchanged.

#### Scenario: Changed snapshot invalidates verification

- **GIVEN** snapshot A has green verification
- **AND** source, parameters, backend, or artifact changes produce snapshot B
- **WHEN** commit targets snapshot B without new verification
- **THEN** commit fails with a stale-verification error
- **AND** no history version is created.

### Requirement: Frontend state is a projection of active snapshot

The workbench SHALL derive source, parameter controls, viewport runtime, and
snapshot-bound actions from one active render snapshot.

#### Scenario: Agent preview updates every projection together

- **GIVEN** snapshot A is visible
- **WHEN** a valid agent preview publishes snapshot B
- **THEN** code, parameters, viewport artifact, manifest, and active identity
  all project snapshot B
- **AND** no projection retains values from snapshot A.

#### Scenario: Invalid preview preserves last good snapshot

- **GIVEN** snapshot A is visible
- **WHEN** an invalid or partial snapshot B arrives
- **THEN** snapshot A remains visible and actionable
- **AND** raw validation detail appears through the normal Ecky error surface.
