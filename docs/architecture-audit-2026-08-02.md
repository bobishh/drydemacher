# Ecky Architecture Audit - 2026-08-02

Scope: boundary correctness, persistence safety, render pipeline, async/state shape, and agent/CAD invariants.

## Findings

- [fixed] `src-tauri/src/test_get.rs` opened a hardcoded production `history.sqlite` path with default read/write flags. It was unreferenced debug code in `src/`, so it violated the no-direct-SQLite-write boundary by proximity.
- [fixed] `src-tauri/src/debug_ast.rs`, `src-tauri/src/parser_test.rs`, and `src-tauri/src/test_macro.rs` were unreferenced parser/debug scratch files in production source.
- [fixed] `src-tauri/src/db.rs` had a `test_read_real_db` unit test referencing a local production `history.sqlite` path. Removed and added a fitness guard against local app-history paths in Rust source.
- [fixed] Campaign frontend clients called `@tauri-apps/api/core` directly. Generated Specta contracts were stale by 20 Rust commands; regenerated contracts and routed clients through `src/lib/tauri/client.ts`.
- [fixed] Frontend architecture fitness now rejects raw Tauri `invoke` imports outside generated contracts. `convertFileSrc` remains allowed as asset URL API, not command transport.
- [tracked-debt] `src-tauri/src/services/render.rs` is a 5k-line dispatcher/lowering/fallback/finalization module. Best-practice drift: too many backend selection and export invariants live in one function cluster. See `docs/render-state-audit.md`.
- [tracked-debt] `src-tauri/src/mcp/server.rs`, `src-tauri/src/mcp/runtime.rs`, `src-tauri/src/db.rs`, and `src/App.svelte` are mega-files. Risk: hidden coupling, lock-order bugs, and hard-to-audit UI state.
- [tracked-debt] `AppState` uses many `Arc<Mutex<_>>`/`tokio::Mutex<_>>` stores. Some are justified, but ownership boundaries are not explicit enough for a CAD/editor core.

## Desired Shape

- Tauri commands go through generated `src/lib/tauri/contracts.ts` and the normalized client wrapper.
- Rust boundary structs use `#[serde(rename_all = "camelCase")]` unless they are protocol-fixed JSON-RPC structs.
- SQLite history mutations flow through Rust services/MCP commands only.
- Render flow is explicit: source -> parse/compile -> plan/lower -> backend render -> post-process -> manifest/export.
- Expensive render/conversion work never hides behind UI-side state mutation.

## Next Fix Slices

- Keep `cargo run --bin export_contracts` / `npm run generate:contracts` in the normal boundary workflow; stale generated contracts hid 20 commands.
- Split render dispatch decision into a pure, unit-tested planner before moving backend execution code.
- Expand architecture fitness tests when adding new persistence surfaces.
- Extract MCP JSON-RPC tool schemas/dispatch from server transport.

## Proof

- `npm run typecheck` passed.
- `cd src-tauri && cargo check` passed.
- `cd src-tauri && cargo test --test architecture_fitness` passed with 4 guards.
- `npm run generate:contracts` is deterministic for current source; pre/post `src/lib/tauri/contracts.ts` SHA-256 matched.
