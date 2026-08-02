# Tasks: Campaign Project Learning Flow

## 1. Canonical content model

- [ ] Replace committed `manifest.json` with canonical `manifest.edn`.
- [ ] Add a Rust campaign-definition service using the existing strict data-only
  EDN parser; commit no JSON mirror and never evaluate campaign EDN.
- [ ] Package canonical Markdown, sources, and preview assets as Tauri resources.
- [ ] Add typed Tauri commands for summaries and one requested step payload.
- [ ] Remove frontend campaign corpus globs and frontend manifest parsing.
- [ ] Parse stable `{#step-id}` sections from mission Markdown.
- [ ] Reject missing, duplicate, orphaned, or empty step sections.
- [ ] Remove substantive `instruction` prose from manifest metadata after each
  Markdown section exists.
- [ ] Project the same Markdown sections into desktop Campaign payloads and static Chapters; static builders do not parse campaign EDN.

## 2. Campaign shell

- [ ] Render prose before source and actions; never duplicate header copy.
- [ ] Use a single prose column for explain steps.
- [ ] Keep source-step prose and editor in one bounded scrolling region.
- [ ] Count history only among steps preceding the active step.
- [ ] Remove autorender lifecycle and edit debounce paths.
- [ ] Keep explicit `RENDER` for edited code only.

## 3. Canonical and edited preview cache

- [ ] Define versioned campaign preview manifest keyed by canonical source and
  runtime digests.
- [ ] Generate required preview STL/bundle assets sequentially with a bounded
  worker; do not run corpus renders in parallel.
- [ ] Fail build/packaging when a source-bearing step lacks a fresh preview.
- [ ] Resolve bundled canonical preview in backend step payload without kernel execution.
- [ ] Query normal immutable cache before rendering edited source.
- [ ] Keep last good preview during pending/failure; show raw failure.

## 4. Draft and manifest migration

- [ ] Key draft overrides by step id plus canonical source digest.
- [ ] Ignore legacy plain-step drafts after canonical source changes.
- [ ] Add explicit step-id migration/fallback for removed steps.
- [ ] Preserve valid completion and challenge progress across prose-only edits.

## 5. Persistent project surface

- [ ] Add typed active Project navigation persistence through Tauri/DB commands.
- [ ] Restore active campaign run and step after reload.
- [ ] Restore Projects window visibility/rectangle through existing layout state.
- [ ] Fall back safely when saved run was deleted.
- [ ] Never create a thread, folder, config field, or direct SQLite write.

## 6. Mission rewrites

- [ ] Mission 1: bracket -> scaffold explanation -> body/lid -> joint coupon ->
  bounded enclosure branch challenge -> printable bracket finish.
- [ ] Mission 2: frame clamp -> named fit -> flat-roof failure -> supportless
  dovetail edit -> full bottle cage.
- [ ] Mission 3: station contract -> wing taper/twist -> repeated blade -> hub
  branch; label all models as geometry studies.
- [ ] Mission 4: shell -> retention coupon -> detents -> dovetail lid -> complete
  three-part razor kit.
- [ ] Mission 5: TPU envelope -> authored lattice -> PETG camera islands ->
  three-part case -> fit coupon.
- [ ] Mission 6: rail/channel fit -> format branch -> scanner subassembly ->
  helicoid coupon -> complete independently printable scanner.
- [ ] Recompute operation/alias coverage across mission sources; place uncovered
  forms into appropriate worked/challenge steps, never a detached dump chapter.

## 7. Proof

- [ ] Focused content assembly gate passes for all six missions.
- [ ] Focused Campaign component flow covers canonical preview, edited stale
  state, render pending, raw error, check failure, and check pass.
- [ ] One real-route reload restores Campaign and Projects window state.
- [ ] Preview manifest validation passes without invoking render.
- [ ] `cd src-tauri && cargo check` passes after persistence changes.
- [ ] `openspec validate campaign-project-learning-flow --strict` passes.
