# Delta for geometry-dag-execution

## ADDED Requirements

### Requirement: Stable resolved execution identity

The system SHALL assign each Direct OCCT command and part a deterministic
content identity derived from resolved semantics and ordered dependency
identities, excluding unrelated parameters and unstable source/slot identity.

#### Scenario: Unrelated parameter preserves identity

- **GIVEN** a parameter is referenced only by part B
- **WHEN** the parameter changes and the model is replanned
- **THEN** part A and its commands retain their prior execution identities
- **AND** part B identities change along the affected transitive closure.

#### Scenario: Runtime input invalidates identity

- **WHEN** an imported byte payload, selector payload, tolerance policy,
  tessellation policy, runner ABI, backend cache schema, or OCCT runtime changes
- **THEN** every affected execution identity changes
- **AND** no prior incompatible shape is admitted as a cache hit.

### Requirement: Concurrent ready-node execution

The system SHALL execute independent ready commands and parts concurrently under
a bounded shared worker budget while preserving dependency order and authored
semantics.

#### Scenario: Independent branches overlap

- **GIVEN** two shape-producing commands have all dependencies satisfied
- **AND** neither command requires an exclusive effect barrier
- **WHEN** the native runner evaluates the part DAG with worker budget greater
  than one
- **THEN** both commands may execute concurrently
- **AND** stage evidence reports peak DAG concurrency of at least two.

#### Scenario: Dependency waits

- **GIVEN** command C consumes outputs from commands A and B
- **WHEN** A and B execute concurrently
- **THEN** C starts only after both results publish successfully
- **AND** failure of either dependency prevents C from executing.

#### Scenario: Independent parts share scheduler

- **GIVEN** a plan has multiple independent parts
- **WHEN** more than one worker is available
- **THEN** ready work from different parts may overlap
- **AND** one slow part does not force unrelated part construction into serial
  order.

### Requirement: Incremental n-ary Boolean decomposition

The system SHALL materialize stable cacheable partial results inside an
expensive n-ary Boolean when independently varying operand groups would
otherwise invalidate an unchanged expensive intersection.

#### Scenario: Thread edit reuses decorated relief

- **GIVEN** a lid union combines thread, seat, analytic dome, and imported
  faceted relief
- **AND** a prior successful render cached the decorated dome partial
- **WHEN** only a thread or seat parameter changes
- **THEN** the decorated dome is loaded from cache
- **AND** the relief/dome intersection executes zero times
- **AND** the final artifact preserves validity, topology contract, bounds,
  volume, components, STEP, and STL truth.

#### Scenario: Relief edit invalidates decorated relief

- **WHEN** the dome or relief resolved identity changes
- **THEN** the decorated dome partial misses and recomputes
- **AND** unrelated parts and partial Boolean nodes retain their prior cache
  identities.

### Requirement: Immutable concurrent slot values

The system SHALL treat published slot values as immutable and SHALL prevent
concurrent operations from mutating shared input topology.

#### Scenario: Proven shared input

- **WHEN** multiple ready commands consume one published shape
- **THEN** they execute concurrently only when their operations have proven
  immutable-input behavior or receive defensive copies
- **AND** cached input topology remains unchanged.

#### Scenario: Unknown effect becomes barrier

- **WHEN** an operation lacks immutable-input proof
- **THEN** it executes under an exclusive effect barrier
- **AND** no concurrent consumer observes partial mutation.

### Requirement: Shared parallelism budget

The system SHALL coordinate outer DAG scheduling with OCCT/TBB internal
parallelism through one bounded process-wide CPU budget.

#### Scenario: Worker budget one

- **WHEN** runner worker budget is one
- **THEN** command execution follows deterministic dependency order without
  outer overlap
- **AND** output remains the semantic baseline for parallel parity tests.

#### Scenario: Production budget

- **WHEN** production worker budget exceeds one
- **THEN** total outer and nested kernel work stays within the configured budget
- **AND** the runner does not create an unconstrained pool per part or command.

#### Scenario: Boolean critical path receives idle capacity

