## 1. Specification

- [x] 1.1 Document dependency graph, safe rewrite admission, topology boundaries, and proof fixtures.
- [x] 1.2 Validate the OpenSpec change strictly.

## 2. Planner BDD

- [x] 2.1 Add failing planner test for nested cutter-union flattening and dead union removal.
- [x] 2.2 Add failing regression tests for keyword/topology consumer retention and transform boundaries.
- [x] 2.3 Add failing graph validation tests for missing refs and cycles.

## 3. Graph Optimization

- [x] 3.1 Implement positional/nested/keyword dependency extraction.
- [x] 3.2 Implement recursive difference-tool union flattening with stable argument order.
- [x] 3.3 Implement root reachability and stable dead-command elimination.
- [x] 3.4 Apply graph optimization in the normal Direct OCCT planning path.

## 4. Native Proof and Benchmark

- [x] 4.1 Prove runner serialization sends every flattened tool to one n-ary difference builder.
- [x] 4.2 Add compact difference-to-fillet/chamfer parity fixture.
- [x] 4.3 Add opt-in Toothbrush Holder benchmark using the MCP-exported source digest.
- [x] 4.4 Record before/after structural counts, artifact parity, and elapsed timing.

## 5. Verification

- [x] 5.1 Run focused planner, runner, and native fixture tests.
- [x] 5.2 Run `cargo check` from `src-tauri`.
- [x] 5.3 Run application smoke proof without changing frontend behavior.

## Evidence

- Benchmark: two real Toothbrush Holder proof runs measured `1383 ms` ->
  `1244 ms` and `1391 ms` -> `1217 ms`; commands `327` -> `294`; unions `36`
  -> `3`. Baseline/optimized artifact parity passed (volume, area, components,
  topology face and edge counts).
- Native barrier parity: optimized multi-tool difference preserved downstream
  fillet and chamfer against the unoptimized runner plan.
- Verification: focused planner/runner/native fixture tests and
  `cd src-tauri && cargo check` passed. Production build passed; live route
  `5173` returned HTML and backend `3001` health responded. Browser attachment
  ended with `SIGKILL`; no UI change belongs to this change.
