## Context

Current API generation has two message objects but repeats policy:

- `design_system_prompt()` puts `TECHNICAL_SYSTEM_PROMPT` in system content.
- `format_contextual_prompt()` puts the same 4,948-character prompt inside user
  content as `EXECUTION RULES`.
- User content also includes full source, full params JSON, a 1,445-character framework
  contract on every generation, design/artifact digests, summary, six dialogue items,
  up to four 2,200-character references, and up to eight asset paths.
- No assembled request ceiling, section accounting, truncation report, or cache-prefix
  telemetry exists.
- `agent-prompt-single-source` already targets a 32,000-character language-reference
  ceiling, but generation and MCP resource wiring do not yet consume it.

Current MCP surface has related width problems:

- Tool definition source spans 79 top-level tools and about 82 KB of Rust JSON schema
  literals before serialization.
- `tools/list` ignores the standard cursor and returns the full enabled catalogue.
- `mcp_tool_success` pretty-serializes arbitrary JSON as text unless a handler builds a
  custom content envelope.
- `target_get` can return source, UI, artifact bundle, and model manifest together,
  despite compact split reads existing.
- Screenshot response carries identical bytes in content image data, structured
  `dataUrl`, and structured `base64`.

Official guidance supports narrower surfaces: MCP separates on-demand resources from
model-controlled tools, permits dynamic `tools/list` plus `list_changed`, structured
tool output, and pagination. OpenAI prompt caching requires an exact static prefix, and
tool search defers schemas until needed.

## Goals / Non-Goals

**Goals:**

- One policy/language copy per API request.
- Deterministic, inspectable context budgets without silent loss of authoritative state.
- Stable-prefix ordering for provider cache reuse.
- Small default MCP discovery and response projections with explicit escape hatches.
- Preserve raw provider/runtime errors and current source truth.

**Non-Goals:**

- Replace provider clients, change output JSON contract, or hide provider errors.
- Summarize source with an LLM.
- Depend on one provider tokenizer or one client's proprietary deferred-tool feature.
- Remove full-fidelity MCP reads; they become explicit fallbacks.
- Duplicate work owned by `agent-prompt-single-source`.

## Decisions

### 1. Typed context envelope before string formatting

Introduce internal `ContextEnvelope` / `ContextSection` values. Each section records:

- stable id and intent;
- priority: mandatory, authoritative, relevant, optional;
- char budget and measured char count;
- included, omitted, or truncated status plus reason;
- sensitivity class for telemetry redaction.

Only the final formatter creates provider text. This makes budget tests independent of
prompt prose.

Alternative: trim the final string. Rejected because boundaries and authority would be
lost.

### 2. Static system prefix, variable user tail

System content becomes:

1. output/behavior contract;
2. shared `agent_language_reference(backend)` language body;
3. applicable static framework rules.

User content contains current state and current ask once. Static content stays before
variable state so exact-prefix caching can work. Remove `EXECUTION RULES` duplicate.

Framework contract is included only for FreeCAD CAD-SDK authoring or a current macro
that uses the SDK. It is omitted for Ecky/build123d requests where irrelevant.

### 3. Mandatory-first dynamic budget

Default dynamic user-envelope ceiling: 64,000 characters, configurable in one backend
constant initially. Approximate token telemetry uses `ceil(chars / 4)` only as a metric;
enforcement remains deterministic Unicode character count.

Mandatory sections are never silently truncated:

- actual user request;
- current/target authoring context and migration policy;
- current source and current params when the intent must author or inspect source;
- current diagnostic for repair.

If mandatory sections exceed the ceiling, fail before provider dispatch with a raw,
structured context-budget error listing observed/allowed section sizes. Do not send a
lossy request.

Optional allocation order:

1. design/artifact digest;
2. relevant summary;
3. recent dialogue, maximum four × 200 characters;
4. relevant pinned references, maximum two × 1,200 characters;
5. relevant assets, maximum four metadata rows.

Trim lowest-priority optional sections first. Store no generated summary in SQLite from
this operation.

### 4. Intent projection

