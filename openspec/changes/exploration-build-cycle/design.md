# Design: Exploration Build Cycle

## Context

Ecky already has immutable render snapshots, authoring actor ordering, explicit
verification, drafts, and version history. The missing boundary is product
meaning. A render is execution. A candidate is a viable option. A version is a
deliberate saved milestone. Treating all three as one record produces noisy
history and gives autonomous exploration no bounded state.

The MVP adds a domain layer over existing authoring and render services. It does
not add another source of truth. Source and parameter inputs are resolved from
the existing canonical draft/snapshot path, then copied into immutable attempt
inputs.

## Goals / Non-Goals

Goals:

- make exploration replayable, bounded, and restart-safe;
- distinguish attempt, candidate, and version in storage and UI;
- attach each expensive build to a hypothesis and expected evidence;
- avoid stale render queues through latest-pending coalescing;
- preserve exact failed work without promoting it to version history;
- keep agent and manual exploration on one backend lifecycle.

Non-goals:

- solve CAD planning generally;
- let the model rewrite an arbitrary remaining plan;
- rank subjective design quality without declared checks;
- introduce parallel search, beam search, or genetic optimization;
- replace thread conversation, source persistence, or render snapshots.

## Domain Model

| Entity | Meaning | Created by | Mutable fields | Primary UI |
| --- | --- | --- | --- | --- |
| Working draft | Recoverable current authoring state | Manual or agent edit | Source, params, editor state | Editor / controls |
| Exploration cycle | Bounded search from one base | Explicit start | Status, budget counters, pending question, active candidate refs | Ecky bubble / exploration controls |
| Attempt | One immutable execution input and outcome | Build request | Outcome attachment only | Expandable evidence ledger |
| Candidate | Promoted viable attempt | Explicit promote | Label and disposition metadata | Candidate comparison |
| Version | Deliberate immutable milestone | Explicit candidate commit | No authoring content mutation | Primary history |

### Exploration cycle

Minimum fields:

```text
cycleId
threadId
baseRef
objective
hardConstraints
softPreferences
status
budget
budgetUsed
pendingQuestion
activeCandidateIds
acceptedCandidateId
createdAt
updatedAt
```

`baseRef` is a tagged reference to either a committed version or an immutable
canonical draft snapshot. It never means "whatever is latest".

### Attempt

Minimum fields:

```text
attemptId
cycleId
parentAttemptId?
baseCandidateId?
hypothesis
expectedEvidence
renderSnapshotInput
inputDigest
status
supersededByRequestId?
checks
artifactDigest?
rawError?
createdAt
completedAt?
```

Attempt statuses are `queued`, `running`, `succeeded`, `failed`, `superseded`,
or `cancelled`. Input fields never change. Completion attaches evidence to the
same attempt. A superseded worker result remains inspectable but cannot replace
the active viewport projection.

### Candidate

A candidate is an immutable reference to one successful attempt plus promotion
metadata:

```text
candidateId
cycleId
attemptId
label
promotionRationale
evidenceSummary
disposition
createdAt
```

Disposition is `active`, `accepted`, or `rejected`. Changing disposition does
not change the referenced attempt or render snapshot. Re-promoting the same
attempt is idempotent.

### Version

A version references the exact candidate attempt snapshot and green explicit
verification record. Commit does not re-render, copy current editor state, or
resolve a moving thread head. Candidate identity and input/artifact digests are
stored as provenance.

## Cycle Policy

The controller selects one next action from current structured state. It may
offer a short preview of the next two or three expected actions for UX, but that
preview is not executable authority.

MVP action kinds:

```text
ASK
HYPOTHESIZE
PATCH
CHECK
BUILD
COMPARE
PROMOTE_CANDIDATE
REJECT
STOP
```

`COMMIT_VERSION` remains an explicit user action outside autonomous cycle
progression.

Each `BUILD` requires:

- one hypothesis;
- an exact source/parameter snapshot;
- expected evidence or declared checks;
- a budget reservation;
- a parent attempt or base candidate when one exists.

The model may propose an action. A deterministic reducer validates allowed
transitions and updates cycle state. Model output never mutates cycle state,
history, or render projection directly.

## State Transitions

```text
idle
  -> exploring
exploring
  -> awaiting_input
  -> checking
  -> building
  -> stopped
awaiting_input
  -> exploring
checking
  -> exploring
  -> building
  -> stopped
building
  -> evaluating
evaluating
  -> exploring
  -> awaiting_input
  -> stopped
  -> completed
completed
  -> stopped
```

The reducer rejects transitions without required identity, budget, or evidence.
`ASK` yields control and persists the exact question plus the decision it will
affect. A user answer appends an answer event and resumes from structured cycle
state; conversation text is context, not lifecycle authority.

