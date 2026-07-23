# Tasks: Render Snapshot Aggregate

## 1. Exploration And Contracts

- [x] 1.1 Trace source/params -> render -> preview event -> frontend stores ->
  Apply/commit/verification.
- [x] 1.2 Inventory durable authorities, caches, restart state, and projections.
- [x] 1.3 Record bounded contexts, aggregate invariants, and tagged target refs.
- [x] 1.4 Record actor topology, envelope, supervision, and revision state
  machine.
- [x] 1.5 Add `RenderSnapshot` and `VerificationRecord` Rust contracts with
  camelCase boundary serialization.
- [x] 1.6 Add canonical digest and invariant tests.

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
- [x] 2.6 Align process-cache and durable draft identity with the actor key
  `(session_id, thread_id)`; reject ambiguous no-thread resolution.
- [x] 2.7 Clear only the committed actor draft, durable storage first and cache
  projection second.
- [x] 2.8 Make durable draft upsert the preview publication commit point before
  RAM, restart projection, or frontend event visibility.
- [x] 2.9 Invalidate every in-flight session actor for a thread after successful
  UI source, parameter, post-processing, runtime, import, or manual-version
  mutation.

## 3. Exact Preview Verification Slice

- [x] 3.1 Add failing integration test: preview params differ from source/saved
  defaults and diagnostics must report preview params.
- [x] 3.2 Add draft lookup by preview identity and artifact guard.
- [x] 3.3 Remove lock-timing-dependent parameter fallback from verifier.
- [x] 3.4 Prove saved-version diagnostics remain unchanged.

## 4. Typed Target Resolution

- [x] 4.1 Add tagged `AuthoringTargetRef` contract and compatibility adapter.
- [x] 4.2 Stop active drafts intercepting explicit saved-version refs.
- [x] 4.3 Return typed stale/not-found errors with requested and resolved refs.

## 5. Verification-Bound Commit

- [x] 5.1 Persist `VerificationRecord` against snapshot and artifact digests.
- [x] 5.2 Require explicit green authored verification before preview commit.
- [x] 5.3 Reject commit after source, params, backend, or artifact changes.
- [x] 5.4 Restore verification record after process-cache loss.

## 6. Frontend Snapshot Projection

- [x] 6.1 Add Playwright happy path for an active-thread MCP preview projection.
- [x] 6.2 Add Playwright failure path proving a background-thread preview cannot
  replace the active workspace.
- [x] 6.3 Gate same-thread generation projection and restart-snapshot writes by
  newest request identity.
- [x] 6.4 Scope manual parameter-render supersession by thread instead of one
  process-global sequence.
- [x] 6.5 Add failing Playwright mismatch path preserving last good snapshot.
- [x] 6.6 Add one active snapshot store and atomic hydrator.
- [x] 6.7 Derive source, params, viewport runtime, and action state from snapshot.
- [x] 6.8 Remove direct writes to compatibility stores incrementally.

## 7. Session Cache Consolidation

- [x] 7.1 Consolidate duplicate runtime snapshot builders/writers into service.
- [x] 7.2 Mark process map and `last_design.json` as caches with digest guards.
- [x] 7.3 Persist restart pointer/reference instead of reconstructing truth from
  frontend stores.
- [x] 7.4 Prove cache deletion preserves draft/saved target resolution.

## 8. Verification

- [x] 8.1 Run focused Rust integration/unit tests for actor sequencing and
  touched preview handlers.
- [x] 8.2 Run relevant frontend unit tests for projection and request guards.
- [x] 8.3 Run Playwright active-preview happy path plus background-preview
  preservation path.
- [x] 8.4 Run `npm run build` after frontend projection changes.
- [x] 8.5 Run `cd src-tauri && cargo check`.
- [x] 8.6 Run `openspec validate render-snapshot-aggregate --strict`.
