# history-projection Specification

## Purpose

Keep durable conversation and version history fully queryable without loading,
deserializing, transporting, or retaining unrelated heavy payloads.

## ADDED Requirements

### Requirement: CAD payload storage migrates once to binary projections

The system SHALL migrate legacy JSON artifact and manifest payloads into a
versioned binary core and indexed binary topology chunks in one verified
transaction. After migration commits, runtime readers and writers SHALL use
only the binary schema and SHALL NOT fall back to legacy JSON.

#### Scenario: Existing local history migrates before upgrade launch

- **GIVEN** legacy rows contain JSON artifact bundles and model manifests
- **WHEN** the explicit offline migration command runs against that database
- **THEN** it streams and converts every payload without materializing dense
  topology arrays
- **AND** verifies core decode and chunk totals before clearing legacy payloads
- **AND** commits one migration marker only after every row succeeds.

#### Scenario: Normal startup sees unmigrated history

- **GIVEN** legacy JSON payload rows remain and no migration marker exists
- **WHEN** the upgraded client initializes history storage
- **THEN** startup returns a migration-required error without rewriting payloads
- **AND** tells the operator to run the offline migration command.

#### Scenario: Migration fails

- **WHEN** any legacy payload cannot be parsed, encoded, written, or verified
- **THEN** the transaction rolls back
- **AND** history commands do not use mixed schemas or a legacy fallback
- **AND** the raw owner identity and migration error are surfaced.

### Requirement: Thread collections use bounded projections

The system SHALL expose thread summaries, timeline pages, selected-version
detail, source windows, and dense topology as distinct bounded projections. No
production collection query SHALL return a full thread aggregate containing all
message payloads.

#### Scenario: Opening a large thread loads bounded data

- **GIVEN** a thread has 150 versions with multi-million-target manifests
- **WHEN** the user opens the thread
- **THEN** the frontend receives one thread summary and one bounded timeline page
- **AND** it receives at most one selected-version detail
- **AND** unrelated artifact bundles, manifests, images, and topology are not
  selected, deserialized, or transported.

#### Scenario: Oversized row remains addressable

- **GIVEN** one message field exceeds the timeline-page budget
- **WHEN** its timeline row is requested
- **THEN** the row remains present with identity and bounded preview
- **AND** observed size, allowed size, truncated fields, and an exact detail read
  are reported explicitly.

### Requirement: Pagination preserves total order

Timeline pagination SHALL use an opaque cursor over a stable total order that
includes timestamp and database sequence identity.

#### Scenario: Equal timestamps cross a page boundary

- **GIVEN** multiple messages share the same timestamp
- **WHEN** the user loads consecutive timeline pages
- **THEN** every eligible message appears exactly once
- **AND** no message is skipped or duplicated at the boundary.

### Requirement: Metadata queries avoid payload materialization

Head, existence, count, unread, status, and point-identity queries SHALL execute
as indexed scalar/projection SQL without selecting or deserializing full message
payload JSON.

#### Scenario: Watcher resolves thread head

- **GIVEN** a thread contains hundreds of megabytes of historic manifests
- **WHEN** the project-folder watcher checks its head
- **THEN** it reads the head identity through an indexed scalar query
- **AND** it does not deserialize any `ArtifactBundle` or `ModelManifest`.

#### Scenario: Provider builds recent context

- **GIVEN** a large thread has a small recent dialogue window
- **WHEN** API, Codex, or Agy builds context
- **THEN** the database selects only bounded recent dialogue and required current
  snapshot/source references
- **AND** unrelated historic runtime payloads are not materialized.

### Requirement: History invalidation is targeted and coalesced

History change events SHALL contain thread/message identity, revision, and kind
without message/runtime payload. Frontend SHALL keep at most one refresh in
flight per thread and SHALL reject stale responses.

#### Scenario: Updates arrive during refresh

- **GIVEN** one history refresh is in flight
- **WHEN** several newer revisions arrive for the same thread
- **THEN** no overlapping full refresh starts
- **AND** exactly one follow-up refresh observes the newest revision
- **AND** stale data cannot replace newer state.

#### Scenario: Working event updates timeline

- **GIVEN** the user expanded working history and set search/filter state
- **WHEN** a new event or message arrives
- **THEN** the relevant row is patched or invalidated
- **AND** expansion, scroll anchor, search, filter, and loaded older pages remain
  unchanged.

### Requirement: Projection transport has explicit budgets

The system SHALL count serialized bytes before IPC and enforce versioned limits
for collection pages, detail sections, and events. It SHALL report truncation or
failure with raw observed and allowed sizes.

#### Scenario: Ordinary request stays bounded

- **WHEN** a thread list, timeline page, or topology page is serialized
- **THEN** it stays within its documented row and byte budgets
- **AND** telemetry records shape and size without payload content.

#### Scenario: Hard ceiling would be exceeded

- **WHEN** an ordinary JSON command or event would exceed the hard transport
  ceiling
- **THEN** emission stops before IPC
- **AND** the user receives the raw observed/allowed size and supported sectioned
  read
- **AND** no generic retry or credential error replaces it.

### Requirement: WebContent termination restores durable projection safely

The system SHALL detect WebContent process termination, reload at most once per
recovery attempt, and restore durable selected thread, timeline cursor window,
selected version reference, and last good render snapshot reference.

#### Scenario: WebContent is terminated after memory pressure

- **GIVEN** native backend and durable history remain alive
- **WHEN** WebContent terminates
- **THEN** the UI reloads and restores bounded durable projections
- **AND** it does not replay a provider message, queue delivery, or render job.

#### Scenario: Recovery terminates again

- **GIVEN** one automatic recovery already ran
- **WHEN** WebContent terminates again before the recovery guard resets
- **THEN** the app stops automatic reload
- **AND** surfaces the raw native failure and recovery action.
