## MODIFIED Requirements

### Requirement: Top-level verify clauses parse under model

The system SHALL accept a top-level `(verify ...)` clause in `.ecky` source so
authors can declare verification intent beside geometry authoring. Each clause
SHALL contain exactly one `tag`, `metric`, and `expect` section and SHALL accept
at most one optional `intent`, `severity`, and `when` section in any order.
Unknown and duplicate sections SHALL fail compilation.

#### Scenario: Extended top-level verify parses

- **GIVEN** `.ecky` source with required sections plus optional `intent`,
  `severity`, and `when` sections in non-canonical order
- **WHEN** the source is parsed and compiled
- **THEN** every section is preserved in authored program constraints
- **AND** geometry compilation continues through existing preparation flow

#### Scenario: Existing three-section verify parses

- **GIVEN** `.ecky` source with only `tag`, `metric`, and `expect`
- **WHEN** the source is parsed and compiled
- **THEN** the clause remains unconditional with `error` severity

#### Scenario: Nested verify is rejected

- **GIVEN** `.ecky` source that places `(verify ...)` inside a geometry or
  expression form
- **WHEN** the source is parsed and compiled
- **THEN** compilation fails
- **AND** the diagnostic identifies `verify` as unsupported in that nested position

#### Scenario: Missing required verify section is rejected

- **GIVEN** `.ecky` source whose verify clause omits `tag`, `metric`, or `expect`
- **WHEN** the source is parsed and compiled
- **THEN** compilation fails
- **AND** the diagnostic names the missing required section

#### Scenario: Duplicate or unknown verify section is rejected

- **GIVEN** `.ecky` source whose verify clause repeats a section or contains an
  unsupported section name
- **WHEN** the source is parsed and compiled
- **THEN** compilation fails with the exact duplicate or unknown section

### Requirement: Verify clauses preserve tag metric and expect payload

The system SHALL preserve authored `tag`, `intent`, `severity`, `when`, `metric`,
and `expect` payload in core IR and SHALL emit sections in canonical order:
`tag`, `intent`, `severity`, `when`, `metric`, `expect`.

#### Scenario: Verify payload preserves authored values

- **GIVEN** a verify clause containing symbols, strings, booleans, numbers, and
  nested list forms
- **WHEN** the source is compiled
- **THEN** the authored section payload is preserved in order
- **AND** nested list shape remains intact

#### Scenario: Extended verify clause roundtrips through emit

- **GIVEN** source with every supported verify section
- **WHEN** the source is compiled, emitted back to legacy source, and reparsed
- **THEN** the emitted source contains each section once in canonical order
- **AND** the reparsed program preserves equivalent payload

### Requirement: Structural verification evaluates authored verify additively

The system SHALL evaluate authored verify clauses additively in structural
verification entrypoints that already consume runtime bundles without replacing
the existing structural result carrier. Rust SHALL return typed intent,
severity, condition, skip, metric, comparator, expected, actual, and diagnostic
evidence for each authored check when applicable.

#### Scenario: Error expectation failure blocks structural verification

- **GIVEN** an Ecky-generated model with an `error` verify clause whose
  expectation fails against runtime evidence
- **WHEN** generated-model verification runs
- **THEN** the returned result reports `passed = false`
- **AND** a typed authored failure is appended beside structural issues
- **AND** existing artifact digest reporting remains intact

#### Scenario: Warning expectation failure remains visible and non-blocking

- **GIVEN** an Ecky-generated model with a `warning` verify clause whose
  expectation fails
- **WHEN** generated-model verification runs
- **THEN** the authored check has failed status and warning severity
- **AND** no structural issue is added for that expectation failure
- **AND** the warning alone does not make the result fail

#### Scenario: False condition emits skipped evidence

- **GIVEN** a verify clause whose valid boolean `when` condition resolves false
- **WHEN** generated-model verification runs
- **THEN** the authored check has `skipped` status, canonical condition,
  false condition result, and skip reason
- **AND** metric and expectation evaluation do not run
- **AND** the skipped check does not affect overall pass status

#### Scenario: Boolean condition reads effective parameters

- **GIVEN** `when` contains a boolean parameter or nested `not`, `and`, and `or`
  forms
- **WHEN** verification runs with resolved effective render parameters
- **THEN** the condition uses those exact resolved values
- **AND** condition evidence identifies the canonical expression and result

#### Scenario: Invalid condition remains a blocking authoring error

- **GIVEN** `when` references an unknown or non-boolean parameter, invalid arity,
  or unsupported operator
- **WHEN** generated-model verification runs
- **THEN** the authored check has error status
- **AND** structural verification fails even if authored severity is warning

#### Scenario: Unsupported authored metric reports deterministic error

- **GIVEN** an Ecky-generated model with readable authored source and a verify
  clause that references an unsupported metric namespace or key
- **WHEN** generated-model verification runs
- **THEN** the request succeeds without panic
- **AND** an authored verification error is reported through the structural issue list

#### Scenario: MCP summary reflects blocking additions only

- **GIVEN** verification contains error failures, warning failures, and skipped checks
- **WHEN** MCP structural verification summary runs
- **THEN** typed authored checks retain all three outcomes
- **AND** `issueCount`, `passed`, and summary text reflect blocking structural
  issues rather than warning or skipped check count

## ADDED Requirements

### Requirement: Existing verification UI presents typed authored outcomes

The existing verification chips SHALL present passed checks as green, blocking
failed/error checks as red, warning expectation failures as amber, and skipped
checks as neutral. The frontend SHALL consume backend status and severity and
SHALL NOT evaluate authored conditions.

#### Scenario: Warning failure is visible without red model state

- **GIVEN** structural verification passes except for one warning expectation failure
- **WHEN** the existing verification UI renders the result
- **THEN** the model result remains passing
- **AND** the authored check appears amber with its intent and warning severity

#### Scenario: Skipped condition is explainable

- **GIVEN** an authored check was skipped by a false condition
- **WHEN** the existing verification UI renders the result
- **THEN** a neutral chip shows skipped state and skip reason
- **AND** no frontend condition evaluation occurs