## Cheap Checks Before Build

Before an exact backend render, the MVP runs available deterministic checks:

- source parse and Core type validation;
- parameter and declared constraint validation;
- backend capability validation;
- input digest and dependency-lock validation.

A failed cheap check completes the attempt as failed without invoking OCCT.
Geometry-dependent verification still runs after successful render. The attempt
records which checks ran, which were unavailable, and their raw outputs.

## Scheduling And Publication

One authoring actor owns one cycle mailbox. Render workers remain stateless.

- At most one attempt is `running` per cycle.
- An explicit controller exploration step is never silently replaced.
- Repeated unstarted interactive rebuilds for the same working target coalesce
  into one pending request containing the newest exact input snapshot.
- When a pending request is replaced, no attempt record is created for work that
  never started.
- When running work becomes obsolete, its attempt receives `superseded` after
  completion or cancellation. Its evidence remains inspectable.
- Only the newest non-superseded successful attempt may publish the active
  viewport snapshot.

This preserves evidence without forcing users to wait through a FIFO queue of
obsolete parameter states.

## Promotion And Commit

Attempt promotion requires successful render plus every mandatory check declared
for the cycle. Promotion stores rationale and an evidence summary. It creates no
version and does not overwrite the working draft.

Candidate commit requires:

- explicit user action;
- candidate disposition `active` or `accepted`;
- green explicit verification naming the same render snapshot and artifact
  digest;
- unchanged candidate attempt input and artifact identity.

Commit creates exactly one version. Repeating the same commit request is
idempotent. A changed draft after promotion requires a new attempt and candidate;
commit never mixes candidate geometry with newer editor parameters.

## Persistence And Recovery

Backend services own cycle, attempt, candidate, and version mutations. Frontend,
watchers, and agents call Tauri or MCP boundaries; none write SQLite directly.

The persistence order is:

```text
resolve exact canonical input
  -> persist attempt as queued
  -> reserve budget and mark running
  -> run checks/render
  -> attach outcome and evidence
  -> publish eligible snapshot
  -> optionally promote candidate
  -> optionally commit version
```

Restart recovery marks orphaned `running` attempts as interrupted unless the
worker can be proven alive. It restores pending questions and candidate refs.
It does not automatically resume paid/model work or expensive render work.

## UI Projection

No new global status bar is introduced.

- Ecky bubble copy exposes cycle state, current hypothesis, budget, pending
  question, and running/pending build status.
- Candidate comparison shows promoted candidates only.
- Primary history shows committed versions and preserved legacy records.
- Attempt details stay collapsed under the owning cycle/candidate by default.
- Failed attempts show raw backend/provider diagnostics.
- Last good viewport snapshot remains visible while failed or pending attempts
  are inspected.

Major layout containers keep `overflow: hidden`; controls follow Tactical
Midnight styling and square borders.

## Boundary Contracts

Frontend and MCP-facing JSON use camelCase. Rust domain fields use snake_case.
Every Rust boundary struct uses `#[serde(rename_all = "camelCase")]` unless a
protocol-fixed JSON-RPC shape requires otherwise.

Minimum commands/tools:

```text
exploration_cycle_start
exploration_cycle_get
exploration_cycle_next
exploration_cycle_answer
exploration_attempt_build
exploration_candidate_promote
exploration_candidate_reject
exploration_candidate_commit
exploration_cycle_stop
```

Commands are idempotent by request ID. Read responses return tagged refs and
digests, never ambiguous generic `head` identifiers.

## Version-History Reconciliation

`lossless-version-history` correctly requires append-before-validation and exact
failure retention, but its generic "version" record carries two meanings. The
implementation separates them:

- draft journal event: changed recoverable authoring content;
- attempt: an execution snapshot and outcome;
- version: explicit candidate commit.

For migration, existing records remain readable and are tagged `legacy` when
explicit promotion provenance cannot be derived. No existing rows or source
bytes are deleted. New version counts exclude attempts and draft journal events.
Legacy counts remain available as a compatibility projection.

## Risks / Trade-offs

- More lifecycle entities increase storage and query complexity. Typed refs and
  one backend service boundary keep ownership explicit.
- Candidate promotion can still preserve weak designs. Mandatory checks prove
  declared requirements, not aesthetics.
- Coalescing drops unstarted intermediate slider states. Their working drafts
  remain recoverable, but they are intentionally not execution attempts.
- Legacy history cannot always reveal original user intent. Preserve it without
  guessing instead of destructive reclassification.
- A single running attempt limits throughput. MVP favors predictable state and
  cost; parallel search can be added after evidence proves need.
