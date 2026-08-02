## 1. Contract

- [x] 1.1 Record documentation hard gate, artifact model, variables, decisions,
  rejected paths, and proof plan.
- [x] 1.2 Strict-validate `repair-ecky-learning-campaign`.
- [x] 1.3 Record current targeted baseline without changing production code.
  - Baseline: docs mission/manifest/evaluator/progress/lessons unit tests → 68 pass.
  - Confirmed defects in source:
    - `EckyMissionWorkbench.svelte` renders ALL six phase sections (brief/study/decide/practice/transfer/debrief) at once; phase nav calls `scrollIntoView` only.
    - Level 01 `workedExample.source` === `solution.source` (identical union of two boxes).
    - Level 03 `workedExample.source` === `solution.source` (identical repeat-union).
    - `DocsSite.svelte` renders `activeSection.bodyHtml` (full campaign chapter incl. the answer) below the workbench.
    - EPUB header button label is `DOWNLOAD CAMPAIGN`.
    - No canonical Corner Bracket render asset exists; Level 01 BRIEF shows no image.

## 2. Slice One: Real phase navigation and pedagogy separation

- [x] 2.0 Remove speculative future-slice tests from
  `e2e/ecky-learning-campaign-repair.spec.ts`. Keep one current route RED only; do not
  test CAD model functionality already covered by Ecky runtime tests.
  - Removed Slice Two/Three/Four describe blocks; retained Slice One (2.1) +
    no-duplicate-chapter boundary only. CAD-model assertions left to Ecky runtime.
- [x] 2.1 Add one Playwright RED: BRIEF is the only visible phase on first load;
  clicking STUDY replaces BRIEF and shows one worked example once.
- [x] 2.2 Add content RED: every mission has materially different worked-example and
  solution source; Level 01 brief does not reveal its construction recipe.
  - `src/lib/docs/eckyLearningCampaignQuality.test.ts`: 2 RED (worked/solution differ for 5/6 missions).
- [x] 2.3 Confirm failures match current all-phases/duplicate-answer defects.
  - RED matches baseline: workbench renders all phases; Level 01/02/03/04/06 workedExample === solution.
- [x] 2.4 Implement one-active-phase shell; preserve hidden practice state.
  - `EckyMissionWorkbench.svelte` mounts one `activePhase` at a time; PRACTICE
    attempt/decision/hint/reveal state lives on the component and survives unmount.
- [x] 2.5 Rewrite Level 01 worked example to an analogous non-bracket artifact.
  - Level 01 `workedExample.source` is a pin-boss (box+cylinder union), materially
    distinct from the bracket solution; briefImage bound to canonical render.
- [x] 2.6 Remove duplicate STUDY markup and simultaneous chapter answer from campaign.
  - `DocsSite.svelte` collapses chapter prose into a `<details>` "Lesson notes";
    `.docs-article__body` is absent beneath the workbench in campaign mode.
- [x] 2.7 Re-run slice tests to green.
  - Slice One e2e (4) + content (Level 01 portion) green.

## 3. Slice Two: Real Corner Bracket asset

- [x] 3.1 After Slice One is green, add one Playwright RED for descriptive Corner
  Bracket image and non-zero
  `naturalWidth`/`naturalHeight`.
- [x] 3.2 Add canonical Corner Bracket `.ecky` source and native render check.
  - `docs/books/ecky-ir/examples/corner-bracket.ecky`; `eckyMissionAssets.test.ts`
    binds PNG to source sha256 + native contentHash.
- [x] 3.3 Add deterministic native-Ecky → STL → Three/WebGL browser → PNG pipeline.
  - `scripts/render_ecky_preview_png.mjs`: native direct-occt → STL → headless
    chromium + Three/WebGL (SwiftShader) → PNG + sidecar manifest. No OpenSCAD.
- [x] 3.4 Publish source-bound asset through book/campaign build; never invoke
  OpenSCAD.
  - `npm run build:mission-assets`; asset at `public/docs/assets/corner-bracket.png`.
- [x] 3.5 Visually inspect actual mesh framing; run image and source-drift tests green.
  - Coverage metric proves non-blank mesh (Corner Bracket 14.61%, Dovetail 7.92%);
    asset + content tests green.

## 4. Slice Three: Honest EPUB action

- [x] 4.1 After Slice Two is green, add one Playwright RED: campaign exposes secondary
  `OFFLINE BOOK · EPUB` and no `DOWNLOAD CAMPAIGN`.
- [x] 4.2 Implement explicit format copy/style while preserving exact EPUB download.
  - `DocsSite.svelte` header action `OFFLINE BOOK · EPUB` with `docs-action--secondary`.
- [x] 4.3 Run pending/error and successful download tests green.
  - Visibility (secondary class, no `DOWNLOAD CAMPAIGN`) + EPUB download e2e green.

