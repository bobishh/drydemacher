# Proposal: Bounded History Projections

## Why

Ecky persists durable conversations correctly, but several read paths recover
that data through a full `Thread` aggregate. A dense CAD version can contain
millions of selection targets inside `artifactBundle` and `modelManifest`.
Timeline refresh, provider context assembly, project-folder status, MCP reads,
and draft-preview delivery therefore deserialize, clone, or cross IPC with
hundreds of megabytes when the caller needs a title, message row, head ID, or
snapshot identity.

The active database observed on 2026-08-28 was 6.0 GB. Its active thread held
about 454 MiB of message payload; one latest message was 8.57 MiB, while a
compact 20-row timeline projection was under 3 MiB. During a dense Voronoi
render, WebContent rose from about 0.39 GB to 4.97 GB and the native process to
about 2.1 GB. A live native sample found the project-folder watcher repeatedly
deserializing the complete active thread and millions of `ViewerEdgeTarget`
records only to resolve the current head ID. An earlier WebContent process was
terminated by WebKit after exceeding its inactive 4 GB memory limit, leaving a
white native window.

Durability and full fidelity remain required. The fix is query-shaped access:
bounded timeline rows, point hydration, stable references, lazy dense topology,
and explicit transport budgets. It is not history deletion or silent data loss.

## What Changes

- Make thread list, timeline page, version detail, source content, runtime
  snapshot, and dense topology distinct backend projections.
- Remove production full-thread aggregate reads from UI refresh, provider
  context, watcher status, MCP metadata, and point-navigation paths.
- Replace timestamp-only pagination with a stable opaque cursor that preserves
  total order for equal timestamps.
- Replace payload-bearing invalidation events with small identity/revision
  events; coalesce refreshes per thread and discard stale responses.
- Publish draft-preview identity and summary first. Hydrate the active preview
  once by reference; never broadcast the full snapshot to uninterested views or
  persist the same payload back through IPC.
- Keep dense anonymous topology behind paged/indexed queries. Core version
  detail retains counts, authored targets, analytic targets, and explicit
  truncation metadata.
- Migrate legacy JSON CAD payloads once into versioned binary core records and
  indexed binary topology chunks. Runtime readers and writers use only the new
  schema; no legacy JSON fallback remains after migration succeeds.
- Make provider context and MCP results bounded at the database query, not only
  after full rows have already been materialized.
- Bound live activity catch-up and frontend retention while preserving finished
  conversation turns as durable history.
- Add content-free payload, query, duration, row-count, and memory telemetry plus
  hard transport guards with raw observed/allowed errors.
- Detect WebContent termination and restore the durable selected thread/version
  without replaying an active provider or render job.

## Capabilities

### New Capabilities

- `history-projection`: bounded thread/timeline/detail queries, stable cursors,
  targeted invalidation, and durable recovery semantics.

### Modified Capabilities

- `render-snapshot-authority`: pass snapshot references and bounded projections;
  hydrate dense topology and active preview detail lazily.
- `agent-context-envelope`: enforce budgets at storage/query boundaries for
  provider context and MCP reads.
- `agent-visibility`: bound transient activity catch-up and retention without
  hiding durable finished turns.

## Relationship to Existing Changes

- `lossless-version-history` defines what must be retained and append ordering.
  This change defines how retained data is queried and transported.
- `hybrid-render-performance-job-control` already requires lightweight preview
  projections and lazy dense topology. This change supplies the shared UI/IPC
  contract and acceptance budgets needed to finish that work.

## Out of Scope

- Deleting or compacting valid history. The explicit offline storage migration rewrites
  representation without dropping snapshot content.
- Changing Voronoi, tessellation, Boolean, or CAD authoring semantics.
- Hiding raw errors, silently dropping timeline rows, or silently truncating a
  requested exact source window.
- Replaying provider turns or render jobs after UI recovery.
- Loading an entire thread as a supported public escape hatch.

## Impact

- Backend: new projection structs and indexed SQL point/page queries; full-row
  helpers become private migration/test utilities or are removed.
- Frontend: history store consumes summaries/pages and hydrates one selected
  version; event handlers use revisioned invalidation and singleflight.
- Provider/MCP: context and message reads use bounded query plans directly.
- Preview: event payload becomes identity/summary; runtime/topology use point
  hydration or asset-backed sidecars.
- Runtime: native WebContent termination hook restores durable UI state safely.
- Tests: Rust query/budget tests, frontend store tests, Playwright large-history
  flows, and a repeatable profiling fixture.
