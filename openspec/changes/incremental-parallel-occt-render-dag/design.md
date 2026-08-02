# Design: Incremental Parallel OCCT Render DAG

## Context

`OcctPartPlan` is already a deterministic dependency DAG encoded as ordered
commands and slot references. Current native execution ignores DAG concurrency:
it loops over parts, then loops over commands. Individual OCCT Booleans and the
mesher request parallel execution, but independent construction branches never
overlap.

Current Direct OCCT cache stores complete bundles. Its identity includes full
source and full parameter JSON. Any parameter change creates a cache miss for
the entire model even when resolved geometry changes in one part only.

## Goals

- Execute independent Direct OCCT work concurrently inside one render job.
- Reuse successful clean geometry across parameter edits.
- Make dirty execution proportional to the affected transitive closure.
- Retain analytic BRep and exact STEP authority.
- Produce measurable speedup with correctness evidence.

## Non-Goals

- Frontend debounce, latest-task suppression, cancellation, or request queues.
- Approximate geometry, silent mesh conversion, or reduced preview quality.
- Parallelizing dependency-related nodes whose ordering is semantically fixed.
- Byte-identical STEP/STL output when semantic topology evidence proves parity.
- Direct SQLite manipulation or cache metadata stored in application history.
- Unbounded thread creation or independent thread pools fighting OCCT/TBB.

## Decisions

### 1. Resolved semantic fingerprints

Rust serializes the fully normalized, parameter-resolved, graph-optimized plan.
The native runner computes the final fingerprints from those semantics, then
binds the digest of its running binary and its actual OCCT runtime identity.
Only the runner can truthfully supply those runtime facts. Raw node IDs, source
offsets, labels, and unrelated parameter values are not identity inputs.

Command fingerprint:

```text
sha256(
  cache-schema,
  operation,
  resolved non-reference arguments,
  normalized keywords and selector payloads,
  ordered dependency fingerprints,
  imported resource byte digests,
  geometry/tolerance policy,
  runner ABI and OCCT runtime identity
)
```

Part fingerprint adds ordered command/root identity, part representation, and
part-affecting export settings. A parameter affects a part or command only by
changing the resolved plan or a dependency fingerprint. This naturally covers
structural `if`, `repeat`, and component expansion without trusting unstable
post-expansion slot numbers.

Input order stays significant. Floating-point identity uses canonical IEEE-754
bits with normalized negative zero and rejected non-finite values.

### 2. Two selective cache levels

Level A caches every successful analytic part root. This is the primary
localized-parameter fast path.

Level B caches only admitted expensive shape-producing commands: Boolean,
fillet, chamfer, shell, loft, sweep, solidify/import, hull, and operations whose
measured cost crosses the configured admission threshold. Cheap primitives and
simple transforms recompute unless needed to load a cached dependent.

Level B identity at an authored n-ary Boolean boundary is insufficient when one
operand change invalidates a much more expensive unchanged operand interaction.
The planner therefore materializes deterministic cacheable partial Booleans for
independently varying operand groups. For the bracelet lid this means a stable
`decorated-dome = union(dome, ladybug-relief)` node followed by the final union
with thread and seat. A thread/seat edit reuses the decorated dome and MUST NOT
repeat the faceted-relief/analytic-dome intersection. Grouping and reduction
order are explicit plan semantics, fingerprinted, and admitted only after
validity/topology parity proof; this is not an implicit source rewrite.

Analytic shapes use OCCT `BinTools` binary BRep. Cache metadata records schema,
fingerprint, representation, runtime identity, artifact digest, byte size, and
whether triangulation is present. Manifold/indexed-mesh entries continue using
their canonical indexed representation; no BRep claim is fabricated.

Cache path is outside model history under the application data cache root.
Writes use a unique temporary file, validation, digest creation, fsync where
supported, and atomic rename. Failure, cancellation, invalid BRep, digest
mismatch, or incomplete metadata publishes nothing. Eviction is byte-budgeted
LRU. Cache reads never update SQLite.

### 3. Dirty closure

Every render still performs Rust parsing, normalization, and planning. Those
steps are cheap compared with kernel work and produce current fingerprints.

Native admission walks from every part root:

- complete part cache hit: load root; execute zero commands for that part;
- command cache hit: load result; mark its dependency closure unnecessary
  unless another miss consumes those dependencies;
