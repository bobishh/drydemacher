# Change: Exploration Build Cycle

## Why

Ecky currently exposes too many exploratory renders as peer history versions. A
long CAD search can produce dozens of records without showing which records were
failed experiments, viable alternatives, or deliberate user milestones. The
flat history also gives the agent no bounded, replayable exploration state: it
can repeat failed changes, invoke expensive geometry too early, and continue
without an explicit success condition.

The product needs a small exploration loop that separates execution evidence
from user decisions. Lossless persistence remains mandatory, but persistence
alone must not promote every changed draft or render into a version.

## What Changes

- Add an explicit exploration cycle rooted at one committed version or canonical
  working draft snapshot.
- Choose the next typed action from current cycle state. Do not generate an
  unbounded plan up front.
- Record every build execution as an immutable attempt with its hypothesis,
  exact input snapshot, status, evidence, and raw failure.
- Promote a successful attempt into a candidate only through an explicit
  promotion action. Candidate promotion creates no version.
- Create a version only through an explicit commit of one verified candidate.
- Keep versions in the primary history. Show candidates as the comparison set
  and attempts as expandable exploration evidence.
- Run at most one build attempt per cycle at a time. Coalesce unstarted
  interactive rebuild requests to the latest input.
- Persist cycle state and attempts before expensive execution so restart or
  failure cannot lose work or evidence.

## Relationship To Existing Changes

This change revises terminology in `lossless-version-history`.

The following invariants remain:

- exact changed authoring content is recoverable before validation or render;
- failed work and raw diagnostics are retained;
- persistence flows through backend services or MCP commands;
- no direct SQLite writes occur.

The following rule is superseded:

- every persisted draft or changed source observation is a version.

Draft journal events and exploration attempts become durable authoring records.
Only explicit candidate commits become new versions. The implementation MUST
reconcile both active changes before either is archived as baseline truth.

## Out Of Scope

- A general workflow language or arbitrary continuation rewriting.
- Up-front generation of a complete SPEC/ASK/BUILD/VERIFY program.
- Parallel candidate builds or distributed workers.
- Global geometry optimization or automatic aesthetic ranking.
- New CAD language operations, constraints, or feature semantics.
- Automatic candidate promotion or automatic version commit.
- Deleting, squashing, or reclassifying legacy history records destructively.

## Proof Gates

- Six build attempts containing failures and superseded work can produce one
  candidate and zero versions until the user commits that candidate.
- Committing the candidate creates exactly one immutable version bound to the
  candidate's verified render snapshot.
- A failed attempt retains exact input and raw diagnostics while the last good
  viewport snapshot remains visible.
- Three queued interactive rebuild requests produce the running attempt plus
  one attempt for the newest pending input, not three stale renders.
- Restart restores the active cycle, candidate set, attempt ledger, budget, and
  pending user question without promoting any record.
