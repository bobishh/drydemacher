# Tasks: Source-Aware Model Authoring

## 1. Hide Detached Sketch Workspace

- [x] 1.1 Add failing Playwright test proving Sketch launcher absent.
- [x] 1.2 Add failing Playwright test proving stale persisted Sketch visibility
  cannot mount window while another persisted window still restores.
- [x] 1.3 Remove Sketch launcher from dock.
- [x] 1.4 Normalize hidden-window state in window store so show/toggle/layout
  restoration cannot expose Sketch Workspace.
- [x] 1.5 Keep Sketch implementation and persisted draft data intact.
- [x] 1.6 Run focused Playwright happy and stale-layout scenarios.
- [x] 1.7 Ignore saved Sketch preview drafts during workbench boot so dormant
  diagnostic geometry cannot replace current model.

## 2. Shared Authoring Graph

- [x] 2.1 Add backend tests joining AST stable node keys, dependency graph,
  feature outputs, constraints, and stable viewer target IDs.
- [x] 2.2 Extract shared service from MCP-facing shape/dependency/selector logic.
- [x] 2.3 Add camelCase Tauri authoring graph contract and command.
- [x] 2.4 Mark targets without exact binding non-editable with raw reason.
- [x] 2.5 Add frontend contract tests rejecting incomplete editable targets.

## 3. Synchronized Read-Only Lenses

- [ ] 3.1 Add failing Playwright test: geometry selection focuses owning AST node,
  upstream params, and affected output targets.
- [ ] 3.2 Add failing Playwright test: AST selection highlights model targets.
- [ ] 3.3 Add failure proof for ambiguous or missing provenance.
- [ ] 3.4 Render compact focused dependency trace in viewer overlay.
- [ ] 3.5 Preserve full source graph in New Params/source lens.

## 4. Source-Backed Direct Handles

- [ ] 4.1 Define and test `HandleBinding` contract.
- [ ] 4.2 Add failing Playwright radius-handle drag scenario.
- [ ] 4.3 Add stale source/node digest scenario proving attempted draft becomes a
  failed version with raw backend error and current head.
- [ ] 4.4 Implement guarded preview patch lifecycle.
- [ ] 4.5 Implement box, cylinder, extrusion, transform, hole, fillet, and
  chamfer adapters incrementally under separate red-green cycles.
- [ ] 4.6 Append every distinct persisted draft as an immutable version before
  validation; move head on every append.
- [ ] 4.7 Expose latest-successful filtering without changing head semantics.
- [ ] 4.8 Serialize stale/concurrent draft appends without conflict refusal,
  force mode, overwrite, or history deletion.

## 5. Source-Authored Point Handles

- [ ] 5.1 Define stable point identity and emitted local frame.
- [ ] 5.2 Add polygon/path point drag proof.
- [ ] 5.3 Add constrained or non-editable point failure proof.
- [ ] 5.4 Add loft control-point support after polygon/path proof.

## 6. LLM Manipulation Integration

- [ ] 6.1 Route annotation and language intent into candidate AST patches.
- [ ] 6.2 Require same inspect -> append version -> validate -> preview -> record
  status lifecycle.
- [ ] 6.3 Add ambiguity confirmation scenario.
- [ ] 6.4 Prove LLM path records stale source or node digests as version failure
  evidence without losing attempted content.

## 7. Verification

- [ ] 7.1 Run relevant frontend unit tests.
- [ ] 7.2 Run happy plus failure/pending Playwright scenarios per UI slice.
- [ ] 7.3 Run `cd src-tauri && cargo check` after Rust changes.
- [x] 7.4 Run `openspec validate source-aware-model-authoring --strict`.
- [ ] 7.5 Inventory dormant Sketch code before any deletion proposal.
