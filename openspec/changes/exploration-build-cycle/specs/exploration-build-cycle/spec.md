## ADDED Requirements

### Requirement: Exploration cycle has an explicit base and bounded objective

The system SHALL start an exploration cycle from one tagged committed-version or
canonical-draft snapshot reference. The cycle SHALL persist its objective, hard
constraints, soft preferences, budget, and status. It SHALL NOT resolve its base
through an ambiguous generic latest/head lookup.

#### Scenario: Cycle starts from committed version

- **GIVEN** committed version A is visible
- **WHEN** the user starts exploration with an objective and build budget
- **THEN** the cycle stores an immutable reference to version A
- **AND** later thread or draft changes do not change that base reference.

#### Scenario: Cycle starts from canonical working draft

- **GIVEN** a recoverable working draft has an immutable snapshot identity
- **WHEN** the user starts exploration before committing it
- **THEN** the cycle stores that exact draft snapshot reference
- **AND** it does not create a version merely to establish a base.

### Requirement: Next action is selected from current structured state

The system SHALL select and validate one typed next action from current cycle
state. It MAY display a short expected action preview, but SHALL NOT treat an
up-front generated unbounded plan or model-rewritten continuation as lifecycle
authority.

#### Scenario: New evidence changes next action

- **GIVEN** the expected next action after BUILD was COMPARE
- **WHEN** a cheap check fails before render
- **THEN** the reducer records the failed evidence
- **AND** selects a valid next action from the updated state
- **AND** no stale pre-generated BUILD tail executes automatically.

#### Scenario: Invalid model transition is rejected

- **GIVEN** model output proposes BUILD without a hypothesis or available budget
- **WHEN** the reducer validates the action
- **THEN** the transition is rejected before persistence or render
- **AND** the raw rejection identifies each missing requirement.

### Requirement: Every build execution is an immutable attempt

The system SHALL persist an attempt with its hypothesis, expected evidence,
exact source and effective parameter snapshot, input digest, parent identity,
and queued status before checks or render begin. Outcome data SHALL attach to
that attempt without changing its inputs.

#### Scenario: Backend failure retains attempt

- **GIVEN** an attempt is persisted and starts rendering
- **WHEN** the backend fails
- **THEN** the same attempt becomes failed
- **AND** its exact input and raw backend error remain inspectable
- **AND** no candidate or version is created.

#### Scenario: Cheap check prevents expensive build

- **GIVEN** an attempt contains unsupported Core IR for the selected backend
- **WHEN** capability validation runs
- **THEN** the attempt fails with the raw capability diagnostic
- **AND** OCCT is not invoked
- **AND** budget usage records the check without charging a completed exact
  build.

### Requirement: Builds require hypotheses and expected evidence

The system SHALL reject a BUILD action that does not name the intended change,
its hypothesis, and the evidence or checks expected to evaluate it.

#### Scenario: Repeated geometry change remains attributable

- **GIVEN** a prior attempt failed minimum wall verification
- **WHEN** a repair BUILD is requested
- **THEN** the new attempt identifies the wall repair hypothesis
- **AND** names minimum wall evidence as an expected result
- **AND** references the prior attempt or promoted base candidate.

### Requirement: Interactive pending builds use latest-wins coalescing

The system SHALL run at most one build attempt per cycle at a time. Unstarted
interactive rebuild requests for the same working target SHALL coalesce to the
newest exact input. Explicit controller exploration actions SHALL NOT be
silently coalesced.

#### Scenario: Rapid parameter applies avoid stale FIFO work

- **GIVEN** attempt A is running
- **WHEN** interactive requests B, C, and D arrive for the same working target
- **THEN** only D remains pending
- **AND** no attempts are created for unstarted B or C
- **AND** D starts after A completes or cancels.

#### Scenario: Superseded completion keeps evidence but not projection

- **GIVEN** running attempt A becomes obsolete after pending request D
- **WHEN** A completes successfully
- **THEN** A retains its result with superseded disposition
- **AND** A does not replace the active viewport snapshot
- **AND** D remains eligible to publish after success.