- Design edit: exact source/params plus relevant state.
- Repair: exact source/params and latest raw diagnostic; omit unrelated history.
- Question: digest by default. Include exact source only for deterministic source/code/
  parameter questions.
- Classifier: separate 8,000-character ceiling; prompt, design digest, latest dialogue,
  and frontend working snapshot only. Pinned references enter only when attachment or
  reference intent exists.

No new model call decides relevance; pure rules and existing intent signals do.

### 5. Safe local telemetry

Emit section ids, character counts, approximate tokens, inclusion decisions, total
size, provider-reported input/output/cache tokens, and request stage. Never log content,
source, reference bodies, image bytes, API keys, authorization headers, or full paths.
Surface detail through existing session activity/profiler mechanisms, not a new status
bar.

### 6. Session-scoped MCP capability groups

Keep tool names and handlers. Partition definitions into groups with concise summaries:

- core session/workspace;
- target reads;
- source/buffer edits;
- AST edits;
- semantic controls;
- verify/printability;
- components/library;
- project files.

Compact managed sessions start with core plus capability search/enable. Enabling a group
changes that session's list and emits standard `notifications/tools/list_changed`.
Groups target fewer than ten detailed tool schemas where practical. External/legacy
clients may use a full compatibility profile. Full profile also implements standard
cursor pagination; compact profile normally fits one page.

Alternative: paginate only. Rejected because clients that load every page still inject
the entire catalogue. Alternative: remove specialist tools. Rejected because capability
would be lost.

### 7. Canonical structured MCP results

For ordinary tools, `structuredContent` is canonical machine output. `content` carries a
short user/model summary plus continuation hints. Large reads require section/window/
limit input and return observed count, returned count, total count, truncation flag, and
continuation cursor. Full tools remain explicit and label returned size.

Screenshot output carries image bytes exactly once in the MCP image content item.
Structured content carries thread/message/model identity, dimensions, camera, MIME type,
and capture time only—no `dataUrl` or duplicate base64.

### 8. Error truth outranks width

Budgeting never replaces a raw provider/backend error with generic advice. Error text
remains bounded only by a generous transport cap; if cap is exceeded, return head/tail,
observed bytes, and a local diagnostic reference. UI continues showing raw body/details.

## Risks / Trade-offs

- [64K character ceiling mismatches provider tokens] → Keep enforcement deterministic,
  record provider usage, tune from measurements, and fail before lossy dispatch.
- [Compact MCP client ignores list-changed] → Use full compatibility profile for
  external clients; managed sessions are tested against dynamic discovery.
- [Structured content unsupported by old client] → Text summary remains useful and
  points to explicit detail tools; compatibility tests cover current clients.
- [Question projection omits source needed for answer] → Conservative deterministic
  source-required rules; user can request source detail; tests cover ambiguous cases.
- [Stable prefix changes by backend] → Cache is backend-specific by design; keep variable
  thread content out of system prefix.

## Migration Plan

1. Add failing envelope unit tests and provider payload snapshots.
2. Wire shared language builder, remove duplicate technical prompt, add telemetry.
3. Add MCP structured summaries and screenshot de-duplication.
4. Add capability groups/list-changed, then compact managed-session default.
5. Add pagination and full compatibility profile.
6. Run Rust tests, MCP protocol/e2e smoke, `cargo check`, and API raw-error proof.

No database migration. Any future UI-configurable budget/profile MUST persist through
`save_config`; initial constants avoid new settings shape.

## Open Questions

- Tune 64K dynamic and 8K classifier ceilings after capturing real p50/p95 section
  telemetry from representative large models.
- Which external MCP clients in supported matrix honor `tools/list_changed` reliably?

## References

- https://modelcontextprotocol.io/specification/2025-06-18/server/index
- https://modelcontextprotocol.io/specification/2025-06-18/server/tools
- https://modelcontextprotocol.io/specification/2025-06-18/schema
- https://developers.openai.com/api/docs/guides/prompt-caching
- https://developers.openai.com/api/docs/guides/tools-tool-search
