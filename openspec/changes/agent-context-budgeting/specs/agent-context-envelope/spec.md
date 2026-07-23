## ADDED Requirements

### Requirement: API policy appears once in a stable prefix

The system SHALL place the output contract and shared language reference once in API
system content, ahead of variable thread state, and SHALL NOT copy the technical system
prompt into user content.

#### Scenario: Generation payload has no duplicate policy

- **WHEN** an API generation payload is assembled
- **THEN** technical output rules occur once in the system message
- **AND** user content contains current state and current request without a duplicate
  `EXECUTION RULES` block.

#### Scenario: Variable state follows stable content

- **WHEN** two turns use the same provider, backend, language, and output contract
- **THEN** their static prompt prefix is byte-identical
- **AND** thread-specific content appears after that prefix.

### Requirement: Context envelope enforces explicit budgets

The system SHALL assemble typed context sections under explicit per-stage and total
character ceilings, with mandatory authoritative state allocated before optional
history or references.

#### Scenario: Optional sections yield first

- **WHEN** candidate context exceeds the generation ceiling
- **THEN** lowest-priority optional sections are omitted or compacted first
- **AND** the envelope records each decision and observed/allowed size.

#### Scenario: Mandatory state never truncates silently

- **WHEN** actual request, required current source, params, authoring context, or repair
  diagnostic cannot fit within the ceiling
- **THEN** provider dispatch does not occur
- **AND** the user receives a raw context-budget error naming observed and allowed sizes.

#### Scenario: Classifier uses its own compact projection

- **WHEN** intent classification runs for an existing thread
- **THEN** its context remains within the classifier ceiling
- **AND** unrelated pinned references and full source are excluded by default.

### Requirement: Intent controls section relevance

The system SHALL deterministically project context for design, repair, question, and
classification stages without an extra summarization model call.

#### Scenario: Repair prioritizes diagnostic and source

- **WHEN** a repair request is assembled
- **THEN** exact current source, params, and latest raw diagnostic are included
- **AND** unrelated dialogue and references do not displace them.

#### Scenario: General question uses digest

- **WHEN** a question does not require source-level inspection
- **THEN** design/artifact digest is used instead of full source
- **AND** source-required questions still receive exact source.

### Requirement: Request-size telemetry is useful and content-free

The system SHALL record local context metrics without recording sensitive prompt
content.

#### Scenario: Envelope telemetry records shape, not data

- **WHEN** an API or MCP envelope is assembled
- **THEN** telemetry includes section ids, character counts, approximate tokens,
  inclusion decisions, total size, stage, and provider usage when available
- **AND** it excludes API keys, authorization headers, source, reference bodies, image
  bytes, prompt text, and full paths.

### Requirement: MCP discovery supports compact dynamic capabilities

The system SHALL expose session-scoped capability groups for managed compact clients,
emit standard tool-list change notifications after a group is enabled, and retain a
full compatibility profile for clients that need the complete catalogue.

#### Scenario: Compact session starts narrow

- **WHEN** a managed MCP session starts in compact profile
- **THEN** `tools/list` returns core workflow tools plus capability discovery controls
- **AND** specialist group schemas are absent until enabled.

#### Scenario: Specialist group loads on demand

- **WHEN** the agent enables a specialist capability group
- **THEN** the server updates that session's tool list
- **AND** emits `notifications/tools/list_changed`
- **AND** the next `tools/list` includes that group's schemas.

#### Scenario: Compatibility client can obtain full catalogue

- **WHEN** a client uses full compatibility profile
- **THEN** every enabled legacy tool remains discoverable
- **AND** standard cursor pagination is honored when the result spans pages.

### Requirement: MCP results are structured and bounded

The system SHALL return canonical machine data in `structuredContent`, concise text in
`content`, and continuation metadata for bounded partial reads.

#### Scenario: Ordinary result avoids pretty-printed duplication

- **WHEN** a tool returns structured JSON
- **THEN** the result exposes that JSON in `structuredContent`
- **AND** text content summarizes outcome and identity without repeating the full JSON.

#### Scenario: Large read reports truncation honestly

- **WHEN** a target, manifest, message, AST, or source read exceeds its response budget
- **THEN** the tool returns a bounded section/window
- **AND** reports observed count, returned count, total count, truncation, and a
  continuation cursor or exact next read.

#### Scenario: Full-target read remains explicit

- **WHEN** the agent calls the documented full-fidelity fallback
- **THEN** complete data remains available subject to transport safety limits
- **AND** the response reports its serialized size.

### Requirement: Screenshot bytes occur once

The system SHALL place screenshot bytes in one MCP image content item and SHALL keep
structured screenshot metadata byte-free.

#### Scenario: Screenshot result has no duplicate base64

- **WHEN** `get_model_screenshot` succeeds
- **THEN** image content contains the base64 payload once
- **AND** structured content contains identity, dimensions, camera, MIME type, source,
  and capture time without `dataUrl` or another base64 field.

### Requirement: Budgeting preserves raw errors

The system SHALL preserve raw provider/backend error body and diagnostic details when a
request fails; it SHALL NOT replace them with generic credential or retry advice.

#### Scenario: Provider rejection remains actionable

- **WHEN** a provider rejects an API request
- **THEN** UI-visible error details contain the raw provider body
- **AND** context telemetry reports sizes without logging that body.
