## Why

Authored `verify` clauses can execute geometric and artifact checks, but they
cannot state why a check exists, when it applies, or whether failure blocks the
build. This makes assembly-only rules run against print layouts, collapses
warnings into hard failures, and leaves agents and authors to recover intent
from comments.

## What Changes

- Extend top-level `verify` with optional named `(intent ...)`, `(severity ...)`,
  and `(when ...)` sections while retaining required `tag`, `metric`, and
  `expect` sections.
- Keep `(tag ...)` as the stable authored check identity; do not introduce a
  second requirement identifier or a separate requirements DSL.
- Evaluate `(when ...)` against resolved render parameters and emit an explicit
  `skipped` authored check when the condition is false.
- Support `error` and `warning` severities. Error failures remain blocking;
  warning failures remain visible without making structural verification red.
- Carry intent, severity, condition evidence, and skip reason through the Rust
  result contract, generated TypeScript contract, MCP responses, and existing
  verification chips.
- Update the existing centralized Ecky language source and run its existing
  projection pipeline as implementation hygiene.
- Explicitly exclude manual/pending checks and a new `(requirements ...)` form.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `verify-authoring`: Add optional intent, severity, and condition sections;
  skipped evaluation; non-blocking warning semantics; typed evidence output;
  and existing-UI presentation.

## Impact

- Ecky parser, Core IR, legacy emitter, and roundtrip tests.
- Authored verification evaluator and structural result finalization.
- Rust/TypeScript boundary contracts and MCP verification payloads.
- Existing PromptPanel/New Params verification presentation and tests.
- Centralized language source and its existing freshness checks.

Existing three-section `verify` clauses remain valid and retain blocking error
semantics. No geometry operator, render backend, or database schema changes.
