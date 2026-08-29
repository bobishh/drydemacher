## 1. Outer Red

- [x] 1.1 Add Playwright scenario: large Ecky manifest opens grouped ownership sections
  directly below search instead of one flat list; confirm current failure.
- [x] 1.2 Add Playwright scenario: selecting a proven part/target reveals only linked
  viewport controls; confirm current disabled-overlay failure.
- [x] 1.3 Add Playwright pending scenario: ambiguous or empty target exposes no editable
  overlay and no all-parameter fallback.

## 2. Backend Inner Loop

- [x] 2.1 Add failing Rust test for two parts with disjoint parameter dependencies.
- [x] 2.2 Preserve reachable AST dependencies in native `PartBinding.parameterKeys` and
  derive stable part/model groups.
- [x] 2.3 Add failing Rust tests for transitive named build dependencies and unused build
  exclusion; implement the resolver and named groups/feature nodes.
- [x] 2.4 Add failing direct-OCCT test for authored topology binding to narrow parameter
  keys; decode bindings and implement exact mapping.
- [x] 2.5 Keep unbound/ambiguous topology parameter keys empty and mesh fallback part-only.
- [x] 2.6 Reject stale semantic carry-forward for Ecky previews; keep freshly compiled
  provenance and controls authoritative.

## 3. Frontend Inner Loop

- [x] 3.1 Add failing unit test for deterministic ownership sections with shared keys
  rendered once and dense sections collapsed.
- [x] 3.2 Render ownership sections directly below search using manifest part/group
  provenance; preserve Tactical Midnight, square borders, and overflow boundaries.
- [x] 3.3 Expand/foreground selected part or target scope; keep unrelated sections below
  and collapsed.
- [x] 3.4 Enable viewport overlay only in Select mode with non-empty exact generated-Ecky
  provenance; preserve Orbit/Measure and ambiguous-empty behavior.
- [x] 3.5 Update AST map projection/tests to consume the same ownership rules.

## 4. Authoring Contract

- [x] 4.1 Add prompt/card guidance for stable parts, meaningful build shapes, explicit
  feature primary params, and interaction-critical topology tags.
- [x] 4.2 Add/adjust validation tests proving inference works without LLM-authored Views.

## 5. Integration And Dryer

- [x] 5.1 Run focused Rust, frontend unit, and Playwright tests; fix regressions in scope.
- [x] 5.2 Run `cargo check` from `src-tauri`.
- [x] 5.3 Reinspect and validate bound filament-dryer target through MCP.
- [ ] 5.4 Rebuild dryer through supported source/preview flow; inspect part/group
  parameter keys and verify generated model.
- [ ] 5.5 Record Playwright and persisted-manifest proof for grouped default state,
  selected part editing, and ambiguous/pending state.

## 6. Project Folder Latency

- [x] 6.1 Add failing integration test: a settled `model.ecky` edit emits detection and
  enters apply within two seconds, then produces one version.
- [x] 6.2 Replace minute-scale watcher ownership with filesystem notification plus
  one-second trailing debounce and short fallback polling.
- [x] 6.3 Prove repeated/missed notifications do not create duplicate applies.

## 7. Semantic-Only Native Render

- [x] 7.1 Add failing test: tag-face-only edit leaves geometry digest unchanged but
  updates selection/tag manifest.
- [x] 7.2 Split geometry and semantic cache identity; reuse BRep/topology when evaluated
  geometry, parameters, dependencies, and runner binary are unchanged.
- [x] 7.3 Remove authored-binding part-cache disablement and cover it with a regression.
- [x] 7.4 Prove runner binary digest change forces exactly one cold geometry render.

## 8. Preview And Verification Truth

- [x] 8.1 Add project-card BDD: current head without image never silently displays an
  older preview under the current timestamp.
- [x] 8.2 Render explicit placeholder/stale state for a head with no current preview.
- [x] 8.3 Make declared separated print layout disconnects nonblocking for folder apply;
  preserve blocking behavior for real structural/authored failures.
