## 1. Outer Red: Provider And MCP Envelopes

- [ ] 1.1 Add API integration/payload snapshot test requiring one technical-policy copy, stable system prefix, variable user tail, and total-size metadata; run it and confirm duplicate prompt failure.
- [ ] 1.2 Add context-overflow integration case requiring no provider dispatch and raw observed/allowed section sizes; confirm current unbounded dispatch fails it.
- [ ] 1.3 Add MCP protocol test for compact managed tool discovery, capability enable, `tools/list_changed`, and full compatibility pagination; confirm current 79-tool eager list fails it.
- [ ] 1.4 Add MCP screenshot response test requiring one image byte payload and byte-free structured metadata; confirm current triple duplication.
- [ ] 1.5 Add large-result MCP test requiring structured content, concise text, truncation counts, and continuation metadata; confirm current pretty-printed full JSON failure.

## 2. Inner Loop: Typed Context Envelope

- [ ] 2.1 Add failing unit tests for section priorities, Unicode character accounting, optional eviction order, and mandatory-overflow error.
- [ ] 2.2 Implement internal `ContextEnvelope`, `ContextSection`, inclusion decision, sensitivity, and measurement types with camelCase only on serialized boundaries.
- [ ] 2.3 Add failing unit tests for design, repair, question, source-required question, and classifier projections.
- [ ] 2.4 Implement deterministic intent projection with 64K generation and 8K classifier defaults; no LLM summarization call.
- [ ] 2.5 Add unit tests proving exact current source/params/diagnostic are never silently truncated.
- [ ] 2.6 Refactor legacy summary/dialogue/reference/asset formatting behind the envelope while tests remain green.

## 3. API Assembly And Stable Prefix

- [ ] 3.1 Complete or depend on `agent-prompt-single-source` shared language-builder wiring; add test preventing another language copy.
- [ ] 3.2 Remove `TECHNICAL_SYSTEM_PROMPT` from contextual user content and assemble output contract/language/framework once in system prefix.
- [ ] 3.3 Include CAD framework contract only for applicable FreeCAD/CAD-SDK contexts.
- [ ] 3.4 Route generation and classification through their typed envelopes; preserve raw provider error body/details.
- [ ] 3.5 Re-run API payload and overflow integration tests; refactor only with green snapshots.

## 4. Safe Context Telemetry

- [ ] 4.1 Add failing tests that telemetry contains section ids/counts/decisions/usage but no prompt, source, references, image bytes, API keys, headers, or full paths.
- [ ] 4.2 Emit envelope and provider cache/input/output usage through existing profiler/session activity path without adding a status bar.
- [ ] 4.3 Add p50/p95 audit fixture/report for representative empty, normal, repair, and large-source turns; use evidence to confirm or revise default ceilings.

## 5. MCP Capability Discovery

- [ ] 5.1 Extract tool definitions into typed capability groups; add drift test proving every dispatched tool belongs to exactly one group.
- [ ] 5.2 Add capability search/enable controls and session-scoped enabled-group state.
- [ ] 5.3 Advertise `listChanged`, emit `notifications/tools/list_changed`, and return core plus enabled groups for compact managed sessions.
- [ ] 5.4 Implement full compatibility profile and standard cursor pagination without renaming existing tools.
- [ ] 5.5 Update managed-agent startup guidance and generated MCP skill to prefer compact discovery and explicit detail reads; run skill drift check.

## 6. MCP Result Projection

- [ ] 6.1 Change ordinary success envelopes to canonical `structuredContent` plus concise text summary; add compatibility tests.
- [ ] 6.2 Add shared response-budget/continuation metadata for source, messages, AST, manifest, and target section reads.
- [ ] 6.3 Keep `target_get` and other full reads explicit; report serialized size and transport-limit failure honestly.
- [ ] 6.4 Remove screenshot `dataUrl` and duplicate `base64` from structured content while preserving identity, dimensions, camera, MIME type, source, and capture time.
- [ ] 6.5 Verify tool-origin errors use MCP `isError` with raw actionable details and never generic credential advice.

## 7. Integration Green And Proof

- [ ] 7.1 Run targeted context, generation, LLM payload, MCP server, handler, and screenshot Rust tests after each green step.
- [ ] 7.2 Run API dialogue Playwright happy path plus provider-error/pending state; assert raw body remains visible.
- [ ] 7.3 Run MCP inspect → validate → preview → verify → commit smoke with compact discovery; confirm no direct SQLite writes or throwaway threads.
- [ ] 7.4 Run `npm run typecheck`, relevant frontend unit tests, `cd src-tauri && cargo test`, then mandatory `cd src-tauri && cargo check`.
- [ ] 7.5 Run `npm run check:skill` and OpenSpec strict validation; do not stage or commit unless user asks.
