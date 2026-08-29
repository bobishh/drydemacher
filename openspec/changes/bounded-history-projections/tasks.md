# Tasks: Bounded History Projections

## 1. Baseline and outer-loop failures

- [ ] Add content-free IPC/SQL probes: command/event name, bytes, duration,
  projection, rows, truncation, JSON-column selection, and DB mutex hold time.
- [ ] Add an isolated large-history fixture: 150 versions, equal timestamps,
  large text, and multi-million dense topology. Do not copy it into production
  app data.
- [ ] Add failing Playwright flow: open large thread, receive repeated history
  updates, preserve expansion/scroll/search state, page older rows, and select a
  version without calling full `get_thread` or exceeding budgets.
- [ ] Add failing Playwright pending/failure flow: oversized row exposes honest
  detail/truncation metadata and raw observed/allowed error.
- [ ] Record baseline and post-change WebContent/native RSS for the isolated
  fixture and repeated update cycle. Keep render memory separate from history
  projection memory.

## 2. P0 scalar and bounded database queries

- [x] Add an explicit offline transactional migration from legacy JSON CAD
  payloads to versioned binary core records and indexed binary topology chunks;
  normal startup must only enforce migration readiness.
- [x] Verify migrated core/chunk counts before commit, clear legacy payloads,
  and prohibit runtime legacy fallback.
- [ ] Replace `project_thread_head` full-thread scan with indexed scalar head
  query. Assert no payload JSON deserialization and verify query plan.
- [ ] Replace existence, count, unread, latest-status, finalize lookup, and
  version point lookups that materialize full threads.
- [ ] Replace provider/context full-thread scan with bounded recent-dialogue and
  exact-current-snapshot queries.
- [ ] Add stable `(timestamp,rowid)` opaque cursor and equal-timestamp no-gap,
  no-duplicate tests.
- [ ] Add thread-summary, timeline-row/page, version-detail, source-window, and
  dense-topology-page Rust contracts with `#[serde(rename_all = "camelCase")]`.

## 3. Frontend history projection

- [ ] Replace `history-updated -> getThread` with revisioned targeted
  invalidation and per-thread singleflight.
- [ ] Replace follow-up guard full-thread fetch with lightweight summary/current
  version lookup.
- [ ] Replace capture, screenshot, working-version, notification, fork, and
  inventory navigation full-thread reads with point/page queries.
- [ ] Keep timeline expansion, scroll anchor, search, filter, and older-page
  state stable across incoming events.
- [ ] Hydrate only the selected version detail; release superseded heavy detail
  and viewer resources when selection changes.
- [ ] Add frontend unit tests for stale revision rejection, coalescing, stable
  cursor merge, detail eviction, and UI state preservation.

## 4. Provider and MCP projections

- [ ] Make API, Codex, and Agy context builders consume the same bounded context
  query service without full message/artifact materialization.
- [ ] Make MCP `thread_get`, `thread_messages_get`, and `thread_meta_get` use SQL
  projections before filtering/limiting.
- [ ] Replace remaining render-preview, mark-read, version, and project-folder
  point lookups that materialize whole threads.
- [ ] Add tests proving large unrelated historic manifests do not affect context
  query bytes, DB mutex duration, or MCP result size.

## 5. Preview and dense topology

- [ ] Change draft preview event to identity, revision, and compact status only.
- [ ] Hydrate a preview snapshot by reference only for its active thread.
- [ ] Persist last-session snapshot reference without echoing full bundle and
  manifest back through IPC.
- [ ] Move anonymous dense edge/face/triangle targets to lazy sidecar pages;
  retain authored/tagged/analytic targets and explicit counts in core detail.
- [ ] Add happy, background-thread, stale-preview, oversized-topology, and raw
  mismatch-error acceptance tests.

## 6. Activity bounds

- [ ] Give backend activity journal sequence cursors and count/byte retention.
- [ ] Page startup/reconnect catch-up instead of returning every event.
- [ ] Bound frontend activity, long-task, and notification source-event
  collections while retaining pinned/error items.
- [ ] Prove interrupted-turn transcript and finished turns remain durable and
  visible after transient activity compaction.

## 7. Transport enforcement

- [ ] Centralize serialized-byte budgets for summary, timeline, detail,
  topology, events, and ordinary JSON IPC.
- [ ] Reject over-budget emission before transport with raw observed/allowed
  sizes and the supported sectioned read.
- [ ] Add debug assertions preventing production full-thread aggregate command
  calls and payload-bearing invalidation events.
- [ ] Add payload regression tests for every Tauri command/event changed here.

## 8. WebContent recovery

- [ ] Expose Wry WebContent termination callback through Ecky's Tauri runtime
  integration; record reason without user content.
- [ ] Reload once and restore durable thread/page/version/snapshot references.
- [ ] Reconcile unknown in-flight work without replaying provider messages,
  queue delivery, or render jobs.
- [ ] Add crash-loop guard and raw native recovery error.
- [ ] Add a native integration test for terminated WebContent plus Playwright
  restore-state coverage where browser simulation is sufficient.

## 9. Proof gates

- [ ] Large-thread initial UI loads one summary page, one timeline page, and at
  most one selected detail; no production `get_thread` call occurs.
- [ ] Fifty repeated history updates remain within timeline/event budgets and
  do not grow retained history memory monotonically.
- [ ] Watcher status performs scalar indexed reads and does not deserialize
  `ArtifactBundle` or `ModelManifest`.
- [ ] Provider and MCP context remain bounded with 150 dense historic versions.
- [ ] Active preview sends one compact event and one active-only point hydrate;
  background threads receive no full runtime payload.
- [ ] Equal-timestamp paging has no gaps or duplicates.
- [ ] WebContent recovery restores durable UI state without duplicate work.
- [ ] Relevant frontend unit tests pass.
- [ ] Relevant Playwright happy and pending/failure flows pass.
- [ ] `cd src-tauri && cargo check` and relevant Rust tests pass.
- [ ] `openspec validate bounded-history-projections --strict` passes.
