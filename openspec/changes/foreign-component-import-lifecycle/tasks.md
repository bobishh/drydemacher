# Tasks: Foreign Component Import Lifecycle

## 1. Contract and persistence

- [x] 1.0 Represent imported Code state as a typed `freecad-component` descriptor
  derived from the selected version bundle and manifest.
- [ ] 1.1 Add failing Rust contract tests for camelCase imported-component source,
  evidence, print state, and authoring state.
- [ ] 1.2 Persist the record through existing history/service commands; no direct
  SQLite writes outside database/service ownership.
- [ ] 1.3 Project pending import state into normal agent context without chat text.

## 2. Discovery and evidence

- [x] 2.1 Prove a workspace-level root finds deeply nested files and retains the root.
- [x] 2.2 Persist selected roots through canonical `save_config` and update shared UI state.
- [ ] 2.3 Produce token-bounded stable FCStd and STEP/BREP evidence with units and
  measurements while storing the complete corpus.
- [ ] 2.4 Add summary, cursor-paged/searchable part index, and exact `partId`
  detail reads. Report total/returned/nextCursor and reject source-drift cursors.
- [x] 2.5 Keep user-visible Code report complete regardless of agent token budget.

## 3. Print branch

- [ ] 3.1 Add failing tests for trusted-unit CAD print readiness.
- [ ] 3.2 Keep conversion failure independent from printable artifact readiness.
- [x] 3.3 Address runtime by content identity and load persisted imported previews
  without invoking Ecky render or reopening donor CAD.

## 4. Authoring branch

- [ ] 4.1 Add failing context test: pending import reaches active agent without a
  synthetic message or direct LLM request.
- [ ] 4.2 Agent authors only through inspect, validate, preview, commit.
- [ ] 4.3 Link verified resulting message and optional saved component to import record.

## 5. UI outer loop

- [ ] 5.1 Add failing Playwright happy path: nested import -> dimensions -> STL
  export ready -> read-only evidence -> later editable verified Ecky.
- [ ] 5.2 Add failing Playwright failure state preserving raw import/export errors.
- [x] 5.3 Split imported Code into read-only Summary and typed Component tabs;
  gate Apply/Commit on valid `freecad-component` source.
- [ ] 5.4 Persist the descriptor as the imported version source and keep Open CAD
  bound to the copied donor.
- [x] 5.5 Remove uncalibrated proposal percentages; retain provenance and warnings.
- [x] 5.6 Keep previous Viewer model mounted during project runtime inspection and
  replace only after successful target load.
- [x] 5.7 Remove imported-source message-output lookup and route descriptor Apply
  through the imported component runtime.

## 6. Proof

- [x] 6.1 Run focused frontend/Playwright tests for implemented slice.
- [x] 6.2 Run `cd src-tauri && cargo check`.
- [x] 6.3 Run `openspec validate foreign-component-import-lifecycle --strict`.
