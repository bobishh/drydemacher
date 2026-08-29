# Tasks: Thread Source Binding

## Guardrails

- Outer Playwright BDD red first. Prove happy, invalid-save, unchanged-save,
  and concurrent-append states.
- Preserve global `history.sqlite` and `model-runtime`; no workspace migration.
- Tauri payload camelCase; Rust snake_case + `#[serde(rename_all = "camelCase")]`.
- Atomic source writes. Digest mismatch is not a conflict or refusal condition.
- No direct SQLite writes outside backend commands. No stage or commit.
- Run `cd src-tauri && cargo check` before success report.

## 1. Persistent Binding And Immediate Mirror

- [x] 1.1 Failing Rust tests: binding create/read/update, root validation,
  digest mismatch refusal.
- [x] 1.2 Failing Playwright: blank thread -> bound folder/default source.
- [x] 1.3 Add DB migration, contracts, and binding service.
- [x] 1.4 Expose `projectsRoot` as Settings source-root picker.
- [x] 1.5 Bind new threads; atomically write default source + sidecar.
- [x] 1.6 Safely backfill existing thread on first open/export.
- [x] 1.7 Focused proof and `cargo check`.

## 2. Automatic Sync

- [x] 2.1 Failing integration: Ecky/agent version refreshes bound source.
- [x] 2.2 Historical digest guard implementation (superseded by lossless
  append semantics; retain implementation history for audit).
- [x] 2.3 Historical settled watcher pipeline and unchanged-save dedupe proof
  (superseded where it drops invalid versions).
- [x] 2.4 Historical external-edit conflict proof (superseded; no longer the
  target contract).
- [x] 2.5 Make `project_folder_*` compatibility wrappers; retain internal
  preview/commit primitives.

## 3. Discoverable UI

- [x] 3.1 Failing Playwright: Project card has OPEN FILE/REVEAL FOLDER path.
- [x] 3.2 Rename Params control; show open error in UI, not console only.
- [x] 3.3 Implement source-root picker and responsive layout proof.

## 4. Agent Source Path

- [x] 4.1 Tests: target metadata includes source path/folder/state.
- [x] 4.2 Agent prompt edits source then syncs, never export-first.
- [x] 4.3 MCP proof: file edit -> sync -> version.
- [x] 4.4 Inventory/deprecate public macro mutation tools only after green path.

## 5. Proof And Archive

- [x] 5.1 Relevant unit + focused Playwright happy/failure.
- [ ] 5.2 Full `npm run test:unit`, `npm run test:e2e`, `cd src-tauri && cargo check`, `cargo test`.
- [x] 5.3 `openspec validate thread-source-binding --strict`.
- [ ] 5.4 Archive through `openspec archive` after all work is green.

## 6. Lossless Version Append Migration (follow-up)

- [ ] 6.1 Add immutable version/append-order and explicit head persistence (or
  equivalent transactionally ordered query) without deleting existing rows.
- [ ] 6.2 Change Ecky, agent, and external saves to append every changed save,
  including invalid validation/render results; attach status and raw errors.
- [ ] 6.3 Remove conflict refusal, raw conflict UI, and force-overwrite paths;
  serialize writers and make last append head.
- [ ] 6.4 Preserve unchanged-observation dedupe while proving two changed saves
  never collapse or lose a version.
- [ ] 6.5 Add migration tests covering invalid saves, concurrent/ordered
  appends, head selection, successful-status filtering, and legacy history.
- [ ] 6.6 Run focused BDD, full unit/e2e, `cargo check`, `cargo test`, and
  `openspec validate thread-source-binding --strict` after migration.
