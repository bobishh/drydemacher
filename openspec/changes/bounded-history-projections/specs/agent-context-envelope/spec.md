# Delta for agent-context-envelope

## MODIFIED Requirements

### Requirement: Context envelope enforces explicit budgets

The system SHALL assemble typed context sections under explicit per-stage and
total character ceilings, with mandatory authoritative state allocated before
optional history or references. Storage queries SHALL select only the bounded
sections needed for that envelope; post-hoc filtering of a fully materialized
thread does not satisfy this requirement.

#### Scenario: Optional sections yield first

- **WHEN** candidate context exceeds the generation ceiling
- **THEN** lowest-priority optional sections are omitted or compacted first
- **AND** the envelope records each decision and observed/allowed size.

#### Scenario: Mandatory state never truncates silently

- **WHEN** actual request, required current source, params, authoring context, or
  repair diagnostic cannot fit within the ceiling
- **THEN** provider dispatch does not occur
- **AND** the user receives a raw context-budget error naming observed and
  allowed sizes.

#### Scenario: Large history has small recent context

- **GIVEN** a thread contains large historic runtime payloads
- **WHEN** a provider or classifier assembles bounded recent context
- **THEN** SQL selects only required dialogue fields and exact current references
- **AND** historic artifact bundles, manifests, images, and topology are not
  deserialized before budgeting.

### Requirement: MCP results are structured and bounded

The system SHALL return canonical machine data in `structuredContent`, concise
text in `content`, and continuation metadata for bounded partial reads. MCP
filters, limits, and cursors SHALL apply at the storage query before payload
deserialization.

#### Scenario: Ordinary result avoids pretty-printed duplication

- **WHEN** a tool returns structured JSON
- **THEN** the result exposes that JSON in `structuredContent`
- **AND** text content summarizes outcome and identity without repeating the
  full JSON.

#### Scenario: Large read reports truncation honestly

- **WHEN** a target, manifest, message, AST, or source read exceeds its response
  budget
- **THEN** the tool returns a bounded section/window
- **AND** reports observed count, returned count, total count, truncation, and a
  continuation cursor or exact next read.

#### Scenario: Filtered message read avoids full thread

- **GIVEN** a thread contains large unrelated message payloads
- **WHEN** MCP requests a limited role/status-filtered message page
- **THEN** filtering, ordering, and limiting execute in SQL
- **AND** unrelated rows and heavy columns are not deserialized.

#### Scenario: Full-target read remains explicit

- **WHEN** the agent calls the documented full-fidelity fallback
- **THEN** complete section data remains available subject to transport safety
  limits
- **AND** the response reports its serialized size
- **AND** the fallback does not load an entire thread aggregate.