### Requirement: Candidate promotion is explicit and version-free

The system SHALL create a candidate only by explicitly promoting a successful
attempt that passed every mandatory cycle check. Promotion SHALL store rationale
and evidence identity, be idempotent, and SHALL NOT create a version.

#### Scenario: Successful attempt becomes candidate

- **GIVEN** attempt A rendered successfully and passed mandatory checks
- **WHEN** A is promoted with rationale
- **THEN** one candidate references A and its evidence
- **AND** the primary version count remains unchanged.

#### Scenario: Failed attempt cannot be promoted

- **GIVEN** attempt A failed or lacks mandatory verification
- **WHEN** promotion is requested
- **THEN** no candidate is created
- **AND** the raw rejection names failed or missing checks.

### Requirement: Version commit uses one verified candidate snapshot

The system SHALL create a version only through an explicit user commit of one
candidate with green verification naming the same render snapshot and artifact
digest. Commit SHALL NOT re-render or merge newer working-draft state.

#### Scenario: Candidate commit creates one version

- **GIVEN** candidate C references verified attempt A
- **WHEN** the user commits C
- **THEN** exactly one immutable version references C, A, and their exact digests
- **AND** repeated delivery of the same commit request creates no duplicate.

#### Scenario: Newer draft cannot leak into candidate commit

- **GIVEN** candidate C was promoted from attempt A
- **AND** the working draft changed after A
- **WHEN** the user commits C
- **THEN** the version contains A's source, parameters, artifact, and verification
- **AND** no value from the newer working draft is merged into the version.

### Requirement: ASK suspends execution with decision context

The system SHALL persist an ASK action with its exact question, the decision it
affects, and current cycle identity before yielding to the user. Conversation
history SHALL NOT be the authoritative record of the pending effect.

#### Scenario: Answer resumes same decision

- **GIVEN** a cycle waits for mounting-hole spacing
- **WHEN** the user answers after restart
- **THEN** the answer attaches to the persisted pending question
- **AND** the reducer resumes from the same cycle state
- **AND** it does not infer which old chat question was pending.

### Requirement: Budget and stop state bound exploration

The system SHALL reserve budget before model or exact-build work and SHALL stop
automatic progression when the relevant budget is exhausted. Stopping SHALL
preserve attempts and candidates without committing a version.

#### Scenario: Build budget prevents another render

- **GIVEN** a cycle used its exact-build budget
- **WHEN** another BUILD is proposed
- **THEN** the reducer rejects automatic execution
- **AND** exposes STOP, ASK, or explicit user budget extension as valid next
  actions.

### Requirement: Restart restores exploration without auto-resume

The system SHALL restore cycle state, attempt ledger, candidates, budget, and
pending question after restart. It SHALL NOT automatically resume model calls or
expensive render work whose worker liveness cannot be proven.

#### Scenario: Restart interrupts running attempt safely

- **GIVEN** an attempt was marked running when the process stopped
- **WHEN** the project reopens and no worker can be proven alive
- **THEN** the attempt becomes interrupted/cancelled with diagnostic evidence
- **AND** candidates and versions remain unchanged
- **AND** the user can explicitly retry from the same input.

### Requirement: UI separates versions, candidates, and attempts

The workbench SHALL show committed versions as primary history, promoted
candidates as the active comparison set, and attempts as collapsed exploration
evidence. Pending/running/ASK state SHALL appear through existing Ecky state
surfaces rather than a separate agent status bar.

#### Scenario: Many attempts do not flood version history

- **GIVEN** one cycle contains six attempts and one promoted candidate
- **WHEN** the user opens primary history
- **THEN** no new version appears until candidate commit
- **AND** candidate comparison shows one candidate
- **AND** the six attempts remain available under exploration details.

#### Scenario: Failed attempt preserves last good viewport

- **GIVEN** a successful candidate snapshot is visible
- **WHEN** a newer attempt fails
- **THEN** the successful snapshot remains visible and actionable
- **AND** the failed attempt's raw diagnostic is visible in exploration details.
