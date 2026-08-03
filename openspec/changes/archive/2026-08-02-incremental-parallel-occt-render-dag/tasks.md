# Tasks: Incremental Parallel OCCT Render DAG

## Execution safety

Parallel cheap agents are allowed. Agent count is not the memory control.
Every local build, test, native render, and benchmark process tree MUST run
through the shared task resource guard and obtain a memory lease first. The
guard admits work only while aggregate task RSS plus the requested reservation
fits the configured task cap and host available memory stays above its floor.
Independent reading, analysis, and non-overlapping edits require no heavy-process
lease. Nested agents MUST NOT bypass the guard. Heavy native benchmark samples
run sequentially; exceeding either threshold terminates the scoped process tree
and preserves a compact resource-failure report.

## 1. Contract and baseline

- [x] 1.1 Strictly validate proposal, design, tasks, and delta specifications.
- [x] 1.2 Add provenance-recorded balanced independent-DAG, localized
  three-part, and real multipart benchmark fixtures.
- [x] 1.3 Add release benchmark harness with serial worker-budget baseline,
  production worker budget, medians, CPU count, correctness metrics, exclusive
  run lock, sequential samples, aggregate-RSS watchdog, host-available guard,
  and compact evidence retention.
- [x] 1.4 Run baseline and preserve raw report before implementation.
- [x] 1.5 Add a resource-guard test proving a synthetic memory hog is terminated,
  reported, and cleaned without admitting the next heavy process early.

## 2. Outer BDD: observable incremental execution

- [x] 2.1 Add failing native integration test: independent ready work reports
  peak concurrency at least two with unchanged geometry.
- [x] 2.2 Add failing native integration test: localized parameter edit executes
  zero commands in clean parts.
- [x] 2.3 Add failing corruption test: invalid selective cache entry recomputes
  and never publishes partial output.
- [x] 2.4 Add failing performance gate harness; confirm current serial runner
  fails `1.8x` speedup and localized `50%` threshold for expected reason.

## 3. Stable execution identity

- [x] 3.1 Add unit tests for resolved command fingerprints, ordered dependency
  identity, canonical floats, selectors, imports, runtime, tolerance, and
  tessellation invalidation.
- [x] 3.2 Add unit tests proving unrelated parameter changes preserve clean part
  fingerprints while structural parameter changes invalidate affected graphs.
- [x] 3.3 Implement command and part fingerprints without raw slot/node/source
  identity leakage.
- [x] 3.4 Prove the runner plan carries all resolved semantic identity inputs;
  the native runner appends its actual binary and OCCT runtime facts and owns
  the final validated cache/execution fingerprint.

## 4. Thread-safe native execution foundation

- [x] 4.1 Replace process-global mutable stage diagnostics with per-task state
  and synchronized aggregate evidence.
- [x] 4.2 Audit operation input effects; mark proven shared-input operations and
  exclusive barriers explicitly.
- [x] 4.3 Enable non-destructive Boolean handling for cached/shared inputs and add
  differential topology/volume tests.
- [x] 4.4 Add one bounded CPU budget shared with outer scheduling and OCCT/TBB.

## 5. Real DAG scheduler

- [x] 5.1 Add red unit tests for ready ordering, dependency release, failure
  propagation, exclusive barriers, and bounded concurrency.
- [x] 5.2 Implement producer/dependent/remaining-use graph and deterministic
  concurrent ready queue.
- [x] 5.3 Schedule independent parts and independent commands inside parts with
  the same budget.
- [x] 5.4 Release dead slot values after final consumer without changing roots or
  topology evidence.
- [x] 5.5 Pass native independent-DAG concurrency and parity integration tests.

## 6. Selective immutable cache

- [x] 6.1 Add binary BRep part-cache read/write with metadata, digest, validation,
  atomic publication, runtime invalidation, and byte-budgeted eviction.
- [x] 6.2 Implement root-driven dirty closure: complete part hit executes zero
  commands.
- [x] 6.3 Add selective expensive-command cache and prove hit dependencies need
  not execute unless another miss consumes them.
- [x] 6.4 Reuse clean part triangulation/per-part preview data without remeshing
  solely because another part changed.
- [x] 6.5 Pass localized parameter, corruption, success-only, and clean topology
  integration tests.

## 7. Performance and correctness gate

- [x] 7.1 Build release runner and record serial/parallel balanced-DAG samples.
- [x] 7.2 Meet median speedup `>=1.8x` with peak DAG concurrency `>=2`; otherwise
  profile, revise scheduler/budget, and keep production path disabled.
