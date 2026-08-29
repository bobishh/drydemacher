# Design: Bounded History Projections

## Measured failure

Current persistence is not the primary fault. Read shape is.

- `history-updated` refreshes summaries, then calls full `getThread` for the
  active thread. Bursts can overlap because the handler has no per-thread
  singleflight or revision check.
- Follow-up dispatch calls full `getThread` before every provider turn.
- Provider context loads every full message, then filters to recent dialogue and
  the latest snapshot.
- `project_thread_head` loads and deserializes every full message to find one
  head ID. A live sample during the reported slow Voronoi render showed this
  exact stack hot in every sample.
- MCP message/meta handlers load the full thread before filtering and limiting.
- Draft-preview events carry full design, artifact bundle, and manifest to all
  listeners. The active UI persists the same full aggregate back to Rust even
  though Rust already stored it.
- Activity journals and frontend activity collections grow without a bounded
  catch-up window.

Large JSON values amplify at least four representations: SQLite text, Rust
objects, serialized IPC strings, and parsed JS objects. Three/viewer structures
add more memory after hydration. A row count limit applied after full
deserialization does not bound work or memory.

## Projection model

No production command returns `Thread { messages: Vec<Message> }`.

```text
ThreadSummary
  id, title, timestamps, unread/status counters, headVersionRef

ThreadTimelinePage
  rows: TimelineRow[]
  nextCursor, hasMore, observedBytes, truncatedFields

TimelineRow
  message identity, role, status, timestamp, compact text preview
  version identity/status/origin, hasSource, hasRuntime, target counts

VersionDetail
  exact selected version metadata and bounded authoring/result content
  runtimeRef, topologySummary, available detail sections

RenderSnapshotProjection
  immutable snapshot identity, core runtime paths/digests, authored/analytic
  targets, denseTopologyRef and counts

DenseTopologyPage
  snapshot identity, kind, items, nextCursor, totalCount, observedBytes
```

Full fidelity remains in durable storage as a versioned binary core plus
indexed binary topology chunks. Consumers ask for the smallest exact projection
they need. Exact source uses a dedicated byte-window read with
digest, total bytes, returned range, and continuation metadata. Binary preview
assets use file/asset URLs rather than JSON/base64 IPC.

## Stable ordering and cursors

Timeline order is `(timestamp DESC, rowid DESC)`. Cursor encodes both values plus
a schema version and thread identity. It is opaque to frontend and MCP callers.
The next query uses lexicographic comparison:

```sql
timestamp < :timestamp
OR (timestamp = :timestamp AND rowid < :rowid)
```

This prevents duplicates or gaps when multiple messages share a timestamp.
Deleted/discarded visibility remains governed by lossless-version-history; a
cursor never changes semantic filtering.

## Database access rules

Projection starts in SQL. Heavy JSON columns are absent from summary, timeline,
head, count, existence, unread, and recent-dialogue selects.

- Head/status/existence: indexed scalar query with `ORDER BY timestamp DESC,
  rowid DESC LIMIT 1` or aggregate SQL.
- Recent dialogue: select only bounded text/status columns plus the one exact
  current snapshot reference needed by context assembly.
- Selected version: query one message ID and deserialize only requested detail
  sections.
- MCP messages: execute role/status/cursor/limit in SQL before deserialization.
- Watcher tick: read scalar head identity/revision; never acquire a full thread.

Indexes and query plans are tested. A query that only needs metadata must remain
coverable without reading payload JSON. Full-row helpers cannot be called from
production request/event paths.

## Change notification and concurrency

`history-updated` becomes a typed, payload-free notification:

```text
HistoryChanged { threadId, messageId?, revision, kind }
```

Frontend keeps at most one refresh in flight per thread. A newer revision marks
the current response stale; after it completes, exactly one refresh runs for the
latest revision. An event patches a known timeline row when possible. It never
resets expansion, scroll, search, or filter state.

Point-navigation events carry thread/version references. Missing rows are
hydrated with `VersionDetail`; they do not fetch a full thread.

## Preview delivery

The authoring actor persists the immutable draft snapshot once. It then emits a
small `DraftPreviewChanged` event containing thread/session/preview/snapshot
identity, status summary, and revision. Only a view whose active thread matches
hydrates `RenderSnapshotProjection` by reference. Background views do nothing
beyond updating summary state.

Frontend updates its durable last-session pointer by snapshot reference. It does
not echo artifact bundle or manifest back to Rust. Snapshot coherence checks
remain mandatory on point hydration.

Dense anonymous edge/face/triangle targets are binary chunk sidecar data. Core hydration
contains authored/tagged/analytic targets, counts, bounds, and a
`denseTopologyRef`. Picking or inspection pages dense targets on demand.

## Provider and MCP context

Provider context uses dedicated queries for:

