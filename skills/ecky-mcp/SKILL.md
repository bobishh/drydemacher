---
name: ecky-mcp
description: Author and edit Ecky CAD models over the Ecky MCP server. Use when connected to an Ecky workspace to inspect, append, preview, and verify .ecky models.
---

# Ecky MCP authoring

You are driving an Ecky CAD workspace through its MCP server. Ecky models are
written in **Ecky IR** — a small Scheme surface (`(model (part ...))`) that lowers
to a finite Core IR and renders on an exact OCCT B-rep kernel. You author the
surface; the kernel only ever sees the lowered Core IR.

The complete tool list, with arguments, is in [`reference/tools.md`](reference/tools.md)
— it is generated from the live server, so trust it over memory.

## The authoring loop: inspect → validate → publish

Always move in this order. Do not write history directly; every state change
flows through MCP.

1. **Inspect.** Call `workspace_overview` first — it resolves the default
   editable target, lists recent threads, and reports any conflicting lease.
   A targetless session borrows a chosen thread (`thread_borrow`) or creates one
   (`thread_create`) before editing. An Ecky provider session is already
   pre-bound: confirm `defaultTarget.threadId` and do not borrow its assigned
   thread again.
2. **Choose the source boundary.** Read `sourcePath` / `sourceState` from target
   metadata before editing.
   - When `sourcePath` exists, edit that file with normal file tools. The
     project-folder watcher appends, validates, and renders the settled edit.
     Do not call `macro_preview_render` for a full-source replacement.
   - When `sourcePath` is absent, validate and render a draft with
     `macro_preview_render`, which appends before validation and attaches its
     outcome to that version automatically.
   - Guarded `ecky_ast_*` patches are the intentional draft exception. Use
     `ecky_ast_patch_validate` → `ecky_ast_patch_preview` → verification →
     Bound-file refresh happens automatically after preview persistence.
3. **Validate and verify.** Check compilation/Core IR before rendering. For a
   rendered draft, call `verify_generated_model`; it updates that existing
   version with pass/fail evidence.
4. **Observe.** Wait for watcher sync on bound-file edits. Record returned
   `threadId`, `messageId`, and `modelId`. No commit/finalize command exists.

## Transpiling foreign CAD source

No separate MCP transpile tool exists. Send the foreign CAD text in a normal
thread message with an explicit request to translate it into parametric Ecky.
Treat that message as the recoverable source attachment: do not overwrite or
discard it while translation or verification is red.

Translate through the same authoring loop above. Add `(verify ...)` clauses for
the source invariants and any requirements accumulated in dialogue, preview the
complete `.ecky` model, call `verify_generated_model`, repair exact diagnostics
within the attempt cap. Verification attaches evidence to the already-appended
version automatically; no commit/finalize call exists. The
answer is a new Ecky version in that thread. Report capped red honestly; the
failed version remains in history.

## Rules

- **No direct SQLite writes.** During Ecky CAD modeling, never write
  `history.sqlite` or any app database directly. Route all model history,
  version, draft, and artifact state changes through MCP commands.
- **Prefer AST patches over full rewrites.** When an `ecky_ast_*` patch can
  express the change, use it instead of replacing the whole macro. Smaller diffs
  preserve stable node ids and selector bindings.
- **Verify red-to-green.** Treat each authored `verify` clause as an outer test:
  write it from the requirement, expect the first `verify_generated_model` run to
  go red, then fix the geometry or parameters and re-render until it goes green.
  Never weaken or delete a clause to force a pass.
- **Verification updates, never creates.** Call `verify_generated_model` on the
  preview/render draft. Green or red evidence attaches to that already-created
  version; no commit/finalize command exists.
- **Never promise STEP unless artifact truth proves it.** Read the
  `artifactBundle` (`hasStepExport`, `stepExportPath`) rather than assuming.
- **No junk threads.** Do not create throwaway `TMP` threads for debugging; fork
  or inspect an existing target, and clean up any noise you create.

## The language

For Ecky IR syntax and patterns — primitives, booleans, parameters, selectors,
fillets/shells, repetition, components, and verification — read the **Ecky IR
Field Guide** (`docs/books/ecky-ir/`), which builds up real models chapter by
chapter. Also available over MCP as the `ecky://guides/ecky-source` resource.
