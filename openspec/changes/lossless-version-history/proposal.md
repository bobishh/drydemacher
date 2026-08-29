# Change: Lossless Version History

## Why

Make version history an append-only record of authoring, not a catalogue of
only renderable CAD results. Every changed version-owned authoring source file
or persisted model draft becomes a version, including source that fails parsing,
validation, preview, or backend render. The latest append is always the thread
head; users can filter the timeline to successful versions when they need
printable geometry.

Current history counts and resolves `success` assistant messages with an
artifact bundle as versions. Failed or artifact-less attempts are therefore
not addressable as versions, and the latest successful result can remain head
after a newer failed edit. Source binding and project mirror code also reject
stale edits as `conflict`/`threadAdvanced`, which loses the append event and
requires force or re-export.

## What Changes

- Define one append path for file observations, draft persistence, manual edits,
  watcher sync, and MCP edits; finalization never creates a version.
- Preserve exact changed source bytes (or draft payload) before attempting
  validation or rendering; attach every result and raw error to that version.
- Resolve head by append order, independent of status or artifact availability.
- Keep successful-version filtering as an explicit query/projection, not head
  selection.
- Serialize concurrent writers. The append that commits last becomes head;
  no optimistic conflict, refusal, or force mode is needed for version writes.
- Keep existing history readable and migrate old records without deleting them.

## Out of Scope

- Making failed versions printable or silently repairing invalid source.
- Deleting history, deduplicating distinct content, or changing artifact
  validation rules.
- Versioning binary assets, capture frames, caches, telemetry, or local scratch
  that is not persisted as an authoring draft.
- Direct SQLite writes from frontend, mirror code, or agents.

## Proof Gates

- A changed invalid file appears as a version with exact bytes and failure
  evidence; it is head while successful filtering excludes it.
- Two changed inputs from concurrent/stale writers both remain in history, in
  serialized append order, with the later append as head and no conflict.
- Repeated observation of identical bytes creates no duplicate version.
- Existing databases load with the new head/filter semantics and retain old
  successful history.
