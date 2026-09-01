# Delta for campaign-learning-flow

## ADDED Requirements

### Requirement: File-backed campaign teaching content

The system SHALL source every campaign step's substantive prose from a stable
section in canonical mission Markdown. The campaign manifest SHALL contain
ordering and interaction metadata in canonical data-only EDN, not duplicated
tutorial paragraphs. JSON SHALL NOT be a committed campaign definition format.

#### Scenario: Campaign definition is built

- GIVEN canonical mission Markdown and `manifest.edn`
- WHEN packaged campaign resources are loaded
- THEN the Rust campaign-definition service uses the strict data-only parser
- AND no EDN form is evaluated
- AND no committed JSON mirror is read or written

#### Scenario: Frontend requests campaign content

- GIVEN packaged campaigns contain complete prose, source, and preview corpora
- WHEN frontend lists campaigns or opens one step
- THEN typed Tauri commands return summaries or that one bounded step payload
- AND frontend bundles contain no campaign EDN parser, corpus glob, or complete
  source collection

#### Scenario: Desktop and HTML show one canonical section

- GIVEN a mission Markdown section identifies a campaign step
- WHEN desktop Campaign renders through the Rust service and static Chapters
  render the Markdown directly
- THEN both project the same Markdown section
- AND Svelte components contain no substantive lesson copy

#### Scenario: Content mapping is incomplete

- GIVEN a manifest step has no matching non-empty Markdown section
- WHEN campaign content is assembled
- THEN assembly fails with mission id and step id
- AND no empty tutorial screen is shipped

### Requirement: Ordered progressive instruction

The system SHALL present campaign steps in order and SHALL introduce every form
or modeling pattern before a challenge requires it.

#### Scenario: Challenge uses a new modeling pattern

- GIVEN a challenge starter or reference uses a form not introduced earlier
- WHEN campaign coverage is validated
- THEN validation fails with the missing form and prerequisite step

#### Scenario: Learner completes a bounded edit

- GIVEN worked prose and a working example introduced the required forms
- WHEN the learner edits the challenge and submits it
- THEN acceptance compares compiled Core IR with the reference
- AND source grep is not the acceptance decision

### Requirement: Canonical preview without runtime render

The system SHALL package a verified preview artifact for every source-bearing
campaign step and SHALL load it by canonical source digest without invoking a
geometry kernel.

#### Scenario: Learner opens or revisits a source step

- GIVEN the packaged preview digest matches canonical source and runtime
- WHEN the step opens, reloads, or is revisited
- THEN its STL appears immediately
- AND no render command or kernel process starts

#### Scenario: Learner edits source

- GIVEN editor source differs from the displayed artifact digest
- WHEN no render action was requested
- THEN the last good preview remains visible with a stale marker
- AND no render starts

#### Scenario: Learner presses render

- GIVEN edited source has a verified immutable cache artifact
- WHEN RENDER is pressed
- THEN that artifact is displayed without kernel execution
- OTHERWISE exactly one render starts and successful output enters cache

### Requirement: Drafts follow canonical source identity

The system SHALL bind a campaign draft override to campaign definition, step id,
and canonical source digest.

#### Scenario: Lesson source changes

- GIVEN an old draft exists for the same step id and an older source digest
- WHEN the updated campaign opens
- THEN current canonical source is loaded
- AND the old draft is not projected onto it

### Requirement: Campaign is a persistent Project surface

The system SHALL persist the active Project kind and id through typed Tauri/DB
commands and SHALL restore campaign and Projects-window navigation on reload.

#### Scenario: Reload during a campaign

- GIVEN a campaign run and step are active
- WHEN the application reloads
- THEN the same campaign run and current valid step reopen
- AND no design thread or folder is created

#### Scenario: Reload with Projects visible

- GIVEN the Projects window is visible with a saved rectangle
- WHEN the application reloads
- THEN Projects is visible at its restored bounded rectangle

#### Scenario: Saved campaign was deleted

- GIVEN persisted navigation references a missing campaign run
- WHEN boot restores navigation
- THEN the app opens Projects instead
- AND reports the missing run without a blank surface

### Requirement: Rust owns campaign progression atomically

The system SHALL load canonical run and packaged current-step state in Rust for
draft save, continue, back, and challenge-check actions. Frontend SHALL NOT
construct replacement progress collections or chain acceptance and persistence.

#### Scenario: Continue persists one canonical transition

- GIVEN a non-challenge current step has a next step
- WHEN learner continues
- THEN one Rust intent marks current step completed, selects next step, persists
  canonical run state, and returns next step projection
- AND no caller-authored replacement run is accepted by that flow.

#### Scenario: Challenge check and progression are atomic

- GIVEN current step is a challenge with Core IR acceptance
- WHEN learner checks candidate source
- THEN Rust evaluates packaged reference and candidate
- AND matched result saves draft, completion, pass, and next step atomically
- AND unmatched result saves only canonical draft state and check outcome.

#### Scenario: Illegal navigation preserves run

- GIVEN previous step is not completed or challenge has not passed
- WHEN caller requests back or continue
- THEN Rust returns exact validation error
- AND persisted run remains unchanged.

### Requirement: Rust owns campaign project opening

The system SHALL accept only campaign definition identity for start and run
identity for resume. Rust SHALL derive canonical run/step facts and persist the
matching active Project navigation atomically.

#### Scenario: Start campaign

- GIVEN a packaged campaign definition
- WHEN frontend submits `start` with its definition id
- THEN Rust selects title, first step, and definition version
- AND creates the run and active campaign navigation in one transaction
- AND returns canonical run plus current step.

#### Scenario: Resume campaign

- GIVEN a persisted campaign run
- WHEN frontend submits `resume` with its run id
- THEN Rust reloads the run and packaged current step
- AND validates definition identity before changing active navigation
- AND returns raw not-found or conflict errors without partial navigation.

#### Scenario: Delete active campaign

- GIVEN active navigation points to a campaign run
- WHEN that run is deleted
- THEN Rust deletes the run and matching navigation in one transaction
- AND frontend performs no follow-up navigation repair write.