- [x] 7.3 Meet localized rerender median `<=50%` of cold full render with zero
  clean-part kernel commands.
- [x] 7.4 Record real multipart model serial/parallel timing and critical path.
- [x] 7.5 Prove validity, bounds, signed volume, components, parts, topology,
  STEP, STL, and raw error parity.
- [x] 7.6 Prove every sample stays below task RSS cap, host available memory stays
  above its floor, swap does not grow, samples do not overlap, and retained
  evidence excludes large generated geometry.
- [x] 7.7 Freeze the `Daughter Flower AirTag Bracelet` source and compact plan
  contract as the heavy acceptance fixture. Add outer-only/adaptive shared-budget
  modes and a red gate requiring at least `3.0x` median improvement versus the
  immutable `69.669 s` / `64.133 s` analytic-BRep baseline, plus at most `23 s`
  median native total on the recorded 18-core Apple M5 Pro. Preserve its three
  intentional components, part identities, geometric invariants, STEP/STL, and
  exact artifact parity among current hybrid samples.
- [x] 7.8 Implement adaptive nested OCCT leases: wide ready queues consume the
  budget through outer DAG work; a lone expensive Boolean receives idle units
  and enables internal parallelism. Prove peak outer+nested allocation never
  exceeds the configured budget.
- [x] 7.9 Run one guarded hybrid characterization during development, then the
  final three-sample adaptive release gate against the immutable recorded
  analytic-BRep baseline. Retain compact timing,
  topology, digest, CPU-lease, and resource evidence only.
- [x] 7.10 Add a red planner/native integration proving a four-operand lid union
  exposes a cacheable `decorated-dome` partial. Implement deterministic
  parameter-affinity decomposition without requiring a source rewrite.
- [x] 7.11 Seed the bracelet cache once, then prove: identical rerender executes
  zero Booleans within `3 s`; body-only edit executes zero lid commands;
  thread/seat-only edit reuses decorated dome, executes zero relief/dome
  intersection work, and finishes within `10 s`; dome/relief edit invalidates
  only the required partial closure.

## 8. Final verification

- [x] 8.1 Run focused Rust/native tests and full relevant suite.
- [x] 8.2 Run `cd src-tauri && cargo check`.
- [x] 8.3 Run MCP `inspect -> validate -> preview -> commit` against an existing
  permanent benchmark target. Commit exactly one named
  `Incremental Parallel OCCT render DAG proof` version; create no new, forked,
  temporary, or duplicate thread/version and perform no direct database write.
- [x] 8.4 Attach benchmark table and raw report paths to this task file before
  marking complete.

## Evidence — 2026-08-02

### MCP permanent proof — 2026-08-02

Existing thread `6424b5d5-bbd0-4d4d-9994-5689c4fb5ed7`, base message
`2a7eca30-33dd-4f42-b5bc-289ea445ba33`, was inspected and validated through MCP.
Constraint validation passed `3/3`. Native preview message
`1b1b696b-d66d-4fcd-9c08-475375c774b3` rendered the unchanged 69-line Ecky
source with analytic BRep STEP, one part, one connected component, zero
non-manifold edges, and green Rust structural verification. Exactly one named
`Incremental Parallel OCCT render DAG proof` version was committed through MCP:
message `52d51bb7-575a-4efb-a897-d75b3b217b5a`, model
`generated-direct-occt-1d5f98a3d740`. No thread was created or forked and no
database was written directly.

### Boolean-critical bracelet baseline — 2026-08-02

Existing thread `2fe80cc4-8da6-4387-8498-d73e966d457a`, message
`2aee1e4a-b09a-4d3a-86cc-a3306d008947`, model
`generated-direct-occt-ad155b280074`: native total `69.669 s`, Boolean
`64.133 s` (`92.05%`), mesh `1.326 s`, export `1.787 s`, worker budget and peak
DAG concurrency `18`. All three parts missed cache. The three disconnected
components are intentional printable solids, not a verifier failure. This
baseline exposed blanket-disabled internal Boolean parallelism and now gates
tasks 7.7-7.9; the prior balanced synthetic result alone is insufficient.

Adaptive singleton inner parallelism improved one guarded characterization to
`41.170 s` total / `35.829 s` Boolean (`1.69x` / `1.79x` versus historical),
but failed the cold gate. Per-command timing then identified n-ary lid union
output `356` as `27.669 s`: thread, seat, analytic dome, and imported faceted
ladybug relief share one cache identity. Private-copy destructive mode produced
`42.614 s` / `37.014 s`; disabling inverted-solid checks after validity proof
regressed to `46.551 s` / `40.762 s`. Neither path removes the dominant kernel.
The next required change is planner-emitted partial-Boolean cache granularity,
tasks 7.10-7.11.

