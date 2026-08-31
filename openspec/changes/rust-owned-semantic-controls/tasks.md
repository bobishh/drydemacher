# Tasks: Rust-owned semantic control edits

## 1. Outer BDD

- [x] 1.1 Add real-route view-save happy path expecting one intent command.
- [x] 1.2 Add pending duplicate-suppression and raw-error scenarios.

## 2. Rust inner loop

- [x] 2.1 Add failing service tests for manual normalization, validation, delete cleanup,
  and Ecky rejection.
- [x] 2.2 Implement shared semantic manifest mutation service.
- [x] 2.3 Refactor MCP control-view handlers onto shared service.
- [x] 2.4 Add Tauri save/delete commands returning canonical manifests.

## 3. Frontend

- [x] 3.1 Add focused semantic Tauri wrapper with camelCase payloads.
- [x] 3.2 Wire ParamPanel save/delete to intent commands and returned projection.
- [x] 3.3 Add pending state without lifecycle authority.
- [x] 3.4 Route view save/delete through tagged semantic intent and remove direct
  Tauri bindings/client wrappers.

## 4. Proof

- [x] 4.1 Focused Rust tests green: 5/5.
- [x] 4.2 Focused Playwright scenarios green: happy/pending/delete and raw failure 2/2.
- [x] 4.3 `cargo check` green.
- [x] 4.4 Record remaining TypeScript semantic ownership in handoff.

## 5. Remaining semantic edits

- [x] 5.1 Add failing Rust service tests for primitive ownership/order/view attach,
  delete cleanup, advisory/relation ownership, proposal binding rebuild, and Ecky
  rejection.
- [x] 5.2 Add tagged camelCase semantic edit contracts and canonical result.
- [x] 5.3 Implement shared Rust mutation service for primitive, advisory, relation,
  and proposal edits.
- [x] 5.4 Add one Tauri command loading and persisting canonical manifest.
- [x] 5.4a Add atomic batch proposal-status intent for import enrichment callers.
- [ ] 5.5 Wire ParamPanel edit actions to tagged intent and returned projection.
- [ ] 5.6 Prove UI happy, pending, and raw failure paths for new intents.
- [ ] 5.7 Remove replacement-manifest helpers once no remaining caller needs them.

## 6. Semantic control values

- [x] 6.1 Add failing Rust tests for binding clamps, multi-hop relation propagation,
  generated Ecky AST parameters, ownership rejection, and camelCase input.
- [x] 6.2 Add exact-target Rust value-resolution intent returning a canonical patch.
- [x] 6.3 Wire workbench semantic changes to the Rust result and remove frontend patch policy.
- [x] 6.4 Prove staged happy path and raw backend failure in Playwright.
