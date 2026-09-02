## Context

`verify` currently stores three positional sections (`tag`, `metric`, `expect`)
and turns every failed or unevaluable check into a blocking structural issue.
Authors therefore cannot express the check's purpose, gate assembly-only checks
by a render parameter, or retain a visible advisory check without failing the
model. The result crosses parser, Core IR, evaluator, Rust/TypeScript contracts,
MCP projections, existing verification UI, and generated language docs.

## Goals / Non-Goals

**Goals:**

- Add optional named `intent`, `severity`, and `when` sections to `verify`.
- Preserve old three-section clauses without source migration.
- Make false conditions explicit `skipped` evidence.
- Make warning expectation failures visible but non-blocking.
- Preserve one typed result through backend, MCP, and existing UI.
- Document syntax once in the centralized language source and regenerate all
  projections.

**Non-Goals:**

- No separate `requirements` DSL or second check identifier.
- No manual, pending, or externally acknowledged checks.
- No frontend condition evaluation.
- No arbitrary Scheme evaluation inside `when`.
- No geometry or render-backend changes.

## Decisions

### Named, order-independent sections

`tag`, `metric`, and `expect` remain required exactly once. `intent`, `severity`,
and `when` are optional and accepted at most once. The parser dispatches sections
by name instead of position and rejects unknown or duplicate sections. The
emitter uses canonical order: `tag`, `intent`, `severity`, `when`, `metric`,
`expect`.

This retains the current source shape while making extensions readable. A
keyword tail was rejected because existing verify payload is already sectioned.
A second `requirements` form was rejected because `tag` already supplies stable
identity and `verify` already owns executable evidence.

### Bounded condition language

`when` accepts a boolean literal, a boolean parameter symbol, or nested `not`,
`and`, and `or` forms over those operands. Evaluation uses resolved effective
parameters from the same render snapshot used by verification. Unknown symbols,
non-boolean parameters, invalid arity, and unsupported operators are authored
verification errors.

Arbitrary Scheme evaluation was rejected: it would duplicate compiler/runtime
semantics and make evidence non-deterministic. The bounded grammar covers
assembly/print-layout toggles without introducing an expression VM.

### Explicit skipped result

False `when` emits one `AuthoredVerifyCheck` with status `skipped`, canonical
condition text, resolved false result, and skip reason. Metric resolution and
expectation evaluation do not run. Skipped checks never create structural issues
and never affect overall `passed`.

Omitting `when` means unconditional execution. Existing clauses therefore keep
current behavior.

### Severity controls expectation failure only

Severity values are `error` and `warning`; omission defaults to `error`.
Failed `error` expectations create blocking structural issues. Failed `warning`
expectations remain typed failed checks, render amber, and do not make structural
verification fail. Passed checks render green regardless of severity.

Malformed clauses, invalid conditions, unsupported metrics, and other evaluation
errors remain blocking even when severity is `warning`. Severity cannot hide a
broken verification program.

### One typed boundary contract

Core IR gains optional typed sections. Public authored-check evidence gains
`severity`, optional `intent`, optional canonical `condition`, optional
`conditionResult`, and optional `skipReason`; status gains `skipped`. Rust owns
evaluation and blocking semantics. MCP forwards this contract. Frontend only
maps typed status/severity to existing chips and accessible labels.

## Risks / Trade-offs

- [Condition grammar may need enum comparisons later] -> Ship only boolean
  gates now; extend grammar through a future spec with typed tests.
- [Warning failure may be mistaken for success] -> Keep check status `failed`,
  show amber severity, and reserve overall green/red for blocking result.
- [Old consumers may not know `skipped`] -> Regenerate TypeScript contracts and
  update exhaustive mappings in the same slice.
- [Optional fields create inconsistent messages] -> Stable typed fields are
  authoritative; message remains human summary only.

## Migration Plan

1. Add parser/Core IR support with old-source roundtrip tests.
2. Add evaluator and public contract support with failing integration tests
   first, then focused unit tests.
3. Regenerate TypeScript contracts and update MCP/UI consumers.
4. Update centralized syntax docs and run the existing freshness pipeline.
5. Preserve default `error` and unconditional behavior for all existing source.

Rollback removes optional sections and new evidence fields; no stored source is
rewritten automatically.

## Open Questions

None. Product decisions fixed as 1A, 2A, 3A, and 4A.
