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

Each folder reuses the existing recoverable `ecky-project.json` mirror
manifest. Thread binding does not add a second sidecar, schema version, or
legacy parser branch:

```json
{
  "schemaVersion": 1,
  "projectId": "proj-...",
  "threadId": "thread-...",
  "messageId": "message-...",
  "modelId": "model-...",
  "sourceDigest": "sha256:...",
  "exportedAt": 1781200000
}
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
  -> check binding digest
  -> atomically write source when clean
  -> persist version + new digest

external editor save
  -> watcher settles digest
  -> validate -> preview -> existing commit handler
  -> persist version + new digest
```

Ecky source writes happen before version commit; failed write means no version.
External source is never rewritten. Any digest mismatch refuses an
Ecky-originated write with raw conflict error. No normal UI force overwrite.

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
Then exactly one new version commits
And Ecky preserves the external source bytes
```
