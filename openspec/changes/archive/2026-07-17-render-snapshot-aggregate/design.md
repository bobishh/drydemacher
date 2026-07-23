# Design: Render Snapshot Aggregate

## Runtime Actor Topology

```text
Supervisor
  +-- AuthoringActor(thread/draft)
  |     state: revision + DraftDesign + activeSnapshot + verification
  |     mailbox: Edit | RenderRequested | RenderCompleted | VerifyCompleted | Commit
  |
  +-- RenderWorker pool
  |     input: actorId + revision + immutable RenderInput
  |     output: RenderCompleted(actorId, revision, snapshot)
  |
  +-- VerifyWorker pool
  |     input: actorId + revision + snapshotId
  |     output: VerifyCompleted(actorId, revision, record)
  |
  +-- Presentation subscribers
        input: SnapshotPublished(actorId, revision, snapshot)
        output: code/params/viewport/action projections
```

Workers run concurrently. Only `AuthoringActor` decides whether a completion is
current enough to publish. No worker mutates draft/history/UI state directly.

Actor envelope:

```text
ActorEnvelope<T> {
  actorId,
  revision,
  messageId,
  correlationId,
  causationId,
  payload: T
}
```

`revision` orders intent inside one actor. `correlationId` follows one user/MCP
operation across render and verify. `causationId` links derived messages. IDs do
not establish ordering by themselves.

## Boundary Map

`AuthoringActor` is aggregate root for draft lifecycle. `RenderSnapshot` is an
immutable aggregate value emitted by rendering context. Saved history is a
separate aggregate referencing an accepted snapshot.

## Aggregate Invariants

1. `artifactBundle.modelId == modelManifest.modelId`.
2. Source language and geometry backend agree across design, bundle, and
   manifest.
3. `sourceDigest` derives from exact canonical source in `design`.
4. `parameterDigest` derives from canonical effective parameters, not UI draft
   fields or source defaults.
5. `snapshotId` includes source digest, parameter digest, post-processing,
   backend, artifact content hash, and manifest identity.
6. Verification records include snapshot ID and artifact digest.
7. Commit accepts only a green verification record for the same snapshot.

## Reference Types

Replace polymorphic optional string pairs with tagged references at external
boundaries:

```text
AuthoringTargetRef =
  | { kind: "savedVersion", threadId, messageId }
  | { kind: "draft", threadId, previewId, sessionId }
  | { kind: "latestSaved", threadId }
```

Compatibility adapters may accept old `threadId`/`messageId` arguments during
migration. They resolve once at the boundary and return the tagged reference.
Domain services never reinterpret a saved message ID as a draft base ID.

## Authority And Cache Policy

| Representation | Role | May resolve domain truth? |
|---|---|---|
| Saved message/version | durable authority for saved version | yes |
| Agent draft | durable authority for active draft | yes |
| RenderSnapshot | immutable authority for one render | yes |
| Process `HashMap` | cache by snapshot/draft ID | no |
| `AppState.last_snapshot` | presentation cache | no |
| `last_design.json` | restart pointer/cache | no |
| Frontend store | presentation projection | no |
| working copy / param panel compatibility stores | derived migration views | no |

Cache miss loads authority. Cache disagreement invalidates cache and surfaces a
typed mismatch; it never triggers a merge or source-default fallback.

## Actor State Machine

```text
Idle(rev N)
  -- Edit/RenderRequested --> Rendering(rev N+1)
Rendering(rev N)
  -- RenderCompleted(rev N) --> PreviewReady(rev N)
  -- RenderCompleted(rev < N) --> discard Superseded
PreviewReady(rev N)
  -- VerifyRequested --> Verifying(rev N)
  -- Edit/RenderRequested --> Rendering(rev N+1)
Verifying(rev N)
  -- VerifyCompleted(rev N, green) --> Verified(rev N)
  -- VerifyCompleted(rev < current) --> discard Superseded
Verified(rev N)
  -- Commit(rev N, snapshotId) --> Saved
  -- Edit --> Rendering(rev N+1), verification cleared
```

Failure is actor state/evidence, not process death. Supervisor restarts actor
from durable draft/saved state. In-flight worker messages from prior incarnation
carry stale actor generation and are rejected.

## Backend Ownership

- `services/render_snapshot.rs` owns construction, invariant validation, and
  digest calculation.
- Rendering commands and MCP preview handlers return `RenderSnapshot`.
- Agent draft repository persists the current snapshot payload or immutable
  snapshot reference.
- Actor repository persists revision and generation needed for recovery. A
  process-local actor registry is routing/cache only.
- Verification resolves a tagged target, loads its exact snapshot, and stores a
  `VerificationRecord` bound to its digests.
- Commit loads the same snapshot and its verification record. Title/version
  metadata may change; render inputs and outputs may not.
- One session snapshot service owns restart-cache serialization. Command modules
  call it; they do not duplicate file persistence.

## Frontend Ownership

One `activeRenderSnapshot` store receives a complete validated snapshot. Source,
parameter, viewport, and action-state selectors derive from it. Draft edits live
in a separate `AuthoringDraft` and carry `baseSnapshotId`.

Apply compares `baseSnapshotId` against active snapshot ID. Stale apply returns
the raw mismatch. It does not merge current parameter-panel values into an agent
preview.

Migration order:

1. Add snapshot contract and validation.
2. Route MCP preview event through one snapshot hydrator.
3. Derive compatibility stores from active snapshot.
4. Move manual render/history restore paths to same hydrator.
5. Remove direct compatibility-store writes.
6. Persist restart pointer instead of reconstructed aggregate payload.

## Verification Resolution

During migration, verifier parameter resolution follows strict precedence based
on identity, not availability:

1. If target is draft preview, load matching `AgentDraft` and require its
   artifact model ID to match requested model ID.
2. If target is saved version, load that message output and require its runtime
   model ID to match requested model ID.
3. Only unbound standalone artifacts may use declared source defaults, marked as
   such in diagnostic provenance.

No `try_lock`-dependent behavior. DB lock timing must not change diagnostics.

## Rejected Paths

- Memoize every representation. More copies amplify invalidation and identity
  ambiguity.
- Global frontend store containing drafts, history, viewport, and agent state.
  That creates another god aggregate.
- Eventual reconciliation by shallow object merge. It cannot prove which params
  produced an artifact.
- Put effective parameters only in STL metadata. STL is output, not authoring
  authority.
- Infer draft versus saved version from UUID shape or lookup order.
- Serialize all rendering globally. It prevents useful parallel work and hides
  missing ownership instead of fixing it.
- Let each worker publish directly. Completion order is not intent order.

## Proof Strategy

```gherkin
Given a saved model has base-height 10.5
And its active preview rendered base-height 25
When the preview is verified
Then diagnostic resolved parameters contain base-height 25
And clearing process caches does not change that result
```

```gherkin
Given revision 7 render is slow
And revision 8 render finishes first
When revision 7 completes later
Then the authoring actor keeps revision 8 visible
And revision 7 returns explicit superseded evidence
```

```gherkin
Given source and parameters belong to snapshot A
And viewport artifact belongs to snapshot B
When the frontend receives the payload
Then hydration rejects it before changing any visible projection
And the raw mismatch names both snapshot identities
```

```gherkin
Given snapshot A has green verification
When snapshot B replaces it before commit
Then commit fails stale-snapshot validation
And neither snapshot is silently merged
```

Rust completion requires `cd src-tauri && cargo check`. UI slices require real
route Playwright happy path plus mismatch/failure proof.