## 5. Slice Four: Dovetail Fit

- [x] 5.1 After Slice Three is green, add one Playwright RED replacing Level 03 ribbed
  plate with Dovetail Fit on the campaign route.
- [x] 5.2 Extract the smallest teachable male/female dovetail fixture from
  `docs/books/ecky-ir/examples/ecky-integrated-film-adapter-open-helicoid-v9.ecky`.
  Preserve production profile/fit math; do not create a second dovetail design.
  - `docs/books/ecky-ir/examples/dovetail-fit.ecky` reuses the film-adapter
    `rail_profile_pos` triangular profile and `(+ nominal (* 2 fit_clearance))`
    enlargement; surrounding mechanism dropped.
- [x] 5.3 Represent mating fit with one named `fit_clearance` binding used by both
  sides; reject magic fit offsets in tests.
  - male = nominal profile; female = profile enlarged by `(* 2 fit_clearance)`;
    practice starter ships a hard-coded `0.6` magic offset to be replaced.
- [x] 5.4 Keep preview assembly placement outside production export geometry.
  - two clean `(part ...)` exportable solids; side-by-side layout is the fixture's
    authored geometry, not a diagnostic assembly transform.
- [x] 5.5 Author distinct worked example, repair practice, progressive hints, solution,
  and transfer without exposing answer early.
  - worked example = analogous pin/hole clearance; hints progressive (no full
    solution); solution + reasoning + alternatives authored.
- [x] 5.6 Validate the extracted fixture once through MCP or native Ecky, then generate
  real STL/PNG artifacts through Three/WebGL. Do not add duplicate CAD-operation tests.
  - `ecky check` (2 parts) + `ecky render --backend direct-occt` validated once;
    PNG via the Three/WebGL pipeline; no CAD-operation tests added.
- [x] 5.7 Move rib/repeat example to optional reference notes; update campaign order,
  manifest, projections, EPUB, and tests.
  - rib section retained in corpus as reference (unmapped); Level 03 remapped to
    `## Physical Fit: Dovetail Rail and Channel`; manifest/ids/slug/lessons test/
    EPUB regenerated.
- [x] 5.8 Turn the single Level 03 route RED green: production-derived Dovetail title,
  named-clearance teaching, and non-broken asset. Existing generic practice tests own
  pending/wrong/pass behavior.
  - Level 03 = Dovetail Fit, ribbed plate absent, practice exposes `fit_clearance`,
    non-broken Dovetail image; route e2e green.

## 6. Bounded remaining-level audit

- [x] 6.1 Add invariant test for Levels 02, 04, 05, 06: worked example differs from
  solution, brief avoids recipe, hints avoid exact edits.
  - global tests: worked≠solution, hints≠full-solution, and new
    "no mission brief embeds its worked-example or solution source".
- [x] 6.2 Repair only failing content; do not redesign passing missions.
  - Level 02 → sleeve (analogous difference); Level 04 → 1D peg-bar (analogous
    procedural); Level 06 → shaft/sleeve (analogous fit). Concepts preserved.
- [x] 6.3 Re-run all mission/content tests green.
  - 80/80 focused docs unit tests; 414/414 full `npm run test:unit`.

## 7. Verification

- [x] 7.1 Run targeted Playwright on alternate port.
  - campaign spec on port 4243 (desktop), 9/9 green.
- [x] 7.2 Run mission/content unit tests, then full `npm run test:unit`.
  - full `npm run test:unit` → 414 pass / 0 fail.
- [x] 7.3 Run `npm run typecheck`, source drift checks, book build, and app build.
  - `npm run typecheck` 0 errors (1 pre-existing unrelated CSS warning);
    `npm run build:book` regenerates EPUB/HTML/assets; content + mission sync current.
- [x] 7.4 Audit the full e2e inventory, run the campaign suite, and confirm campaign
  coverage contains no redundant CAD-model assertions.
  - campaign spec audited: zero CAD-operation assertions (no polygon/extrude/
    difference/rail-validity/watertight checks). Campaign coverage (desktop + 390px)
    is the proportional browser proof; unrelated application suites retain their own
    change-level gates.
- [x] 7.5 Browser proof at desktop and 390px: one phase, real images, pending/wrong,
  passing solution, EPUB CTA, no duplicate chapter, no overflow/errors.
  - desktop 9/9; 390px: one phase, real images, EPUB CTA, no `.docs-article__body`,
    0px horizontal overflow, no page/console errors, Level 03 fit_clearance.
- [x] 7.6 Run `cd src-tauri && cargo check`.
  - `cargo check` exit 0.
- [x] 7.7 Run strict OpenSpec validation and `git diff --check`.
  - `openspec validate repair-ecky-learning-campaign` → valid; `git diff --check` clean.
- [x] 7.8 Do not stage or commit; parent agent performs reviewed source-control work.
  - nothing staged or committed.
