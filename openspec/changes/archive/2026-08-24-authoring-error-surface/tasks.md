# Tasks: Self-Teaching Authoring Error Surface

BDD dual-loop. Failing test first (red), minimum code (green), refactor green.
Backend slices run `cd src-tauri && cargo test`; frontend slices run
`npm run test:unit` / targeted Playwright. Run the relevant suite after each
green, the full set before the final checkpoint.

## Composition invariants (hold across every slice)

- Conversion is one-way: `From<AuthoringError> for AppError` only. Never add a
  reverse `From<AppError> for AuthoringError`.
- The lowering/planning crate returns `Result<_, AuthoringError>` end-to-end.
- `AuthoringError` is internal: no `specta::Type`/serde. Only `AppError` crosses
  the boundary.
- `AppError.layer/fix` are `Option`; `None` means "not an authoring error" and
  is correct for persistence/provider/internal — do not retrofit those.

## 1. Error types + one-way conversion (foundation, not delegable)

Write scope:

- `src-tauri/src/contracts/error.rs`

Tasks:

- [x] 1.1 (red) Unit test: `From<AuthoringError>` maps layer→code per the table
  (Surface→Parse, CoreIr→Validation, Backend→Render) and carries op/span/fix.
- [x] 1.2 (red) Unit test: an `AppError` JSON payload omitting `layer`/`fix`
  deserializes with both absent (non-breaking boundary).
- [x] 1.3 (red) Unit test: `AppError` with a layer + fix serializes camelCase and
  round-trips.
- [x] 1.4 (green) Add `ErrorLayer`, `AuthoringReason`, `ErrorFix`,
  `AuthoringError` (layer/reason mandatory; internal, no serde), and
  `AppError.layer: Option<ErrorLayer>` + `AppError.fix: Option<ErrorFix>`
  (optional camelCase serde).
- [x] 1.5 (green) Implement `From<AuthoringError> for AppError` (the only
  conversion direction).
- [x] 1.6 (refactor) Layer-aware constructors on `AuthoringError`
  (`surface/core_ir/backend`) taking reason + optional fix.

## 2. Nearest-op suggester (delegable — isolated module)

Write scope:

- new `src-tauri/src/ecky_ir/op_suggest.rs` (+ `#[cfg(test)]`)

Tasks:

- [x] 2.1 (red) Unit test: a near-miss (`bx`) returns the nearest valid Core IR
  op (`box`) within threshold.
- [x] 2.2 (red) Unit test: a far-off name returns no suggestion.
- [x] 2.3 (green) Edit-distance nearest-op lookup over the Core IR op registry;
  pure function returning `Vec<String>`.
- [x] 2.4 (refactor) Source the op list from the existing registry, not a copy.

## 3. Lowering returns AuthoringError (integrative — keep on main thread)

Write scope:

- `src-tauri/src/ecky_cad_host/direct_occt.rs`
- `src-tauri/src/ecky_cad_host/direct_occt_executor.rs`
- `src-tauri/src/ecky_ir/*` lowering (unknown op / arity / type / parse sites)

Tasks:

- [x] 3.1 (red) Test: unknown op → `layer = CoreIr`, `reason = UnknownOp`, op
  named, nearest-op suggestion present.
- [x] 3.2 (red) Test: op unsupported by active backend → `layer = Backend`,
  `reason = Unsupported`, backend named, fix hint present.
- [x] 3.3 (red) Test: constrained-value site (axis) → `reason = ConstrainedValue`,
  fix lists the valid set.
- [x] 3.4 (red) Test: surface parse failure → `layer = Surface`, span set.
- [x] 3.5 (green) Convert lowering signatures to `Result<_, AuthoringError>`;
  resolve generic-`AppError` call sites by IO-hoist or explicit `.map_err`
  (never a reverse `From`); boundary returns `AppError` via the `From`.
- [x] 3.6 (refactor) Fold repeated layer/reason/fix construction into helpers.

## 4. Frontend renders layer + fix (delegable after slice 1 — disjoint from Rust)

Write scope:

- `src/lib/agents/draftFeedback.ts` (+ `.test.ts`)
- error bubble / code panel rendering

Tasks:

- [x] 4.1 (red) Unit test: presentation exposes a layer chip + fix line when the
  error carries them; falls back cleanly when absent.
- [x] 4.2 (green) Thread layer + fix through; render chip + fix line; keep the
  raw message visible.
- [x] 4.3 (red/green) Playwright: an authoring-error bubble shows the layer chip
  and a suggestion; raw message still present.

## 5. Retire the docs band-aid (delegable — fully isolated)

Write scope:

- `docs/books/ecky-ir/ecky-ir-corpus.md` (canonical) and generated projections
- `src/lib/docs/eckyIrContent.test.ts`, `src/lib/docs/eckyIrGuide.test.ts`

Tasks:

- [x] 5.1 (green) Trim the canonical field-guide preface to a one-line notation
  note; no up-front architecture section before the first Corner Bracket model.
- [x] 5.2 (green) Confirm the campaign projection begins with `Level 01: Corner
  Bracket` and the dry reference parser begins with `Operation Index`; canonical
  projections are regenerated with `npm run sync:book-source`.

## 6. Checkpoint

- [x] 6.1 `cd src-tauri && cargo test` green for new/affected modules.
- [x] 6.2 `npm run test:unit` green (including restored `eckyIrGuide`).
- [x] 6.3 Targeted Playwright green.
- [x] 6.4 `cd src-tauri && cargo check` clean.
