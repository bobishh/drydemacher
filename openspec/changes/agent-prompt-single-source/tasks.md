# Tasks: Single-Source Language Reference → Three Artifacts

## 1. Single source + shared builder

- [x] 1.1 Designate `public/docs/ecky-ir.md` + `surface-reference` as the single
  language source; document it in the book's revision notes and `design.md`.
- [x] 1.2 `agent_language_reference(backend)` builder — DONE. New module
  `src-tauri/src/agent_prompt.rs`: assembles `API_OPERATING_CONTRACT` +
  the canonical Markdown agent projection + injected op catalogue. API and MCP
  consume the same function.

## 2. API system prompt generation

- [x] 2.1 Store a marked, concise agent projection inside canonical Markdown.
  Human HTML/EPUB omits it; API and MCP consume it verbatim.
- [x] 2.2 Inject op catalogue from `surface-reference` — DONE, **as
  documentation-by-example** (per review: LLMs author better from a commented
  example than from prose, and it is terser). `op_catalogue()` emits a `scheme`
  code block of one real `.ecky` snippet per form (the entry's `example` field)
  with the `description` as a trailing `; comment`; restriction note only for
  backend-restricted ops. Never hand-written → cannot drift. Net effect: smaller
  AND more useful. The generated book index exposes the same registry.
- [x] 2.3 API operating contract — DONE. `API_OPERATING_CONTRACT` const: `.ecky`
  only, no tools, mm/deg, code→diagnostic→retry, respect per-op backend support.
- [x] 2.4 Budget guard — DONE. `AGENT_PROMPT_CHAR_CEILING = 32_000` (~8K tokens);
  test `agent_prompt_stays_within_budget` across all 3 backends.
- [x] 2.5 Emit from `generate:docs` (single pipeline with EPUB/HTML) into
  **committed** `.md` under `docs/` (reviewable in diffs; the EPUB stays
  gitignored).
- [x] 2.6 Golden-file/freshness test of generated backend prompt variants.
- [x] 2.7 CI freshness gate: fail if generated prompt differs from the
  committed copy (checked-in generated file cannot go stale).

## 3. Wire MCP + API to the shared builder

- [x] 3.1 Route `ecky://guides/technical-system-prompt` (and the language guide)
  through `agent_language_reference()` so MCP serves the same body.
- [x] 3.2 API mode consumes the shared prompt builder directly as system prompt.
- [x] 3.3 Assert MCP-served language body == API prompt language body (no fork).

## 4. Drift check

- [x] 4.1 Construct the prompt op catalogue only from `surface-reference`.
- [x] 4.2 Test: every `surface-reference` op appears in the prompt op table.
- [x] 4.3 Test: generated book appendix op index covers `surface-reference`.
- [x] 4.4 Wire drift tests into Rust/frontend suites so a new/renamed op appears in all
  three artifacts or the build fails.

## 5. Self-containment guard

- [x] 5.1 Test: the API prompt references no MCP tool name and contains no image
  markup (it must stand alone for a tool-less agent).
