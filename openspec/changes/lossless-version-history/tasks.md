# Tasks: Lossless Version History

BDD dual-loop. Start each behavior with a failing integration test, then drive
the smallest backend/frontend unit changes. No direct SQLite writes outside
the backend history service. Do not stage or commit.

## 1. Contract and storage

- [ ] 1.1 Add red integration coverage: invalid changed file appends and becomes
  head; successful filter excludes it.
- [ ] 1.2 Add append identity/order and exact-content fields with a migration
  that preserves legacy rows and deterministic order.
- [ ] 1.3 Implement one backend append service with digest no-op detection and
  serialized writes; route manual, watcher, draft, and MCP writers through it.
- [ ] 1.4 Record validation/render/artifact outcomes and raw errors as metadata
  on the appended version without replacing its content.
- [ ] 1.5 Compute canonical version-input digest from exact source, full
  effective parameters, runtime/backend/post-processing inputs, dependency
  locks, external input bytes, and cache schema identities.

## 2. Head and query semantics

- [ ] 2.1 Add unit tests proving latest head returns newest error/pending/draft
  version, not newest successful artifact.
- [ ] 2.2 Separate successful-version filter/count from head/history queries.
- [ ] 2.3 Update context, target metadata, project binding, and timeline callers
  to use head for current state and successful filter for printable history.
- [x] 2.4 Preserve delete/restore and pagination order without losing records.

## 3. Source and draft synchronization

- [ ] 3.1 Add red Playwright watcher flow: changed invalid source appears once,
  remains on disk, and becomes head with raw error.
- [ ] 3.2 Remove conflict/thread-advanced/force refusal from version append;
  serialize stale and concurrent writes and rebase mirror metadata.
- [ ] 3.3 Verify unchanged watcher/draft observations do not duplicate versions.

## 4. MCP and frontend

- [ ] 4.1 Adapt MCP inspect -> append -> validate -> preview -> verify so append
  occurs before validation, outcomes attach automatically, and no generic
  commit/finalize tool exists.
- [ ] 4.2 Keep Tauri camelCase/Rust snake_case boundary and backend-only DB
  mutation; update tool descriptions and raw error projection.
- [ ] 4.3 Update timeline/history UI to show all versions, with an explicit
  successful-only filter and head indicator.
- [ ] 4.4 Bind runtime cache lookup/publication to durable version ID plus
  version-input digest; keep artifact-byte deduplication a separate layer.
- [x] 4.5 Replace caller-authored missing-runtime persistence with one backend
  repair intent that reloads exact head inputs, checks optional artifact
  identity, validates render identities, and returns a bounded workspace
  projection. Cover success, source-less legacy, stale identity, and raw render
  failure in Rust.
- [x] 4.6 Add one backend sketch-preview intent that owns preview identity,
  latest-wins auto scheduling, renderer selection, validation, render, BRep
  candidates, hidden-line projection, and one bounded repair/rebuild. Cover
  happy, repair, raw failure, and camelCase boundary in Rust.
- [x] 4.7 Move pre-submit orthographic size/range auto-snap into the backend
  sketch-preview intent; return repaired document/evidence and forbid frontend
  policy in the main preview path.

## 5. Proof

- [ ] 5.1 Run targeted unit and Playwright happy/failure/concurrency tests.
- [ ] 5.2 Run `openspec validate lossless-version-history --strict`.
- [ ] 5.3 Run `cd src-tauri && cargo check` and relevant full suites before
  implementation completion.
