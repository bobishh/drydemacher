## Why

API and MCP authoring paths expose enough state to be accurate, but they lack one
measured context envelope. The API repeats policy text and appends unbounded sections;
MCP exposes a large eager tool catalogue and some responses duplicate or return full
payloads, increasing latency, cost, and attention dilution.

## What Changes

- Build each API request from one authoritative, sectioned context envelope with
  explicit per-section and total budgets.
- Send policy/language guidance once as stable system-prefix content; keep variable
  thread state and the current ask after that prefix.
- Preserve exact current source when required for authoring, but budget summaries,
  dialogue, references, assets, parameters, and framework text by intent and relevance.
- Remove the duplicate `TECHNICAL_SYSTEM_PROMPT` copy from API user content.
- Coordinate with `agent-prompt-single-source`: consume its shared language builder
  instead of introducing another prompt copy.
- Emit local request telemetry: section characters/tokens, truncation decisions,
  provider usage, cached-input usage when returned, and total envelope size. Never log
  API keys, image bodies, private source, or raw reference content.
- Split MCP discovery into small capability groups/profiles and support paginated or
  deferred catalogue loading where the client permits it. Keep backward-compatible
  full discovery for clients that require standard `tools/list` behavior.
- Make compact MCP tools the normal path; keep full-target reads explicit fallbacks.
- Return structured MCP results with concise text summaries. Apply response budgets,
  pagination/section reads, and continuation metadata instead of silently dumping
  oversized JSON.
- Remove duplicate screenshot bytes from structured output; image bytes appear once,
  with metadata and identity references carried separately.
- Add regression tests for budget boundaries, authoritative-state retention, tool
  discovery compatibility, raw provider errors, and no accidental secret/content logs.

## Capabilities

### New Capabilities

- `agent-context-envelope`: Intent-aware API/MCP context assembly, budgets,
  projections, response shaping, and safe size telemetry.

### Modified Capabilities

<!-- None. Existing agent-language-reference work owns language-source generation;
     this change owns request-time assembly and MCP discovery/result width. -->

## Impact

- Backend: `context.rs`, `llm_context.rs`, generation request assembly, provider
  payload builders, MCP tool discovery, MCP success envelopes, screenshot results,
  and large read tools.
- Frontend: optional request-size diagnostics in existing session activity surfaces;
  no new agent status bar or raw terminal/log stream.
- Tests: Rust unit/integration coverage, MCP protocol compatibility, API payload
  snapshots, and raw-error propagation.
- Coordination: depends on or lands after the shared language builder wiring in
  `agent-prompt-single-source`.