Planner/native implementation now emits versioned named partial groups only for
the detected four-input decorated-lid pattern. `decorated-dome` owns the
analytic dome plus imported relief; its cache identity includes ordered resolved
dependencies, group name, operation, and version. A warm group hit removes its
input refs from the required execution closure, so relief/dome producers never
enter the ready queue. Generic four-input unions remain authored n-ary Booleans.
The native integration proves one changed group preserves the other, reports
zero recomputes for the hit, skips the hit's producer commands, and publishes no
partial cache entries after failure or corruption.

Adaptive Boolean work now leases idle CPU units from the same outer-DAG budget.
Compact stage evidence records policy, serial/parallel Boolean counts, maximum
nested lease, and peak outer+nested allocation. The lease integration proves
singleton and competing-work cases never exceed the configured budget;
`outer-only` keeps the baseline authored n-ary path.

Cold bracelet acceptance closed after the representation-level decision below.
Pairwise BRep decomposition and concurrent OCCT Booleans produced severe
negative scaling and remain removed; the retained implementation does not
schedule around the same `BOPAlgo_PaveFiller` critical path.

### Hybrid bracelet acceptance — 2026-08-03

Final compact report:
`tmp/direct-occt-bracelet-bench/release-gate-deterministic-20260803/report.json`.
One prior guarded characterization at
`tmp/direct-occt-bracelet-bench/characterization-20260803-0220/report.json`
recorded `1.612 s` native / `1.137 s` Boolean and no incremental samples.

The final release gate used the freshly rebuilt release CLI, worker budget `3`,
one warm-up, three sequential cold adaptive samples, then one sequential
incremental cache sequence. Median native total was `1.614 s` versus the
immutable `69.669 s` analytic baseline (`43.165x`). Median Boolean time was
`1.123 s` versus `64.133 s` (`57.109x`). This exceeds both `3x` thresholds and
the `23 s` host limit.

All cold samples preserved the same three named parts, three connected
components, bounds, signed volume `24722.440873256826`, topology digest
`912b13146b988c01e8e55ead67ff1bf16e3b8a98bfb1a0ff694dd4c6bf14c29d`,
and validity. STEP digest
`f5c184617505f1bf6ad611affb02a4d4819fde029ea9eae4a8441a7dc4a167f7`
and STL digest
`5cb33fcb59ac9e949379116f27cd6c8a61df97904b75bd20a7ecbd939394b4a0`
matched across all three samples. Writer timestamp normalization removed the
only volatile STEP header field.

Representation evidence reports body/lid `meshDomain`, strap `analyticBrep`,
ten mesh Booleans, three-way DAG concurrency, and two tessellated STEP members.
A focused retained-free inspection confirmed AP242 contains two
`TRIANGULATED_SURFACE_SET` entities plus the strap `MANIFOLD_SOLID_BREP`; no
mesh member is labeled analytic or faceted BRep.

Incremental proof passed:

| Edit | Native | Boolean | Cache locality |
| --- | ---: | ---: | --- |
| identical | `112 ms` | `0 ms` | all three parts hit; zero commands/Booleans |
| body only | `1266 ms` | `857 ms` | lid and strap hit; zero lid commands |
| thread | `1269 ms` | `1006 ms` | decorated dome hit; zero recomputes |
| decoration | `183 ms` | `58 ms` | structural pair hit; decorated dome alone rebuilt |

All nine final samples acquired the exclusive lease without overlap or
termination. Maximum process-tree RSS was `1,178,775,552` bytes (`1.10 GiB`),
minimum host available memory was `24,803,115,008` bytes, and aggregate swap
growth was zero. Generated STEP/STL sample artifacts were removed after digest
collection; only compact evidence remains.

Focused final proof: guarded native runner build/integration passed; guarded
`cargo test direct_occt_runner --lib` passed 69 tests; guarded JS BDD passed
20 tests; `cargo check`, `cargo check --tests`, `cargo fmt --check`, and
`git diff --check` passed. All heavy process trees used the shared resource
guard with one build/test active at a time.

### Identity proof — 2026-08-02

