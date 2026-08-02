# Tasks: Thread Source Binding

## Guardrails

- Outer Playwright BDD red first. Prove happy and external-edit/conflict state.
- Preserve global `history.sqlite` and `model-runtime`; no workspace migration.
- Tauri payload camelCase; Rust snake_case + `#[serde(rename_all = "camelCase")]`.
- Atomic source writes. Never clobber a digest mismatch.
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
- [x] 2.2 Guard every Ecky source write with binding digest.
- [x] 2.3 Settled watcher edit -> validate -> preview -> commit -> digest update;
  no duplicate version per save.
- [x] 2.4 External-edit conflict test proves no file clobber/raw error.
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
