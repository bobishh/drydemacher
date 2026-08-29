# Tasks: Exploration Build Cycle

## 1. Reconcile lifecycle terminology

- [ ] Update the active `lossless-version-history` change so durable draft
  events and failed execution attempts remain lossless without calling every
  record a version.
- [ ] Define tagged Rust identities for working draft snapshots, attempts,
  candidates, committed versions, and legacy records.
- [ ] Add architecture fitness coverage rejecting ambiguous generic lifecycle
  refs at new boundaries.

## 2. Outer BDD: exploration does not churn versions

- [ ] Add a failing Playwright scenario: start from one committed version, run
  failed and successful attempts, promote one candidate, and observe unchanged
  primary version count.
- [ ] Extend the scenario: commit the promoted candidate and observe exactly one
  new version with matching candidate and artifact identity.
- [ ] Add failure proof: raw attempt error remains visible while the last good
  viewport snapshot stays active.

## 3. Domain reducer

- [ ] Add failing Rust unit tests for allowed cycle transitions and required
  action payloads.
- [ ] Implement `ExplorationCycle`, typed actions, events, budget, and a pure
  reducer.
- [ ] Reject BUILD without hypothesis, expected evidence, exact input digest, or
  available budget.
- [ ] Reject autonomous COMMIT_VERSION transitions.
- [ ] Add ASK suspension and answer-resume tests.

## 4. Durable attempts and candidates

- [ ] Add backend-owned persistence for cycles, attempts, candidate refs, and
  outcome evidence without direct SQLite access from callers.
- [ ] Persist queued attempt input before checks or render.
- [ ] Attach success, failure, superseded status, artifact digest, verification,
  and raw diagnostics to the same attempt.
- [ ] Make candidate promotion and candidate commit idempotent by request ID.
- [ ] Preserve legacy history records without destructive reclassification.

## 5. Cheap-check and render orchestration

- [ ] Add failing service tests proving parse/type/parameter/capability failures
  complete attempts without invoking OCCT.
- [ ] Route eligible attempts through the existing immutable render snapshot
  service.
- [ ] Bind result publication to cycle, attempt, input, and artifact digests.
- [ ] Keep late or superseded results out of the active viewport projection.

## 6. Latest-pending scheduler

- [ ] Add failing actor tests for one running attempt and latest-pending
  interactive request coalescing.
- [ ] Preserve explicit controller exploration actions without silent
  coalescing.
- [ ] Record superseded running attempt evidence while publishing only the
  newest eligible result.
- [ ] Surface running and pending counts through existing Ecky state copy.

## 7. Tauri and MCP boundaries

- [ ] Add cycle start/get/next/answer/stop commands.
- [ ] Add attempt build, candidate promote/reject, and candidate commit commands.
- [ ] Keep TS payloads camelCase, Rust fields snake_case, and boundary serde
  translation explicit.
- [ ] Return raw errors and tagged lifecycle refs with source, input, snapshot,
  and artifact digests.
- [ ] Expose compact cycle packets by default; attempt evidence detail remains an
  explicit bounded read.

## 8. Workbench projection

- [ ] Add failing Playwright happy path for cycle state, one promoted candidate,
  and explicit version commit.
- [ ] Show current hypothesis, budget, running/pending build, and ASK state in
  Ecky bubble copy without a separate status bar.
- [ ] Add candidate comparison separate from primary version history.
- [ ] Keep attempt ledger collapsed by default with raw failure expansion.
- [ ] Verify Tactical Midnight styling, square borders, bounded overflow, desktop,
  and mobile layout.

## 9. Restart and migration proof

- [ ] Add restart integration coverage for cycle state, pending question,
  attempts, candidate refs, and budget.
- [ ] Mark orphaned running attempts interrupted without auto-resuming expensive
  work.
- [ ] Prove existing history remains readable and new version counts exclude new
  attempts and draft journal events.

## 10. Final proof

- [ ] Run targeted Rust reducer, persistence, actor, and command tests.
- [ ] Run the exploration Playwright happy path and failure/pending scenario.
- [ ] Run `npm run test:unit`.
- [ ] Run `cd src-tauri && cargo check`.
