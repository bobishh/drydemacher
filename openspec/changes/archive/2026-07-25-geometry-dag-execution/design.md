## Context

`OcctPartPlan` is an ordered command list, but each `OcctArg::Ref` already
defines a dependency edge. The native runner accepts n-ary union/difference and
uses `SetRunParallel(true)`. `repeat-union` normalization, however, produces
union commands that often become a single difference tool:

```text
cutters -> union -> difference(base, union-result)
```

The desired execution is:

```text
cutters ----------------> difference(base, cutter...)
```

Fillet, chamfer, shell, transforms, selector sources, and explicit topology
bindings are semantic boundaries. They must not be flattened.

## Goals / Non-Goals

**Goals:**

- Represent command dependencies explicitly inside the planner post-pass.
- Flatten union-valued difference tools recursively when safe.
- Preserve root identity, argument order, keyword dependencies, selectors, and
  topology bindings.
- Remove dead intermediate union commands.
- Prove equivalent output and measure the real repeated-cut workload.

**Non-Goals:**

- Persistent per-node BREP cache.
- Concurrent command evaluation inside the native runner.
- Replacing OCCT internal boolean parallelism.
- General boolean algebra rewriting.
- Changing the serialized plan ABI.

## Decisions

### Planner-local graph

Create a producer map keyed by `OcctSlot`. Dependency extraction recursively
walks positional arguments, nested lists, and every keyword `source_arg`.
Missing/future references and cycles fail before runner serialization.

Alternative rejected: rewrite Core IR during normalization. At that point the
planner does not yet know every keyword/topology consumer.

### Safe tool flattening

For each difference argument after index zero, recursively inspect its producer.
Flatten only a keyword-free `OcctOp::Union` whose arguments are shape refs.
Never flatten the base operand, a transform around a union, or another boolean
kind. Preserve tool order and duplicates.

The rewrite may bypass a union output for the difference while retaining that
union command when another reachable positional or keyword consumer needs it.

### Stable reachability

After rewriting, traverse backward from the part root through positional and
keyword refs. Retain reachable commands in original source order. This gives
deterministic serialization while removing bypassed union nodes.

### Parallelism boundary

The optimized difference reaches existing n-ary runner code as one object plus
many tools. OCCT remains responsible for internal parallel intersections.
Outer command scheduling is deferred: current native diagnostics use mutable
process-global stage state and would race.

### Benchmark runner compatibility

The real Toothbrush Holder expresses frame origins and rotations through both
point3 literals and evaluated three-number lists. The precompiled runner SHALL
accept those forms for `plane :origin`, `plane :x`, `plane :normal`, and
`location :offset`/`:rotate`. A singleton union SHALL evaluate as its sole
shape, allowing a repeated-cut group with one generated tool to retain normal
union semantics. These are fixture-enabling compatibility fixes, not a broad
frame or Manifold execution expansion.

### Proof fixtures

- Real fixture: MCP-exported `Toothbrush Holder Versions`, source digest
  `sha256:81f7ded44df1dbd1d38588fe2db876e721130889acbc235e4faa5c3b3c7e033f`.
  It contains nested `repeat-union` cutter groups.
- Compact fixture: multi-tool difference followed by fillet and chamfer. It
  proves the rewrite stops at topology-changing consumers.

Benchmark reports source digest, command counts before/after, boolean command
counts, wall-clock time, and artifact topology/volume parity. Timing is
evidence, not a brittle CI threshold.

## Risks / Trade-offs

- Tolerance-sensitive booleans can produce different topology ordering.
  Mitigation: compare validity, bounds, volume tolerance, and exported topology
  contract before enabling the rewrite broadly.
- Recursive flattening can erase a union needed by selectors.
  Mitigation: keyword-aware dependency traversal and dedicated regression test.
- Small models may see no measurable speedup.
  Mitigation: no mandatory timing threshold; retain structural proof.
- Full Toothbrush rendering is slow for normal CI.
  Mitigation: fast planner test always runs; native performance test is opt-in.
- Runner frame semantics could diverge from generated source.
  Mitigation: live runner test covers point3/list forms and emits STEP/STL.

## Migration Plan

1. Add red planner tests.
2. Add graph analysis and rewrite behind the normal planning entry point.
3. Prove generated runner plan uses one multi-tool difference.
4. Run compact native parity and opt-in Toothbrush benchmark.
5. Roll back by removing the planner post-pass; plan ABI and source remain
   unchanged.

## Open Questions

- Whether measured construction cost justifies future ready-node scheduling.
- Whether persistent BREP node serialization beats existing whole-render cache.
