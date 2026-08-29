# interactive-ecky-learning Specification

## Purpose
TBD - created by archiving change interactive-ecky-learning-missions. Update Purpose after archive.
## Requirements
### Requirement: Campaign levels require active solution work

The system SHALL preserve the six-level Ecky campaign as the instructional spine and
SHALL give every level an interactive sequence containing a briefing, annotated worked
example, decision checkpoint, editable completion or repair task, less-scaffolded
transfer prompt, progressive hints, structural feedback, worked solution, reasoning,
and alternative trade-off.

#### Scenario: Learner opens a campaign level

- **WHEN** a learner selects any level on `/learn/ecky-ir`
- **THEN** the matching file-backed mission loads above the long-form chapter
- **AND** its active phase, artifact objective, and progress state are visible
- **AND** no substantive lesson, starter source, criterion, hint, or solution is
  sourced from a Svelte component.

#### Scenario: Learner opens the dry reference

- **WHEN** a learner opens `/docs/ecky-ir`
- **THEN** the language reference renders without campaign attempt state
- **AND** the interactive mission manifest is not required for reference reading.

### Requirement: First mission produces a useful bracket

Campaign Level 01 SHALL teach a connected corner bracket made from intersecting
solids. The campaign SHALL NOT use a ball-on-base, sphere-on-platform, or decorative
marker as its first mission artifact.

#### Scenario: New learner starts Level 01

- **WHEN** Level 01 renders
- **THEN** its title and objective identify a corner bracket
- **AND** its worked example explains root, named part, placement, overlap, and union
- **AND** its practice asks the learner to repair disconnected solids
- **AND** its transfer prompt requests a named reinforcement or mounting feature.

### Requirement: Guidance fades from worked example to transfer

The campaign SHALL reduce supplied solution steps within each mission and across the
ordered levels. It SHALL move from an explained complete example to constrained
decision/completion work and then to a less-scaffolded transfer task.

#### Scenario: Novice studies before editing

- **WHEN** the learner opens STUDY
- **THEN** the system shows complete source with ordered annotations tied to modeling
  decisions
- **AND** the annotations explain purpose and invariant rather than only restating
  syntax.

#### Scenario: Learner reaches transfer

- **WHEN** structural practice criteria pass
- **THEN** the system exposes a related task with fewer supplied source steps
- **AND** the transfer prompt requires reuse of the learned relation on a changed
  artifact or constraint.

### Requirement: Decision checkpoints explain consequences

Every mission SHALL present at least one choice among plausible modeling approaches.
Each option SHALL have specific consequence feedback, and one or more option ids SHALL
be explicitly accepted by mission content.

#### Scenario: Learner selects a weak approach

- **WHEN** the learner selects a non-accepted option
- **THEN** the option remains selected and visible
- **AND** feedback names the resulting modeling, fit, repetition, boolean, or export
  problem
- **AND** the learner may choose again without resetting source work.

#### Scenario: Learner selects an accepted approach

- **WHEN** the learner selects an accepted option
- **THEN** feedback explains why it protects the mission's target relation
- **AND** the practice phase becomes available.

### Requirement: Browser feedback is deterministic and honest

The standalone browser SHALL evaluate only bounded structural criteria from the
mission manifest. It SHALL report every criterion result and SHALL distinguish these
checks from native Ecky compilation, geometry validation, printability, watertightness,
and export.

#### Scenario: Untouched attempt is pending

- **WHEN** a learner has not run `CHECK STRUCTURE`
- **THEN** the mission shows a pending state
- **AND** it does not claim success, compilation, valid geometry, printability,
  watertightness, or export.

#### Scenario: Attempt fails structural checks

- **WHEN** the learner checks source missing a required form, symbol, placement,
  repetition, or placeholder replacement
- **THEN** every failed criterion displays its exact learner-facing failure feedback
- **AND** passed criteria remain separately visible
- **AND** the attempt remains editable.

#### Scenario: Attempt passes structural checks

- **WHEN** every declared structural criterion passes
- **THEN** the system marks structural practice complete
- **AND** it states that native compile and preview remain separate proof
- **AND** it unlocks transfer and normal solution reveal.

#### Scenario: Mission content requests an unsupported check

- **WHEN** manifest validation encounters an unsupported criterion kind
- **THEN** the chapter remains readable
- **AND** the mission surface shows the raw validation failure
- **AND** it does not silently skip the criterion.

### Requirement: Native handoff preserves exact learner source

The system SHALL hand off the exact current attempt to the existing Ecky authoring
surface when that callback is available. A standalone route SHALL provide exact copy
and `.ecky` download instead.

#### Scenario: Learner opens attempt in the desktop workbench

- **WHEN** `OPEN ATTEMPT IN CODE` is activated with an in-app callback
- **THEN** the callback receives the exact current source and mission title
- **AND** no canonical solution replaces learner source during handoff.

#### Scenario: Learner uses standalone campaign

- **WHEN** no in-app callback exists
- **THEN** `COPY ATTEMPT` copies the exact current source
- **AND** `DOWNLOAD .ECKY` downloads the exact current source with the mission's
  file-backed suggested filename.

### Requirement: Worked solutions include reasoning and alternatives

Every mission SHALL include one complete worked solution, ordered reasoning, a
statement of what still requires native proof, and at least one named alternative with
a concrete trade-off.

#### Scenario: Learner reveals after an attempt

- **WHEN** the learner has checked an attempt and activates `SHOW WORKED SOLUTION`
- **THEN** full source, reasoning steps, native-proof limitation, and alternatives
  render
- **AND** the learner's attempt remains available for comparison.

#### Scenario: Learner reveals before an attempt

- **WHEN** the learner activates solution reveal before checking
- **THEN** the system asks for a second explicit `SHOW SOLUTION ANYWAY` activation
- **AND** the first activation does not expose source.

### Requirement: Mission progress is local, bounded, and recoverable

The system SHALL store campaign progress under a versioned local key. It SHALL persist
only known mission attempts and interaction state, ignore corrupt or incompatible
records, and reset only the active mission when requested.

#### Scenario: Learner returns to a mission

- **WHEN** the page reloads with valid progress for the loaded manifest version
- **THEN** attempt source, decision, hint depth, criterion state, and solution reveal
  state restore.

#### Scenario: Stored progress is malformed

- **WHEN** local progress is malformed, incompatible, or names unknown missions
- **THEN** mission content loads with clean defaults
- **AND** the readable chapter remains available
- **AND** no app configuration file or Tauri save command is involved.

### Requirement: Mission content has one canonical publication path

The system SHALL keep chapter prose and interactive mission data under
`docs/books/ecky-ir/`, generate public campaign artifacts under `public/tutorials/`,
and fail source checks when canonical and published artifacts drift.

#### Scenario: Campaign artifacts are checked

- **WHEN** the book-source check runs
- **THEN** all six mission ids and section slugs match campaign order
- **AND** the published mission bundle equals normalized canonical mission files
- **AND** duplicate ids, missing solutions, missing alternatives, and unsupported
  criteria fail with exact errors.

### Requirement: Mission workbench follows established visual boundaries

The mission workbench SHALL use Tactical Midnight tokens, square borders, visible
focus, accessible native controls, and bounded layout containers. Every major
container SHALL use `overflow: hidden`; long source, feedback, and solution content
SHALL scroll inside explicit child surfaces.

#### Scenario: Mission runs on a narrow viewport

- **WHEN** the campaign renders at 390 CSS pixels wide
- **THEN** one phase is primary at a time
- **AND** controls remain reachable without horizontal document overflow
- **AND** editor, feedback, and solution content do not bleed outside the workbench.
