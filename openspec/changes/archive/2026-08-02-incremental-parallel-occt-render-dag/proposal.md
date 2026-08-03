# Proposal: Incremental Parallel OCCT Render DAG

## Why

Direct OCCT currently owns parallel work only inside individual Boolean and
meshing calls. The runner still evaluates parts and commands as serial ordered
lists. Whole-bundle cache identity includes the complete parameter set, so one
localized parameter change rebuilds every part and every downstream artifact.

The archived `geometry-dag-execution` change improved one repeated-cut fixture
from roughly 1.39 s to 1.22 s, but explicitly deferred concurrent command
evaluation and persistent node BRep reuse. That improvement is insufficient for
interactive parametric CAD. The missing execution architecture now becomes the
work, not another planner-local rewrite.

## What Changes

- Assign deterministic content identities to resolved Direct OCCT parts and
  cache-admitted commands from operation semantics and dependency identities.
- Persist successful immutable analytic BRep results with runtime, tolerance,
  imported-resource, and tessellation identity.
- Reuse clean part roots and admitted clean command results after localized
  parameter changes; execute only the dirty transitive closure.
- Evaluate independent parts and independent ready command nodes concurrently
  under one bounded CPU budget shared with OCCT/TBB internal parallelism.
- Make native diagnostics, stage accounting, and result publication safe under
  concurrent evaluation.
- Preserve exact BRep, selectors, topology evidence, STEP/STL correctness, raw
  errors, deterministic plan semantics, and success-only cache publication.
- Add structural concurrency evidence and real native before/after benchmarks,
  with a task-level RSS watchdog preventing benchmark host exhaustion.
  Production enablement requires material speedup, not a small regression-noise
  improvement.

## Capabilities

### Modified Capabilities

- `geometry-dag-execution`: execute independent ready work concurrently and
  report real concurrency instead of using the graph only for rewrites.
- `direct-occt-runtime`: add selective immutable part/node BRep caches and
  localized invalidation beneath the final whole-bundle cache.

## Impact

- Direct OCCT plan/runtime contracts carry every resolved semantic input needed
  for cache identity. Rust owns resolution; the native runner owns the final
  fingerprint because it alone can bind the running binary digest and actual
  OCCT runtime identity.
- Native runner gains a bounded dependency scheduler, thread-safe diagnostics,
  immutable cache reads, and atomic cache writes.
- Runtime bundle output remains authoritative. No source-language behavior,
  frontend debounce, render cancellation, SQLite write, or hidden fallback is
  introduced.
- Existing whole-render cache remains the final fast path. Selective caches
  serve new parameter identities whose clean subgraphs overlap prior renders.
- Cache schema/version changes invalidate prior selective entries without
  invalidating authored source or committed history.
