# Design: Thread Source Binding

## Storage

Keep global `history.sqlite`. Add:

```sql
CREATE TABLE thread_source_bindings (
  thread_id TEXT PRIMARY KEY REFERENCES threads(id) ON DELETE CASCADE,
  folder_path TEXT NOT NULL,
  source_path TEXT NOT NULL UNIQUE,
  source_digest TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
)
```

Each folder reuses the existing recoverable `ecky-project.edn` mirror
manifest. Thread binding does not add a second sidecar, schema version, or
legacy parser branch:

```clojure
{:exported-at 1781200000
 :message-id "message-..."
 :model-id "model-..."
 :project-id "proj-..."
 :schema-version 1
 :source-digest "sha256:..."
 :thread-id "thread-..."}
```

SQLite owns exact binding lookup. The existing mirror sidecar keeps the folder
intelligible and repairable. Neither changes source authority: file is current
working copy, SQLite retains Ecky history.

## Lifecycle

```text
new thread
  -> projectsRoot/<thread-slug>/model.ecky
  -> write default source + binding

Ecky/agent version commit
  -> serialize append with any concurrent source writer
  -> atomically write source and append immutable version
  -> attach validation/render status (including failure)
  -> advance head to appended version + new digest

external editor save
  -> watcher settles digest
  -> serialize append with any concurrent source writer
  -> append immutable version, even when validate/render fails
  -> attach validation/render status + new digest
  -> advance head to appended version
```

Workbench creation uses one `create_design_thread` intent with `mode`, optional
title, and source only for macro mode. Rust allocates thread identity and
atomically creates the SQLite thread plus bound blank source. In macro mode the
same intent appends source before validation, renders it, attaches success or
raw failure to that initial version, refreshes the binding, and returns one
bounded workspace projection. The frontend does not open a blank thread and
then invoke a second manual commit. New-project creation, edited-source fork,
and manual commit without an active thread all use this intent; no frontend
caller manufactures a thread identity. Edited-source forks may provide an exact
immutable base thread/version identity. Rust rejects partial or stale identity
before mutation and derives language, backend, parameters, controls, and
post-processing from that persisted base instead of frontend copies.

Each settled changed save is one append event. Append order is the only head
order; the last append transaction always becomes head. Version records are
immutable and retain source bytes (or a durable source reference/digest) plus
validation/render status and raw failure details when available.

Ecky-originated writes and external saves use one serialized append boundary. A
digest mismatch is not a conflict and never causes refusal, rollback, history
loss, or a force-overwrite path. Unchanged watcher observations are deduped.

## UI

- Rename Params `FILE ↗` to `OPEN FILE`.
- Add `OPEN FILE` and `REVEAL FOLDER` to active Project card actions.
- Add Settings source-root picker using `config.projectsRoot` and show resolved
  path/raw filesystem errors.
- No decorative sync badges. Source actions state exact path and state.

## Agent Boundary

`workspace_overview`/target metadata adds `sourcePath`, `sourceFolder`, and
`sourceState`. Agent edits `sourcePath` with normal file tools, then invokes
source sync. It does not export an ad-hoc mirror or replace macro code through
public mutation tools. Preview/commit remain backend internals.

## BDD Proof

```gherkin
Given a selected source root
When a blank thread is created
Then its folder and default model.ecky exist immediately
And history.sqlite contains the exact binding
```

```gherkin
Given a bound active thread
When Projects renders
Then OPEN FILE and REVEAL FOLDER are visible
And OPEN FILE invokes the bound source path
```

```gherkin
Given an external editor changes a bound source
When the watcher settles the save
Then exactly one immutable version appends even if validation or rendering fails
And the new version records validation/render status and raw failure details
And head points to that version
And Ecky preserves the external source bytes
```

```gherkin
Given two writers save changed source in quick succession
When both saves settle
Then both changed saves append immutable versions in serialized order
And head points to the last appended version
And neither save is rejected as a conflict
```