- **GIVEN** one expensive Boolean is the only runnable heavy command
- **AND** the configured process budget has idle CPU units
- **WHEN** the scheduler starts that Boolean
- **THEN** it grants a bounded nested kernel lease and enables OCCT internal
  parallelism
- **AND** active outer units plus nested units never exceed the configured
  process budget.

#### Scenario: Wide ready queue favors outer overlap

- **GIVEN** multiple independent heavy commands are runnable
- **WHEN** their combined outer work can consume the process budget
- **THEN** nested kernel leases are reduced or withheld
- **AND** the commands overlap through the outer DAG without nested
  oversubscription.

### Requirement: Material native DAG speedup evidence

The system SHALL NOT claim or enable production DAG parallelism until release
native benchmarks prove material improvement and semantic parity.

#### Scenario: Balanced independent DAG gate

- **GIVEN** a provenance-recorded balanced fixture with at least four comparable
  independent native branches and a host with at least four logical CPUs
- **WHEN** at least five serial and five parallel cold-cache samples are compared
  by median after warm-up
- **THEN** parallel execution is at least 1.8 times faster
- **AND** peak DAG concurrency is at least two
- **AND** validity, bounds, volume, components, parts, and topology match.

#### Scenario: Insufficient speedup

- **WHEN** measured speedup is below 1.8 times
- **THEN** production DAG parallelism remains disabled
- **AND** the change remains incomplete rather than reporting a smaller gain as
  success.

#### Scenario: Boolean-critical bracelet gate

- **GIVEN** the frozen `Daughter Flower AirTag Bracelet` fixture whose
  analytic-BRep baseline spends `64.133 s` of `69.669 s` in Boolean work
- **WHEN** at least three sequential adaptive cold-cache hybrid samples from the
  same release runner are compared after warm-up
- **THEN** median native total and Boolean time are each at least `3.0` times
  faster than the immutable recorded baseline
- **AND** median native total on the recorded 18-core Apple M5 Pro reference
  host is at most `23 s`
- **AND** current sample artifact digests match each other
- **AND** three intentional connected components, part identities, bounds,
  volume within tessellation tolerance, STEP, STL, validity, and watertightness
  remain equivalent to the baseline
- **AND** evidence declares body and lid `meshDomain` and both STEP members tessellated
- **AND** stage evidence reports parallel Boolean execution without exceeding
  the configured CPU budget.

#### Scenario: Mixed hybrid export

- **GIVEN** mesh-domain body/lid roots and an analytic strap root
- **WHEN** one export bundle is written
- **THEN** STL uses each part's canonical triangle stream without remeshing the
  mesh-domain lid
- **AND** STEP contains the unchanged analytic strap plus AP242 tessellated body
  and lid members
- **AND** topology/stage evidence never labels the tessellated body or lid analytic.

#### Scenario: Bracelet incremental gate

- **GIVEN** one successful cold bracelet render seeded part, command, and
  partial-Boolean caches
- **WHEN** the model rerenders unchanged
- **THEN** it executes zero Boolean commands and finishes within `3 s` on the
  reference host
- **WHEN** only thread or seat parameters change
- **THEN** relief/dome intersection executes zero times
- **AND** total native time is at most `10 s` on the reference host.

### Requirement: Resource-safe benchmark execution

The release benchmark harness SHALL execute heavy samples sequentially under a
scoped single-instance lock and SHALL supervise aggregate task RSS plus host
available memory. Multiple agents may perform non-heavy independent work.

#### Scenario: Benchmark memory watchdog

- **WHEN** a sample exceeds the configured hard task RSS cap or host available
  memory reaches its floor
- **THEN** the harness terminates that sample with a bounded grace period
- **AND** records the resource failure instead of starting another sample
- **AND** removes sample-scoped large artifacts.

#### Scenario: Compact benchmark evidence

- **WHEN** a measured sample finishes
- **THEN** timing, stage, topology, digest, and resource metrics remain available
- **AND** generated geometry artifacts are not retained unless an explicit
  diagnostic-retention flag was set before the run.