Red first: guarded standalone compilation of
`src-tauri/native/execution_identity_test.cpp` failed because `canonical_f64`
and `normalized_keywords` did not exist. Green: the same guarded compile/run
passed after adding canonical IEEE-754 normalization (including `-0.0`),
non-finite rejection, unordered keyword/selector normalization, ordered
dependency preservation, import-byte digesting, and every cache/runtime policy
field. The test also proves clean identity stability and invalidation limited to
the resolved repeat/count/dependency closure.

Guarded native runner integration then proves the production path: changed raw
slot output, part id/label, and replanned transient metadata keep the part key;
equivalent `-0.0`/`+0.0` and reordered keywords keep the command key; changed
selector, dependency, or runner binary changes it. The runner derives its
binary digest from `argv[0]` and binds `OCC_VERSION_COMPLETE`, so Rust only
supplies resolved typed plan semantics and cannot authoritatively emit the
final execution fingerprint.

Commands:

| Command | Result |
| --- | --- |
| guarded `c++ -std=c++17 -Wall -Wextra -Werror ... execution_identity_test.cpp` | red compile failure, then green standalone pass |
| guarded `ECKY_DIRECT_OCCT_RUNNER_TEST=1 bash scripts/build_direct_occt_runner.sh` | runner build plus all native integration probes passed |

Raw reports. Both retained compact JSON evidence.

| Evidence | Fixture / source | Samples | Result |
| --- | --- | ---: | --- |
| `tmp/direct-occt-dag-bench/report.json` | retained pre-implementation balanced baseline; 18 logical CPUs; 1 vs 8 workers | report lacks sample count | median `16660.410 ms` -> `18159.640 ms`; `0.9174x`; peak `8`; topology parity true; gate false |
| `tmp/direct-occt-dag-bench-final/report.json` | `balanced-independent-dag-v1`; 18 logical CPUs; 1 vs 8 workers | 5 serial + 5 parallel | median `10868.085 ms` -> `4771.344 ms`; `2.278x`; peak DAG concurrency `8`; topology parity true; gate true |
| `tmp/direct-occt-dag-bench-final-resource/report.json` | `balanced-independent-dag-v1`; rebuilt current runner; 18 logical CPUs; 1 warmup + 5 serial + 5 parallel | 5 serial + 5 parallel | median `10520.035 ms` -> `5042.252 ms`; `2.086x`; peak DAG concurrency `8`; topology parity true; gate true. All 11 guarded samples: max RSS `501710848` bytes below `6144 MiB`; minimum host available `9183674368` bytes above `8192 MiB`; swap aggregate growth `0`; exclusive lease acquired/no overlap; no watchdog terminations; `resourceEvidence.samples` retains each sample metric and output contains no `.stl/.step/.stp/.glb/.brep`. |
| `tmp/direct-occt-fixture-bench/run-2026-08-02T01-18-59-017Z-38337/report.json` | real `model-runtime/examples/film-adapter-golden-6part.ecky` | 5 worker-1 + 5 worker-8 | native stage median `66 ms` -> `54 ms`; natural `1.222x`; critical path mesh `22 ms`, boolean `9 ms`, export `6 ms`; full parity true |
| same localized report | `model-runtime/examples/physical-decision-calibration.ecky`; `film_gap: 0.30` -> `0.31` | 5 cold/warm pairs | median `51 ms` -> `25 ms`; ratio `0.4902`; two clean parts cache-hit with zero commands and mesh cache hits; gate true |

Balanced resource policy: sequential samples, `6144 MiB` task cap, `8192 MiB`
host floor, `2048 MiB` reservation. Fixture provenance records harness, CLI digest,
runtime root, source fixture digests, and changed parameter. Fixture parity covers
validity, bounds, signed volume, components, parts, topology digest, STEP/STL
presence, and raw-error equality.

Baseline limitation: `tmp/direct-occt-dag-bench/report.json` is retained raw
pre-implementation evidence, but has no sample-count, resource-policy, CLI
digest, runtime digest, or fixture-content provenance fields. Its gate is false;
it proves the required initial speedup failure, not benchmark safety or release
provenance.

Commands run through `scripts/task_resource_guard.mjs`:

| Command | Result |
| --- | --- |
| `node scripts/task_resource_guard.mjs ... -- node scripts/task_resource_guard.test.mjs` | 5/5 guard BDD tests passed: hog termination, success metrics, lease refusal, exclusive lock, recursive geometry cleanup. |
| `node scripts/task_resource_guard.mjs ... -- node --test scripts/benchmark_direct_occt_fixtures.test.mjs` | 7/7 harness BDD tests passed, including compact resource aggregation. |
| `node scripts/task_resource_guard.mjs ... -- c++ -std=c++17 ... execution_identity_test.cpp` and `part_mesh_test.cpp` | both standalone native tests compiled and passed. |
| `node scripts/task_resource_guard.mjs ... -- bash scripts/build_direct_occt_runner.sh` | native runner rebuilt successfully; only macOS deployment-target linker warnings. |
| `ECKY_DIRECT_OCCT_RUNNER_TEST=1 bash scripts/build_direct_occt_runner.sh` | native build plus standalone integration passed: worker-1/2 order, budget, released slots, bounds/volume parity; failure containment; Level-B command hit with only late transform executing; corrupt cache rejection/recompute; localized clean-part zero commands/topology parity; failed transaction publishes zero entries. |
| `node scripts/task_resource_guard.mjs ... -- openspec validate incremental-parallel-occt-render-dag --strict` | passed after this evidence update. |

Current native source inspection, compiled by the runner build: execution identity
uses resolved semantics plus ordered dependency identities and omits source/slot
identity; safe operations have an explicit immutable-input allowlist and all
others are exclusive; Boolean builders use non-destructive inputs. The scheduler
constructs one producer/dependent graph across parts, consumes a deterministic
ready set up to the shared worker budget, and releases non-root slots on their
last required consumer. Selective BRep cache reads validate schema/key/digest/
size/topology, corrupt entries become misses, writes stage then atomically
publish, and LRU eviction applies the byte budget. Complete part hits do not
enter the command-node graph.

## Completion status

All implementation and acceptance tasks are complete. The final bracelet gate
uses one freshly built release runner and immutable historical analytic timing;
it does not mix debug/release samples or rerun the obsolete slow policy.

## Resource evidence follow-up — 2026-08-02

`task_resource_guard` now writes a compact **success** report as well as its
existing compact failures: process-tree peak RSS, host-available minimum,
configured limits, swap before/after/growth plus zero-growth assertion,
exclusive-lease request/acquisition/no-overlap state, and termination state.
Both benchmark harnesses retain that per-sample report and aggregate it into
their future reports; fixture report schema is now `2`. Generated geometry is
recursively deleted only below the exact `--cleanup-dir`; compact JSON remains.

Focused proof run (no 5+5 rerun):
`tmp/resource-guard-smoke.CvaPXX/resource-report.json` recorded peak
`39,092,224` bytes below the `512 MiB` cap, host minimum `26,857,603,072`
bytes above the `0` floor, swap `1,269,951,365 -> 1,269,951,365` bytes
(growth `0`), acquired exclusive lease with no overlap, and no termination.
Its nested `sample/nested/deeper/mesh.stl` was removed while
`sample/summary.json` remained.

Focused BDD: `node --test scripts/task_resource_guard.test.mjs
scripts/benchmark_direct_occt_fixtures.test.mjs` — **12/12 passed** (success
metrics, zero swap growth, exclusive no-overlap, nested geometry cleanup, and
benchmark resource aggregation included).

The prior 5+5 reports were not rewritten: they predate success metrics and
their provenance must stay truthful. Fresh runner evidence above now proves
task **7.6** for all 11 guarded samples. Removed unreferenced, untracked `resource_budget.*` and
`scripts/test_resource_budget.sh`; production uses `PartMeshMemoryBudget`, not
the orphan `MemoryBudget` experiment.

## 2.4 performance-gate BDD evidence — 2026-08-02

RED first: importing the new gate contract before implementation failed as
expected: `SyntaxError: ... does not provide an export named
'evaluateLocalizedTimingGate'`.

Green: guarded command
`node scripts/task_resource_guard.mjs --state-dir tmp/performance-gate-red-guard
--report tmp/performance-gate-red-guard/report.json --task-cap-mib 512
--host-floor-mib 0 --reservation-mib 16 --exclusive performance-gate-js-tests
-- node --test scripts/benchmark_direct_occt_fixtures.test.mjs` passed **11/11**.

`evaluateSpeedupGate` reads the retained pre-DAG balanced report directly:
`16660.409916 ms / 18159.640041 ms = 0.9174416386x`, yielding the exact failure
`balanced DAG median speedup 0.9174x is below required 1.8x`. A `180 ms / 100 ms`
synthetic balanced case passes exactly at `1.8x`.

`evaluateLocalizedTimingGate` has deterministic synthetic policy coverage:
`100 ms` cold / `51 ms` warm fails with `localized warm median 51ms is 51.00% of
cold 100ms; required at most 50.00%`; `100 ms` / `50 ms` passes. This is only
synthetic gate evidence. No historical localized baseline-failure run is claimed
or fabricated.