- miss: execute the minimal backward closure required by the missed root;
- invalid cache entry: treat as miss, remove/quarantine entry, report cache
  rejection in stage evidence, then recompute normally.

No stale geometry may survive a changed fingerprint.

### 4. Bounded dependency scheduling

Runner constructs producer, dependency, dependent, and remaining-use tables
from every positional, nested, and keyword reference. Missing references and
cycles still reject before kernel execution.

Ready nodes enter a deterministic priority queue ordered by original command
index. Workers may start multiple ready nodes concurrently. Results publish to
their slots only after successful completion. Dependents become ready after all
required results publish.

Parts are independent DAG roots and share the same scheduler. The scheduler
must therefore expose concurrency across both parts and branches inside one
part. A part-level-only implementation does not satisfy this change.

One process-wide execution budget controls outer workers and OCCT/TBB work.
Default budget derives from available logical CPUs and remains configurable for
benchmarking. `workers=1` is the semantic baseline. Nested oversubscription is
forbidden: implementation uses one shared TBB arena/global control or an
equivalent proven budget mechanism, not one unconstrained pool per layer.

The budget is adaptive, not an outer-worker-only switch. Every running DAG node
owns one CPU unit. A kernel operation may acquire an additional bounded nested
lease from currently idle units. Wide ready queues favor outer DAG overlap;
when the ready width collapses onto one expensive Boolean critical path, that
Boolean receives the idle units and enables OCCT internal parallelism. Blanket
`SetRunParallel(false)` for every Boolean does not satisfy this decision. Lease
accounting MUST guarantee that active outer units plus granted nested units
never exceed the configured process budget.

### 5. Immutable slot contract

Published slot values are immutable. Concurrent consumers may share them only
after every admitted operation proves it does not mutate inputs.

- OCCT Boolean builders use non-destructive mode for cached/shared inputs.
- Operations without proven immutable-input behavior execute behind an
  exclusive effect barrier or receive an explicit defensive copy.
- Selector/topology reads may run concurrently only over immutable shapes.
- Process-global current-stage strings and unsynchronized timing maps are
  replaced by per-task diagnostics plus synchronized aggregate counters.

Unknown operations default to exclusive execution. Safety beats speculative
parallelism, but the benchmark fixture must contain enough proven independent
work to demonstrate real overlap.

### 6. Export reuse

Changed parts receive fresh topology and tessellation. Clean part cache entries
may retain validated triangulation and per-part STL identity. The combined STEP
writer remains authoritative and may serialize the compound each render.

Merged preview construction must not remesh clean parts solely because one part
changed. It may reuse cached per-part triangulations or combine validated
per-part triangle streams. Production bundle still contains complete viewer
assets and export evidence.

### 7. Evidence contract

Stage report schema records:

- configured worker budget;
- peak concurrently executing DAG nodes;
- per-part cache hits/misses;
- command cache hits/misses;
- executed command count per part;
- kernel elapsed time and total elapsed time;
- cache read/write/rejection counts;
- Boolean, mesh, verify, and export counts.
- serial/parallel Boolean counts, maximum nested kernel lease, and peak total
  allocated CPU units.

Timing evidence uses release runner binaries. Debug-build timings cannot satisfy
the performance gate.

Benchmark harness runs one native sample at a time. It refuses concurrent
instances through a scoped lock, checks host available memory before each sample,
and supervises runner RSS. A sample exceeding the hard task RSS cap or host
minimum-available threshold receives `SIGTERM`, then `SIGKILL` after a short
grace period, and is reported as a resource failure. Only compact plans, stage
reports, topology summaries, digests, and timing JSON persist by default. Large
generated STEP/STL/GLB/BRep sample artifacts are removed after metrics are
recorded. Failed runs clean scoped temporary files.

## Performance Gates

### Balanced independent native DAG

A provenance-recorded fixture contains at least four independent, comparable,
real OCCT branches and at least two parts. On a host with at least four logical
CPUs:

- compare identical cold-cache executions with worker budget `1` and production
  budget;
- use at least five measured samples after one unmeasured process warm-up;
- run measured samples sequentially under the task-RSS and host-memory watchdog;
- compare medians;
- parallel median MUST be at least `1.8x` faster than serial median;
- stage evidence MUST report peak DAG concurrency of at least `2`;
- output validity, bounds, signed volume, component count, part count, and
  topology contract MUST match.
