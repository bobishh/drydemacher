## Why

Direct OCCT plans already contain slot references and n-ary boolean support, but
the planner preserves nested cutter unions and the runner executes every
intermediate command. Repeated-cut models therefore fuse cutter groups before
performing a cut, wasting a complete boolean pass and hiding the dependency
structure needed for safe optimization.

## What Changes

- Build an internal dependency graph from command arguments and keyword source
  references after Direct OCCT planning.
- Rewrite a union used as a difference tool into ordered direct tool operands
  when that rewrite preserves every semantic/topology consumer.
- Remove only commands unreachable from the part root and keyword/selector
  dependencies.
- Keep each resulting multi-tool boolean as one OCCT operation with existing
  `SetRunParallel(true)` ownership.
- Admit benchmark-required runner forms only: `plane` point3/three-number-list
  frame vectors, `location` point3/three-number-list `:offset`/`:rotate`, and
  singleton union identity.
- Add a real Toothbrush Holder repeated-cut benchmark fixture plus a compact
  fillet/chamfer topology-barrier fixture.
- Record semantic parity and cold timing before claiming a performance win.

## Capabilities

### New Capabilities

- `geometry-dag-execution`: Dependency analysis, safe boolean graph rewrites,
  reachability, and performance evidence for Direct OCCT plans.

### Modified Capabilities

- `direct-occt-plan`: Planned commands gain a behavior-preserving graph
  optimization phase before native runner serialization.

## Impact

- `src-tauri/src/ecky_cad_host/direct_occt.rs`: graph construction, rewrite,
  reachability, tests.
- `src-tauri/src/ecky_cad_host/direct_occt_runner.rs` and native fixture tests:
  runner-plan/parity proof and narrow Toothbrush frame-form compatibility.
- Direct OCCT plan ABI remains ordered commands; no frontend, MCP, source
  language, selector, or persistence contract changes.
- Persistent node caching and outer C++ command scheduling remain follow-up
  work. Current render-level cache remains unchanged.
- Manifold transforms, indexed-mesh admission, and multipart Manifold export
  are not part of this change.