1. current authoritative version/source reference;
2. bounded recent dialogue projection;
3. explicit pinned references;
4. current raw diagnostic when relevant.

The database never materializes unrelated historic artifact graphs. Existing
context-envelope character/token budgets still apply after query budgeting.

MCP `thread_messages_get` and metadata tools use the same projection service.
Filtering and limits happen in SQL. Full-fidelity fallbacks are sectioned point
reads subject to transport limits, not full-thread reads.

## Budgets

Initial hard budgets, versioned in one backend policy:

- thread list: at most 100 summaries per page and 256 KiB serialized;
- timeline: at most 50 rows and 1 MiB serialized per page;
- notification event: at most 64 KiB, expected below 4 KiB;
- version core detail: at most 2 MiB before sectioning;
- dense topology page: at most 1 MiB and 500 targets;
- ordinary JSON IPC response/event: hard ceiling 8 MiB.

When a single field exceeds a page budget, the response returns its identity,
bounded preview, observed/allowed size, and an explicit detail read. It never
silently omits the row. A request that cannot satisfy its documented contract
within the hard transport ceiling fails with the raw observed/allowed error and
the supported sectioned read.

Budgets count serialized bytes before IPC emission. They are not estimated from
Rust heap size or row count alone.

## Activity retention

Finished user/assistant turns remain durable timeline history. Transient agent
activity is a separate bounded stream:

- backend journal has sequence cursor, per-session retention, and a maximum
  count/byte policy;
- catch-up is paged from a cursor;
- frontend retains the visible window plus pinned/error items, not every event
  since process start;
- compaction reports dropped count and next/oldest cursor explicitly.

This cannot hide interrupted-turn transcript content that belongs to durable
history.

## WebContent recovery

Memory prevention is primary. Recovery is defense in depth.

On macOS, Wry already receives
`webViewWebContentProcessDidTerminate`; Ecky SHALL connect a native termination
callback through the available runtime integration (or a minimal upstreamed
runtime hook if Tauri does not expose it). Recovery reloads the UI once, then
hydrates thread summary, selected timeline page, selected version reference,
and last durable render snapshot. It marks any unknown in-flight UI delivery as
requiring reconciliation. It does not replay provider input, queued delivery,
or render work without backend idempotency evidence.

A crash loop guard stops automatic reload after one repeated termination and
surfaces the native raw reason/recovery action.

## Observability and profiling

Development telemetry records no content:

- command/event name, direction, serialized bytes, elapsed time, row count,
  projection kind, truncation, and cache outcome;
- SQL query class, rows scanned/returned, JSON columns selected, and mutex hold
  time;
- timeline store row count and approximate retained bytes;
- render snapshot core/topology byte counts;
- WebContent/native RSS and WebKit termination reason during profiling runs.

Instrumentation must not log prompts, source, API keys, image bytes, raw payload
bodies, or full paths.

Repeatable profiling uses an isolated copy or synthetic database, never mutates
the user's live history. Fixtures include equal timestamps, 150 versions, large
message text, and multi-million dense targets. Browser heap/allocation capture
is reserved for a reproduction build; it is not taken from an active user
render.

## Storage migration

One local client owns the database. Rolling old/new client compatibility is not
required. An explicit offline maintenance command runs one bounded,
transactional migration:

1. stream each legacy artifact/manifest JSON value without materializing dense
   arrays;
2. encode dense arrays into versioned binary chunks of at most 500 targets;
3. encode the topology-free core into a versioned binary record and persist
   scalar presence/model/count columns;
4. verify core decode, chunk counts, and per-owner totals;
5. clear legacy JSON payloads and mark the schema migration complete.

Any parse, encode, write, or verification failure rolls the transaction back and
surfaces the raw migration error. Runtime read/write code supports only the new
binary schema. No JSON fallback or dual-read window remains.

Normal application startup never performs this rewrite. It checks the migration
marker and legacy-row presence only. An unmigrated database fails immediately
with the raw migration-required error; a new empty database records the marker.
The maintenance command requires an explicit database path and must run while
Ecky is closed.

## Rollout

1. Add probes and failing regression tests around the present paths.
2. Replace watcher/head/count/existence scans with scalar SQL.
3. Replace provider-context scans with bounded direct queries.
4. Replace frontend full-thread refresh/follow-up/navigation reads.
5. Replace MCP post-filtering reads.
6. Convert preview event to reference hydration and lazy topology.
7. Bound activity catch-up/retention.
8. Add WebContent recovery after payload prevention is proven.
9. Remove or make unreachable public full-thread commands.

The explicit offline database migration completes before the upgraded app is
opened. Startup exposes no history commands until the marker and binary-only
storage invariant both pass.
No history rows or snapshot content are deleted. After commit, legacy CAD JSON
payload columns contain no canonical data and runtime code cannot read them.