- every sample MUST remain within declared RSS and host available-memory bounds,
  without swap growth or watchdog termination.

If the gate fails, production parallel scheduling remains disabled and the
change remains incomplete. A smaller improvement cannot be reported as success.

### Localized parameter rerender

A three-part fixture gives each part a disjoint parameter and expensive native
subgraph. After a successful cold render, change only the middle-part parameter:

- first and third part fingerprints remain identical;
- first and third parts report cache hits and zero executed kernel commands;
- middle part reports a miss and executes its dirty closure;
- output reflects the changed parameter and preserves clean-part topology;
- warm localized median MUST be no more than `50%` of a cold full-render median.

For expensive n-ary Booleans, dirty closure applies below the authored command
boundary. After one successful cold seed:

- an identical rerender executes zero Boolean commands and completes within
  `3 s` on the reference host;
- a body-only edit loads the complete lid part and executes zero lid commands;
- a thread/seat-only edit loads the decorated-dome partial, executes zero
  relief/dome intersection work, and completes within `10 s`;
- only a dome or relief identity change invalidates the decorated-dome partial.

### Real model evidence

At least one existing provenance-recorded multipart project fixture runs through
serial and parallel modes. Report its natural speedup and critical path. This
fixture protects against benchmark-only architecture but does not replace the
`1.8x` balanced-DAG gate.

### Boolean-critical AirTag bracelet

The frozen `Daughter Flower AirTag Bracelet` fixture is the acceptance case for
adaptive nested OCCT parallelism. Its recorded outer-only baseline is
`69.669 s` native total, including `64.133 s` Boolean time (`92.05%`). It has
three intentional disconnected printable solids; component count three is not
an error even though the authored model has fewer part declarations.

Development uses one guarded before/after characterization sample. The final
gate uses at least three sequential cold-cache samples per policy from the same
release runner, after one unmeasured warm-up. Compare the existing outer-only
policy with the adaptive shared-budget policy:

- adaptive median native total MUST be at least `3.0x` faster;
- adaptive median Boolean time MUST be at least `3.0x` faster;
- on the recorded 18-core Apple M5 Pro reference host, adaptive median native
  total MUST be at most `23 s`; the stretch target is `15 s`;
- topology, bounds, volume, three components, STEP, and STL MUST match;
- stage evidence MUST show at least one parallel Boolean and MUST prove peak
  allocated CPU units never exceeds the configured budget;
- every sample remains under the existing RSS, host-memory, swap, exclusivity,
  and compact-retention guards.

If this gate fails, the render-speed change remains incomplete regardless of
the balanced synthetic DAG result.

The cold gate does not replace the incremental gate. Repeating the dominant
relief/dome intersection for an unrelated slider change is a correctness defect
in dirty-closure granularity even if cold performance later meets its target.

## Risks and Mitigations

- **OCCT input mutation:** non-destructive mode, effect barriers, copies, and
  differential validity tests.
- **Topology ordering changes:** compare durable semantic targets plus geometric
  invariants; never assume thread scheduling order is topology identity.
- **Cache corruption:** digest validation and atomic success-only publication.
- **Cache lookup overhead:** selective admission and stage-level accounting.
- **Oversubscription:** one total CPU budget shared with OCCT/TBB.
- **Critical-path starvation:** lend idle outer capacity to one eligible heavy
  kernel command, then reclaim it before admitting competing outer work.
- **Host exhaustion:** memory reservations, RSS/available-memory guards,
  sequential heavy samples, hard watchdog termination, and compact evidence.
- **False cache hits after runtime changes:** runner binary, OCCT version, ABI,
  cache schema, tolerances, imports, and tessellation settings in identity.
- **Memory pressure:** remaining-use release, memory-aware admission,
  byte-budgeted cache, bounded mesh/export scratch, and no accumulated large
  sample artifacts.

## Rollout

1. Land fingerprints, evidence schema, and disabled scheduler/cache paths.
2. Prove deterministic scheduler concurrency using controlled executors.
3. Prove native parallel parity with `workers=1` baseline.
4. Enable part cache after localized invalidation proof.
5. Enable admitted command cache after differential proof.
6. Enable production parallel budget only after `1.8x` gate passes.
7. Keep serial mode as diagnostic fallback, not silent retry after a parallel
   kernel failure.
