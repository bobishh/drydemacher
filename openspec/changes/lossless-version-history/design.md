# Design: Lossless Version History

## Version record

Treat a version as an append event containing the exact authoring content
snapshot and its provenance. Source-backed records retain exact version-owned
source file bytes/digest; structured model-draft records retain the canonical
serialized draft payload/digest. Binary assets, capture frames, caches,
telemetry, and non-persisted scratch remain outside model version history. The
record
also carries status (`pending`, `working`, `success`, `error`, or `discarded`),
validation/render evidence, raw backend/provider error detail, and optional
artifact/model metadata. Content and append identity never change. Result
metadata may be completed after validation without creating another content
version.

Every version also owns a canonical `versionInputDigest`. Its input is the
complete effective authoring/render payload: exact source bytes, resolved
parameters (the full map, not the caller patch), dialect, engine, source
language, geometry backend, UI parameter contract, normalized post-processing,
locked component dependencies, dereferenced external-input content digests, and
render/cache schema identities that can change produced geometry. Display-only
labels, status, diagnostics, timestamps, artifact paths, and produced artifact
bytes are excluded. Map ordering is canonical, so equal inputs produce one
digest independent of insertion order.

An observation whose content digest equals the most recently appended content
for that source/draft is a no-op. A changed observation appends exactly once,
even when compilation, validation, preview, or artifact production fails.

## Head and projections

Head is the newest append by the serialized append sequence (a monotonic
database sequence or equivalent timestamp+rowid ordering), including discard
tombstones. Versions are never hard-deleted by ordinary authoring actions. Head
is not the newest successful/renderable record. `get_thread_latest_version`, source
binding metadata, context, and project status use this head. A separate
successful filter/query selects assistant versions with successful validation
and any artifact requirements; existing successful timeline views can use that
projection without changing head.

Deleted/trash behavior remains recoverable and must not renumber append order.
Restoring a record exposes that record but does not rewrite later history or
move head backward.

## Write pipeline

All writers use the backend append service, which owns serialization and
deduplication:

```text
inspect current source/draft
  -> append changed exact snapshot (head advances immediately)
  -> validate / preview / render
  -> attach status, evidence, artifact, or raw failure to that version
```

`commit` and `finalize` are not authoring primitives. Canonical MCP flow is
inspect -> append -> validate -> preview -> verify. Preview/render/verify attach
outcomes automatically to the appended version. No separate persistence,
commit, or finalize command exists. A failed validation/render marks the
appended version `error` with the raw diagnostic. A later repair appends a new
snapshot. No caller may roll back an append because a check failed.

Runtime cache ownership is `(durableVersionId, versionInputDigest)`. A worker
must receive the already-appended version ID and its digest; it may attach a
result only when both still match that version. The underlying immutable
artifact store may deduplicate identical artifact bytes across versions, but a
`modelId`, artifact path, or artifact content hash alone is never sufficient to
select a version runtime. Cache loss causes deterministic rebuild from the
version inputs. Cache publication occurs only after successful render and never
mutates an artifact directory owned by another version/digest.

Source watchers and project-folder apply read the bytes once, append that exact
snapshot, then process it. They must not compare against a thread head to reject
the write. A concurrent/stale writer is simply another serialized append; the
last accepted append is head and the earlier snapshot remains queryable.

## Compatibility and migration

Keep the existing message table and payloads readable. Backfill append order
from existing rowid/timestamp order; treat existing successful artifact
messages as successful versions and preserve existing failures as history
records. During rollout, old clients may still request a successful projection;
the backend serves it explicitly while head APIs return the latest append.
Change version counts and source-binding head lookups to count/resolve all
version records, while exposing a separate successful count/filter where the UI
needs printable-only totals. Do not rewrite source files or delete SQLite rows.

Project manifests remain mirrors and are rebased after every append. Their
status may report file freshness, but `conflict`, `threadAdvanced`, and force
gates are not version-write outcomes; stale edits append and rebase normally.

## Ownership and safety

The backend append service is the only history mutation boundary. Frontend,
watchers, MCP handlers, and agents call it; none write SQLite directly. Tauri
payloads stay camelCase while Rust remains snake_case with serde translation.
Raw diagnostics remain visible. Exact source bytes and digests are retained for
recovery and audit.
