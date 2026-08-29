# Tasks: Ecky CLI Surface

## Worker Rules

- Use subagents for disjoint write scopes only.
- No worker may revert unrelated edits.
- Workers must report changed files and tests run.
- Keep raw backend error detail intact.
- Run `cd src-tauri && cargo check` before claiming completion.

## 1. W1 - CLI Parser, Check, Lower

Write scope:

- `src-tauri/src/bin/ecky.rs`
- CLI parser/unit tests colocated with binary if added

Tasks:

- [x] 1.1 Add `ecky` binary entry.
- [x] 1.2 Implement usage/help text.
- [x] 1.3 Implement `check` command with compile diagnostics.
- [x] 1.4 Implement `lower` command with backend selection.
- [x] 1.5 Implement `--out` handling and stdout fallback.
- [x] 1.6 Implement exit-code mapping for usage/check/lower failures.

Evidence: `cd src-tauri && cargo test --test ecky_cli` (7 passed); `cd src-tauri && cargo check`; `openspec validate ecky-cli-surface --strict`.

## 2. W2 - Render Orchestration

Write scope:

- `src-tauri/src/bin/ecky.rs`
- `src-tauri/src/services/render.rs` only if seam required

Tasks:

- [x] 2.1 Parse `--param key=value` and `--params file.json`.
- [x] 2.2 Merge parameter sources deterministically.
- [x] 2.3 Route render to `build123d`.
- [x] 2.4 Route render to `freecad`.
- [x] 2.5 Route render to `direct-occt`.
- [x] 2.6 Copy requested STL/STEP outputs and fail if missing.
- [x] 2.7 Add optional `--json` render summary.
- [x] 2.8 Preserve raw backend/runtime errors.

Evidence: `cd src-tauri && cargo test --bin ecky render_` (4 passed: FreeCAD/direct-OCCT routing, STEP copy/missing artifact, raw backend detail); `cd src-tauri && cargo test --test ecky_cli render_` (3 passed: malformed params/output usage and build123d STL render with JSON params overridden by CLI `--param` plus JSON summary).

## 3. W3 - Proof, Docs, Smoke

Write scope:

- `README.md`
- CLI integration tests or smoke scripts
- OpenSpec task updates

Tasks:

- [x] 3.1 Add README CLI examples tied to docs/tutorial flow.
- [x] 3.2 Add proof for `check` happy/fail path.
- [x] 3.3 Add proof for `lower` build123d/freecad path.
- [x] 3.4 Add proof for one render backend with params.
- [x] 3.5 Run `cd src-tauri && cargo check`.
- [x] 3.6 Run targeted CLI tests/smokes.

Evidence: `cd src-tauri && cargo check`; `cd src-tauri && cargo test --test ecky_cli` (11 passed: check success/failure, build123d/freecad lower, build123d render with JSON parameters overridden by `--param`); `cd src-tauri && cargo test --bin ecky` (4 passed: backend routing, STEP copy/missing artifact, raw backend detail); `openspec validate ecky-cli-surface --strict`.

## 4. Main Thread Integration

Tasks:

- [x] 4.1 Review worker patches for overlap/regression.
- [x] 4.2 Re-run CLI-targeted tests after integration.
- [x] 4.3 Update tasks as slices land.
- [x] 4.4 Leave packaging/global install for later change.

Evidence: reviewed `src-tauri/src/bin/ecky.rs` and `src-tauri/tests/ecky_cli.rs`; packaging remains explicit out-of-scope in proposal; cargo check, targeted tests, and strict validation above pass.
