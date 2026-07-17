# Tasks: Render Snapshot Aggregate

## 1. Exploration And Contracts

- [x] 1.1 Trace source/params -> render -> preview event -> frontend stores ->
  Apply/commit/verification.
- [x] 1.2 Inventory durable authorities, caches, restart state, and projections.
- [x] 1.3 Record bounded contexts, aggregate invariants, and tagged target refs.
- [x] 1.4 Record actor topology, envelope, supervision, and revision state
  machine.
- [ ] 1.5 Add `RenderSnapshot` and `VerificationRecord` Rust contracts with
  camelCase boundary serialization.
- [ ] 1.6 Add canonical digest and invariant tests.

## 2. Authoring Actor Sequencing Slice

- [x] 2.1 Add failing integration test: revision N+1 publishes before revision
  N completes, and N cannot replace N+1.
- [x] 2.2 Add per-draft actor coordinator with monotonic revision and generation
  reservation.
- [x] 2.3 Send parameter, macro, and semantic-transform render completions
  through actor revision guard.
- [x] 2.4 Return explicit superseded evidence without changing active preview or
  surfacing a false session failure.
- [x] 2.5 Prove separate draft actors use independent publish locks.

## 3. Exact Preview Verification Slice

- [ ] 3.1 Add failing integration test: preview params differ from source/saved
  defaults and diagnostics must report preview params.
- [ ] 3.2 Add draft lookup by preview identity and artifact guard.
- [ ] 3.3 Remove lock-timing-dependent parameter fallback from verifier.
- [ ] 3.4 Prove saved-version diagnostics remain unchanged.

## 4. Typed Target Resolution

- [ ] 3.1 Add tagged `AuthoringTargetRef` contract and compatibility adapter.
- [ ] 3.2 Stop active drafts intercepting explicit saved-version refs.
- [ ] 3.3 Return typed stale/not-found errors with requested and resolved refs.

## 5. Verification-Bound Commit

- [ ] 4.1 Persist `VerificationRecord` against snapshot and artifact digests.
- [ ] 4.2 Require explicit green authored verification before preview commit.
- [ ] 4.3 Reject commit after source, params, backend, or artifact changes.
- [ ] 4.4 Restore verification record after process-cache loss.

## 6. Frontend Snapshot Projection

- [ ] 5.1 Add failing Playwright happy path for one complete MCP snapshot.
- [ ] 5.2 Add failing Playwright mismatch path preserving last good snapshot.
- [ ] 5.3 Add one active snapshot store and atomic hydrator.
- [ ] 5.4 Derive source, params, viewport runtime, and action state from snapshot.
- [ ] 5.5 Remove direct writes to compatibility stores incrementally.

## 7. Session Cache Consolidation

- [ ] 6.1 Consolidate duplicate runtime snapshot builders/writers into service.
- [ ] 6.2 Mark process map and `last_design.json` as caches with digest guards.
- [ ] 6.3 Persist restart pointer/reference instead of reconstructing truth from
  frontend stores.
- [ ] 6.4 Prove cache deletion preserves draft/saved target resolution.

## 8. Verification

- [x] 8.1 Run focused Rust integration/unit tests for actor sequencing and
  touched preview handlers.
- [ ] 8.2 Run relevant frontend unit tests when frontend projection slice starts.
- [ ] 8.3 Run Playwright happy plus failure path for each UI slice.
- [ ] 8.4 Run `npm run build` after frontend contract changes.
- [x] 8.5 Run `cd src-tauri && cargo check`.
- [x] 8.6 Run `openspec validate render-snapshot-aggregate --strict`.
