## 1. OpenSpec And Baseline

- [x] 1.1 Document content source, file locations, serving path, frontend-shell
  responsibility, variables, decisions, rejected paths, and proof plan.
- [x] 1.2 Compare the instructional loop with worked examples, faded completion,
  Parsons-style constrained practice, and retrieval/transfer research.
- [x] 1.3 Validate with
  `openspec validate interactive-ecky-learning-missions --strict`.
- [x] 1.4 Record existing campaign/source checks and targeted docs tests before edits.
  - Baseline: `npm run check:book-source` passes (sources current).
  - Baseline: `npm run test:unit` passes (314 tests).
  - Baseline: targeted docs content tests pass (eckyIrContent, eckyIrSource, eckyIrWorkedExamples, eckyIrGuide, eckyIrBook).

## 2. Outer Red: Level 01 Mission

- [x] 2.1 Add Playwright scenario proving Level 01 renders Corner Bracket with
  BRIEF/STUDY/DECIDE/PRACTICE/TRANSFER, pending state, and no ball-on-base content.
- [x] 2.2 Run it and confirm failure because current campaign has no interactive
  workbench and still renders Marker/ball-on-base.
- [x] 2.3 Add Playwright wrong-attempt scenario requiring criterion-specific
  connection/placement feedback and absence of compile/printability claims.
- [x] 2.4 Add Playwright passing-attempt scenario requiring complete structural state
  and solution/debrief availability.

  - Red evidence: added three scenarios under `test.describe('interactive-ecky-
    learning-missions: Level 01 mission (outer red)')` in `e2e/docs-site.spec.ts`.
    Ran `PLAYWRIGHT_WEB_PORT=4244 npx playwright test e2e/docs-site.spec.ts -g
    "Level 01"` (defaults 5173/4243/8787 occupied). All three fail RED for the
    intended reason — no interactive mission workbench: Test 1 times out at
    `getByRole('button', { name: /^BRIEF$/ })`; Tests 2 and 3 time out at the attempt
    `getByRole('textbox', { name: /attempt/i })`. The preceding `Corner Bracket`
    heading assertion in Test 1 passes first, so the red is the missing workbench,
    not the chapter title. The four pre-existing docs-site scenarios stay green
    (`--grep-invert "outer red"` -> 4 passed). Note: the "still renders
    Marker/ball-on-base" clause in 2.2 is now stale because a concurrent Section-3
    change already replaced Level 01 with Corner Bracket in the corpus/campaign, so
    no ball-on-base content remains; the real, confirmed red reason is the absent
    interactive workbench. No production code was changed to obtain this red.

## 3. Inner Loop: Canonical Content And Publication

- [x] 3.1 Add failing content-projection test for renamed Level 01 Corner Bracket and
  absence of ball-on-base teaching content.
- [x] 3.2 Update canonical book corpus; regenerate campaign, split levels, EPUB inputs,
  and render assets through existing book scripts.
- [x] 3.3 Add failing tests for canonical mission index, six ordered mission files,
  section-slug mapping, schema version, and required worked/practice/solution fields.
- [x] 3.4 Implement canonical mission files under `docs/books/ecky-ir/missions/`.
- [x] 3.5 Add normalized publication to
  `public/tutorials/ecky-missions.json`; extend source drift checks.
- [x] 3.6 Run `npm run check:book-source` and relevant content/book tests.

  - 3.1 evidence: added `campaign Level 01 projects as Corner Bracket with no
    ball-on-base teaching content` to `src/lib/docs/eckyIrContent.test.ts`
    (asserts `## Level 01: Corner Bracket`, `(part bracket`, `(union`, and the
    absence of `First Solid: Ball on a Base` / `ball on a base` / `(part marker`
    in corpus and campaign). The rename is authoritative newer work from the
    prior agent, so this confirms green. Also updated two stale projections left
    red by the rename: `Level 01: Marker` -> `Level 01: Corner Bracket` in
    `eckyIrBook.test.ts` and `eckyIrGuide.test.ts`, and the book's first-asset
    assertion to `assets/04-cut-and-join-01.png` because the Corner Bracket
    chapter is intentionally imageless (no Ecky-runtime render regen is in this
    content/publication slice).
  - 3.2 evidence: the canonical corpus already ships `## First Solid: Corner
    Bracket` (prior agent, authoritative). `npm run check:book-source` reports
    `Ecky content sources are current.`, so campaign, split levels, and EPUB
    inputs are in sync with the corpus; `public/docs/ecky-ir-field-guide.epub`
    and `.html` were already rebuilt. No corpus edit was required or made.
  - 3.3 evidence: added `src/lib/docs/eckyIrMissions.test.ts`. Confirmed RED
    first (only level-01/02 files existed): `level-03-parametric-pattern.json
    must exist` failures. Tests assert numeric schemaVersion, six ordered unique
    ids, one-to-one sectionSlug <-> campaign section order, required
    worked/decision/practice/solution/handoff fields, and the bounded criterion
    vocabulary.
  - 3.4 evidence: implemented `level-03-parametric-pattern.json` (repair:
    copied ribs -> `repeat-union`), `level-04-procedural-workshop.json`
    (completion: `map`/`range`/`apply union` + final `difference`),
    `level-05-toothbrush-holder.json` (completion: proven spade cutter ->
    `repeat-union` group + final cut), and `level-06-film-adapter.json`
    (repair: shared `fit_clearance` binding across mating sides). Honesty check:
    every solution passes all its structural criteria; every starter fails at
    least one discriminating form-based criterion.
  - 3.5 evidence: added `scripts/ecky_ir_missions.ts` (`projectEckyMissions` +
    `syncEckyMissions` with exact validation errors for duplicate ids, missing
    solutions/alternatives, unsupported criteria, and mismatched schema), wired
    into `scripts/sync_ecky_ir_sources.ts` and `scripts/build_ecky_ir_book.ts`,
    and generated `public/tutorials/ecky-missions.json` via
    `npm run sync:book-source`. Added projection + drift tests asserting the
    bundle is camelCase and equals the canonical projection.
  - 3.6 evidence: `npm run check:book-source` is current (now covers the mission
    bundle drift). `npx tsx --test` on `eckyIrContent`, `eckyIrSource`,
    `eckyIrWorkedExamples`, `eckyIrBook`, `eckyIrGuide`, `eckyIrMissions` ->
    23/23 pass; `eckyIrDocsSite` -> 6/6 pass.

## 4. Inner Loop: Manifest And Structural Evaluator

- [x] 4.1 Add failing unit tests for valid manifest parsing and raw validation errors
  for duplicate ids, unknown section slugs, unsupported criteria, missing solutions,
  and missing alternatives.
- [x] 4.2 Implement typed manifest validation and normalization without a new runtime
  dependency.
- [x] 4.3 Add failing evaluator tests for `containsForm`, `containsSymbol`,
  `excludesForm`, `minFormCount`, `balancedDelimiters`, and `removedPlaceholder`.
- [x] 4.4 Implement an Ecky-aware structural scanner that ignores forms inside strings
  and comments; return every criterion result and exact feedback.
- [x] 4.5 Add failing tests proving results never claim compile, geometry, printability,
  watertightness, or export success.
- [x] 4.6 Refactor evaluator while targeted tests remain green.

  - 4.1 evidence: added `src/lib/docs/eckyMissionManifest.test.ts`. Confirmed RED
    first (`ERR_MODULE_NOT_FOUND`, module did not exist). Tests assert the six
    bounded criterion kinds, valid-bundle normalization preserving order/ids, the
    real published bundle normalizing against the real campaign section slugs, and
    exact issue codes for duplicate ids (`duplicate-id`), unknown section slugs
    (`unknown-section-slug`), unsupported criteria (`unsupported-criterion`),
    missing solutions (`missing-solution`), and missing alternatives
    (`missing-alternatives`), plus multi-issue collection and graceful
    invalid-shape fallback.
  - 4.2 evidence: implemented `src/lib/docs/eckyMissionManifest.ts` exporting
    `MISSION_CRITERION_KINDS`, the typed mission/criterion/bundle interfaces, and
    `normalizeMissionBundle(raw, knownSectionSlugs)` returning
    `{ ok: true; manifest } | { ok: false; issues }` with exact `ManifestIssue`
    codes/messages. No new runtime dependency (pure TS, stdlib only). The runtime
    validator is intentionally separate from the publication validator in
    `scripts/ecky_ir_missions.ts`, which owns build-time source drift.
  - 4.3 evidence: added `src/lib/docs/eckyMissionEvaluator.test.ts`. Confirmed RED
    first. Tests cover every criterion kind: `containsForm` (present/absent/
    comment-ignored/string-ignored/multi-token whitespace/token boundary),
    `containsSymbol` (real token vs comment/string), `excludesForm` (absent/
    comment-only passes, real-code fails), `minFormCount` (threshold +
    comment/string ignored), `balancedDelimiters` (balanced, unbalanced open,
    closing-before-opening, comment/string parens ignored), and
    `removedPlaceholder` (fails while present, passes once removed, and honestly
    fails when the placeholder survives only in a comment). Also asserts results
    return in input order with exact authored pass/fail feedback.
  - 4.4 evidence: implemented `src/lib/docs/eckyMissionEvaluator.ts` with
    `evaluateAttempt(criteria, source)` returning every `CriterionResult` plus a
    `StructuralEvaluation` status/summary. The scanner
    (`stripCommentsAndStrings`) mirrors the canonical Ecky lexer in
    `src-tauri/src/ecky_ir/syntax.rs::strip_comments`: `;` line comments and
    `"..."` string literals with backslash escapes are removed before form/
    symbol/delimiter matching, so quoted or annotated shapes cannot satisfy a
    check. `removedPlaceholder` is the explicit exception: it scans raw source
    (including comments) because the placeholder is an author marker the learner
    must physically delete.
  - 4.5 evidence: added a forbidden-claim guard in the evaluator test file. The
    evaluator-synthesized `summary` and `status` are scanned for `compil`,
    `geometry`, `printab`, `watertight`, `export`, and `valid solid` across a
    synthetic pass, a synthetic fail, and every real mission solution and
    starter. `status` is bounded to `structural-pass` / `structural-fail`.
    Authored criterion feedback is content and is returned verbatim, so the
    guard applies only to evaluator-generated fields.
  - 4.6 evidence: refactored `evaluateAttempt` to share one `ScanContext`
    (`raw` / `code` / `codeTokens`) across all criteria instead of passing four
    parameters; ran a focused strict `tsc --noEmit` (clean, exit 0) and re-ran
    the targeted tests.
  - Section 4 proof: `npx tsx --test src/lib/docs/eckyMissionManifest.test.ts
    src/lib/docs/eckyMissionEvaluator.test.ts` -> 34/34 pass (10 manifest + 24
    evaluator). Real-content sanity (not committed as a test) confirms every
    published solution evaluates `structural-pass N/N` and every starter
    evaluates `structural-fail` with at least one discriminating failure.
    `npm run check:book-source` -> `Ecky content sources are current.`;
    `openspec validate interactive-ecky-learning-missions --strict` -> valid.
    No frontend/workbench, Rust, Tauri, config, or DB code was touched.

## 5. Inner Loop: Progress

- [x] 5.1 Add failing tests for versioned progress serialize/restore, unknown mission
  filtering, corrupt payload fallback, and one-mission reset.
- [x] 5.2 Implement bounded `eckyLearningProgress:v1` storage adapter.
- [x] 5.3 Ensure mission progress does not use app configuration or Tauri
  `save_config`.

  - 5.1 evidence: added `src/lib/docs/eckyMissionProgress.test.ts`. Confirmed RED
    first (`ERR_MODULE_NOT_FOUND`, module did not exist). The 20 tests cover every
    required behavior: the versioned `eckyLearningProgress:v1` key/version;
    serialize/deserialize round-trip and a parseable versioned object payload;
    full restore of attempt source, decision, hint depth, criterion state, and
    solution reveal through an injected storage; unknown-mission filtering on
    load; corrupt-payload fallback for non-JSON, incompatible version, missing
    entries, non-object payload, throwing storage access, empty store, a single
    malformed entry (sibling kept), and a non-finite hint depth; bounded storage
    on save (only known ids persisted, unknown id rejected, retired mission
    dropped across reloads); and reset of one active mission leaving other
    missions intact (plus the no-op reset case).
  - 5.2 evidence: implemented `src/lib/docs/eckyMissionProgress.ts`, a pure,
    dependency-free module exporting `LEARNING_PROGRESS_KEY`
    (`'eckyLearningProgress:v1'`), `LEARNING_PROGRESS_VERSION`, the
    `LearningStorage` interface, `MissionProgressEntry` / `LearningProgressRecord`
    types, and `emptyProgress` / `serializeProgress` / `deserializeProgress` /
    `loadProgress` / `saveProgress` / `saveMissionProgress` /
    `resetMissionProgress`. Bounded storage is enforced symmetrically: both load
    and save run `boundToKnownMissions(knownMissionIds)`, so the persisted set can
    never exceed the active campaign. Corrupt or incompatible records return clean
    defaults via `deserializeProgress` (returns `null`) without throwing; per-entry
    corruption is tolerated so one bad entry cannot wipe the rest.
  - 5.3 evidence: the adapter persists only through the injected `LearningStorage`
    abstraction (`getItem`/`setItem`/`removeItem`, the browser `localStorage`
    shape) under the single `eckyLearningProgress:v1` key. It imports nothing from
    `@tauri-apps/*`, never calls `save_config`, never reads/writes
    `app_config_dir/config.edn`, and never opens a database. The isolation test
    injects an in-memory storage and proves the only read and write key ever
    touched is the versioned progress key.
  - Section 5 proof: `npx tsx --test src/lib/docs/eckyMissionProgress.test.ts` ->
    20/20 pass. `npm run check:book-source` -> `Ecky content sources are current.`
    `openspec validate interactive-ecky-learning-missions --strict` -> valid. No
    evaluator, manifest, Rust, Tauri, config, DB, or frontend-workbench file was
    touched; footprint is exactly two new files under `src/lib/docs/`. No
    source-control staging or commit was performed.

## 6. Inner Loop: Mission Workbench

- [x] 6.1 Implement fetch/loading/error states and active-section mission mapping in
  `DocsSite.svelte`.
- [x] 6.2 Build `EckyMissionWorkbench.svelte` as shell only: phase navigation,
  decision options, attempt editor, structural feedback, hint ladder, transfer prompt,
  solution override/reveal, reasoning, and alternatives.
- [x] 6.3 Apply Tactical Midnight tokens, square borders, visible focus, and
  `overflow: hidden` to every major workbench container.
- [x] 6.4 Add accessible live feedback, native buttons, labels, keyboard flow, and
  explicit internal scrolling.
- [x] 6.5 Add exact `OPEN ATTEMPT IN CODE` handoff when callback exists; otherwise
  expose exact copy and `.ecky` download.
- [x] 6.6 Re-run Level 01 outer tests to green; refactor only under passing coverage.

  - 6.1 evidence: `src/lib/DocsSite.svelte` now owns the campaign mission
    manifest path independently of the chapter markdown. Added
    `missionBundle`/`missionLoading`/`missionError` state; `loadMissions()`
    fetches `/tutorials/ecky-missions.json` with `cache: 'no-store'`, runs the
    runtime `normalizeMissionBundle(raw, knownSectionSlugs)` against the parsed
    campaign section slugs, and on failure surfaces a raw
    `Mission manifest invalid: <code>: <message> · …` error while the chapter
    stays readable. `activeMission` is `$derived` by matching
    `mission.sectionSlug === activeSection.slug`; `knownMissionIds` is derived
    from the bundle for progress bounding. The fetch is fire-and-forget after
    the chapter parses so the article renders immediately. Three render states
    mount inside `docs-article` above the chapter: a `.docs-mission-error`
    alert, a `.docs-mission-loading` status, or the workbench keyed by
    `{#key activeMission.id}` so it remounts on section change.
  - 6.2 evidence: added `src/lib/EckyMissionWorkbench.svelte`, a shell that owns
    no content (every briefing, worked example, decision option, criterion,
    hint, solution, reasoning, and alternative is the file-backed `mission`
    prop). It renders phase navigation buttons BRIEF/STUDY/DECIDE/PRACTICE/
    TRANSFER (anchored, scroll-to-phase nav), a DECIDE group with per-option
    `aria-pressed` and consequence feedback (accepted vs weak styling), a
    PRACTICE attempt editor (`<textarea aria-label="Attempt">` bound to
    `attemptSource`), CHECK STRUCTURE driving the pure evaluator into a
    `data-testid="mission-structure-feedback"` live region, a one-at-a-time
    hint ladder (REVEAL HINT n/total), the TRANSFER prompt, and a gated
    DEBRIEF: SHOW WORKED SOLUTION reveals directly after a check, while a
    pre-check activation only arms SHOW SOLUTION ANYWAY and exposes no source.
    Revealed solution renders source, ordered reasoning, the
    native-proof-limitation note, and named alternatives with trade-offs. A
    two-step RESET MISSION clears only the active mission's progress.
  - 6.3 evidence: styles reuse `--secondary`, `--primary`, `--red`, `--bg-300`,
    `--text`/`--text-dim`, and the existing Tactical Midnight grid background;
    no `border-radius` is declared anywhere (app.css already forces 0 on form
    controls, divs/pre use square borders). `:focus-visible` outlines are on
    the textarea and every action button. Every major container
    (`.mission-workbench`, `.mission-workbench__header`, each `.mission-phase`,
    `.mission-feedback`, `.mission-hints`, `.mission-editor`, `.mission-options`,
    `.mission-reveal`, `.mission-solution`, `.mission-reset`) declares
    `overflow: hidden`; the editor textarea, feedback list, code blocks, and
    hint list own explicit `max-height` + `overflow: auto`.
  - 6.4 evidence: feedback is a `role="status" aria-live="polite"
    aria-atomic="true"` region; the attempt editor has a visible `<label>` +
    `aria-describedby` help; options are a `role="group"` of native buttons with
    `aria-pressed`; the phase nav uses `aria-current="step"`; the workbench root
    and each phase carry `aria-label`/`aria-labelledby`. All controls are native
    `<button>`/`<textarea>` so the keyboard tab/activation flow is default.
  - 6.5 evidence: when `onOpenAttemptInCode` is supplied (the in-app
    DocsSite mount passes `onOpenSnippet`), an `OPEN ATTEMPT IN CODE` button
    hands the exact `attemptSource` + `mission.title` to the callback; it is
    absent on the standalone `/learn/ecky-ir` route. `COPY ATTEMPT` and
    `DOWNLOAD .ECKY` are always present and use the exact current attempt and
    `mission.handoff.suggestedFilename`.
  - 6.6 evidence: `PLAYWRIGHT_WEB_PORT=4244 npx playwright test
    e2e/docs-site.spec.ts` -> 7 passed (the 4 pre-existing docs-site scenarios
    plus the 3 Level 01 outer scenarios: pending/happy, disconnected wrong
    attempt, and passing repair with native-proof separation and solution
    availability). All three Level 01 states are proven on the real
    `/learn/ecky-ir` route in an isolated browser context per test. Initial run
    caught a `getByText` strict-mode collision (the word "native" inside
    "alternatives" and in the editor help matched the native-proof assertion);
    fixed by rewording the two shell strings (no content JSON touched) so the
    completion banner is the sole match, then re-ran to green. `npx
    svelte-check --tsconfig ./tsconfig.json` -> 0 errors, 0 warnings (the only
    flagged line, `state_referenced_locally` on the starter-initialized
    `attemptSource`, was resolved with `untrack` since `{#key mission.id}`
    remounts the shell per section). Targeted unit proof unchanged:
    `npx tsx --test` on the manifest/evaluator/progress/missions/docs-site
    suites -> 66/66 pass. No Rust, Tauri boundary, config, DB, or content JSON
    was touched; no source-control staging or commit was performed.

## 7. Content Slices: Six Real Missions

- [x] 7.1 Level 01 Corner Bracket: repair disconnected solids; transfer to named
  reinforcement or mounting feature.
- [x] 7.2 Level 02 Mounting Plate: complete cutter overrun/placement and final
  difference; compare copied cutters with a repeat rule.
- [x] 7.3 Level 03 Parametric Pattern: replace copied ribs with `repeat-union`; protect
  count/pitch relations.
- [x] 7.4 Level 04 Procedural Workshop: complete the data-to-cutter list and one final
  boolean; compare regular arrays with `map`/`range`.
- [x] 7.5 Level 05 Toothbrush Holder: use existing four-stage files for checkpoint
  diagnosis, one-cutter proof, and final repeated group.
- [x] 7.6 Level 06 Film Adapter: repair one shared fit binding across mating sides and
  distinguish preview placement from export geometry.
- [x] 7.7 For every level, validate one decision, progressive hints, one starter,
  criterion feedback, transfer prompt, full worked solution, reasoning, and at least
  one named alternative/trade-off.
- [x] 7.8 Add Playwright level-selection loop proving all six manifests load distinct
  artifacts and later missions reduce scaffolding.

  - Approach: BDD outside-in. Added a focused content-projection test file
    `src/lib/docs/eckyMissionLessons.test.ts` (8 tests). Each of 7.1-7.6 is one test
    that proves the level's EXACT lesson through the real structural evaluator on the
    published, normalized bundle: the canonical solution passes every criterion; the
    starter fails its discriminating criterion (a real exercise); the accepted
    decision option and a weak option's consequence feedback are level-specific; and
    the transfer prompt names the level's required relation (reinforcement/mounting,
    repeat/array, count/pitch binding, map/range vs grid-array, staggered row,
    preview-only offset vs export geometry). Test 7.7 re-runs the full required
    teaching set across all six levels (>=1 accepted decision option, >=2 progressive
    hints that never paste the solution, one starter, every criterion pass+fail
    feedback, transfer prompt, worked solution, reasoning, and >=1 named
    alternative). The final test asserts scaffolding reduction.
  - Red evidence: ran the new file RED first. 7.1-7.7 content already satisfied the
    required lesson behavior (prior Section-3 content was authoritative and complete);
    the only real failure was scaffolding reduction: all six levels shipped 4 hints,
    so `4 -> 4` failed the non-increasing/strictly-lighter assertion. Two transient
    test-expectation bugs (L04/L05 starter-omission regex matched the COMPLETE
    instruction comment) were fixed by proving omission through the evaluator's
    criterion result (`cutter-list`/`repeated-group` fail on the starter) instead of a
    raw-source match, which is also more honest.
  - Content patch (only the gap the test exposed): per the design's "later levels may
    omit syntax hints already practiced earlier," removed the redundant trailing
    `Remove the DEFECT/COMPLETE comment` hint from level-03, level-04, and level-05
    (the `removedPlaceholder` criterion and its `failFeedback` already tell the
    learner to remove the marker, and the marker convention is established in
    level-01/02), and removed that hint plus one restate-of-starter hint from the
    final level-06. Level-01/02 keep 4 hints. Resulting hint ladder: 4, 4, 3, 3, 3, 2
    (non-increasing; last strictly lighter than first). Conceptual and key-actionable
    hints were preserved; only redundant syntax/restate hints were dropped. Regenerated
    `public/tutorials/ecky-missions.json` via `npm run sync:book-source`.
  - 7.8 evidence: appended a six-level selection loop to `e2e/docs-site.spec.ts` under
    `test.describe('interactive-ecky-learning-missions: six-level selection loop
    (7.8)')`, reusing the existing workbench on the real `/learn/ecky-ir` route. It
    selects each campaign level via the sidebar, asserts the matching mission mounts
    (title + artifact), all five phases render, the level's distinct starter
    pre-fills the attempt editor (`inputValue`), and reads the hint-ladder total from
    the live `REVEAL HINT (0/N)` button. It then proves all six artifacts, objectives,
    and starters are mutually distinct and that hint totals are non-increasing with the
    final level strictly lighter than the first.
  - Proof: `npx tsx --test` on the mission docs suites (`eckyIrMissions`,
    `eckyMissionLessons`, `eckyMissionEvaluator`, `eckyMissionManifest`,
    `eckyMissionProgress`, `eckyIrDocsSite`) -> 74/74 pass.
    `PLAYWRIGHT_WEB_PORT=4245 npx playwright test e2e/docs-site.spec.ts -g "Level 01|
    six-level selection loop"` -> 4 passed (3 Level 01 outer scenarios + the new loop)
    on the real route; ports 5173/4243/4244/8787 were occupied, the existing 8787
    server was reused. `npm run check:book-source` -> `Ecky content sources are
    current.`; `openspec validate interactive-ecky-learning-missions --strict` ->
    valid. No Rust, Tauri boundary, config, DB, frontend-workbench logic, section 8/9,
    source-control staging, or commit was involved; footprint is one new test file,
    four canonical mission JSON hint arrays, the regenerated public bundle, and one
    appended Playwright describe block.

## 8. Failure, Pending, And Solution Proof

- [x] 8.1 Prove untouched attempt remains pending and no success/compile language
  appears.
- [x] 8.2 Prove a wrong decision remains visible with consequence-specific feedback.
- [x] 8.3 Prove malformed manifest request shows raw loading/validation error without
  hiding the readable Markdown chapter.
- [x] 8.4 Prove solution requires an attempt or explicit override, then shows exact
  source, reasoning, runtime limitation, and alternative trade-off.
- [x] 8.5 Prove reload restores bounded progress and reset affects only active mission.

  - Approach: BDD browser-first on the real `/learn/ecky-ir` campaign route.
    Added one `test.describe('interactive-ecky-learning-missions: failure, pending,
    and solution proof (8.1-8.5)')` block to `e2e/docs-site.spec.ts` (six focused
    scenarios, one per task, with 8.5 split into reload-restore and reset-scope).
    Route interception is used only for 8.3 (malformed bundle); a `localStorage`
    fixture is used only for 8.5. No production code, content JSON, Rust/Tauri,
    config, or DB file was touched — the workbench and DocsSite already implement
    every required behavior, so every scenario went green once the assertions were
    expressed against the real DOM (the only edits were test mechanics: `toHaveValue`
    on the attempt `<textarea>` instead of `toContainText`, and matching the real
    native-proof note wording `do not prove`).
  - 8.1 evidence: `8.1` proves the untouched attempt still holds the starter
    (`DEFECT` marker present in the editor via `toHaveValue`), the
    `mission-structure-feedback` live region shows the pending state, and that same
    region does NOT contain `success|succeed|passed|complete|compiled|valid solid|
    printable|watertight|exported|export success`. The forbidden-language scan is
    scoped to the feedback region so the legitimate campaign prose ("compile, and
    export as STL") does not weaken the proof.
  - 8.2 evidence: `8.2` selects the weak `two-parts` DECIDE option, asserts it stays
    `aria-pressed="true"`, the `.mission-decision-feedback` carries the
    `--weak` class (not `--accepted`) and names the consequence (loose/independent
    bodies, not one connected body). It then types distinct in-progress source,
    switches to the accepted `overlap-union` option, and asserts the attempt source
    is exactly preserved — proving the learner may re-choose without resetting
    source work.
  - 8.3 evidence: `8.3` intercepts `**/tutorials/ecky-missions.json` via
    `route.fetch()`, clones the real published bundle, and mutates Level 01's first
    criterion `kind` to `compilesCleanly` (unsupported) so the ONLY discriminating
    failure is the unsupported criterion. It asserts `.docs-mission-error` is
    visible with `Mission manifest invalid`, the `unsupported-criterion` code, the
    `compilesCleanly` kind (not silently skipped), and the "chapter below remains
    readable" note; `.mission-workbench` is absent; and the readable chapter still
    renders (`.docs-article h2` = Corner Bracket, body prose present).
  - 8.4 evidence: `8.4` proves both reveal paths. Before any attempt, the first
    `SHOW WORKED SOLUTION` activation only arms `SHOW SOLUTION ANYWAY` and exposes
    no source (`.mission-code--solution` count 0). The explicit override then
    reveals exact canonical source (`(part bracket`/`(union)`), ordered reasoning,
    the labeled runtime limitation (`.mission-solution__section-head` "Native proof
    required" + note asserting `do not prove` + `compil|printable|watertight|export`),
    and at least one named alternative with non-empty name and trade-off. It then
    hides, runs CHECK STRUCTURE on the passing repair, and proves the after-attempt
    path reveals directly with no override.
  - 8.5 evidence: `8.5a` injects a `localStorage` fixture under
    `eckyLearningProgress:v1` with a passing/checked Level 01 (accepted decision,
    hintDepth 2, solution revealed) plus an unknown `level-99-ghost` entry, then
    reloads and asserts the restored attempt value equals the saved source (not the
    starter), the accepted option stays pressed, exactly two hints are revealed,
    the feedback shows the re-evaluated pass (deterministic evaluator re-runs on
    restored source), the solution stays revealed, and the ghost source never leaks
    into the DOM (bounded). `8.5b` stores progress for Level 01 and Level 02,
    performs the two-step reset on Level 01 (RESET MISSION -> CONFIRM RESET), asserts
    Level 01 returns to the starter (`DEFECT`) and pending with no solution, then
    switches to Level 02 via the sidebar and confirms ITS distinct attempt and
    accepted decision survived, and returns to Level 01 still on the starter —
    proving reset was persisted and affected only the active mission.
  - Section 8 proof: `PLAYWRIGHT_WEB_PORT=4244 npx playwright test
    e2e/docs-site.spec.ts -g "8.1|8.2|8.3|8.4|8.5"` -> 6 passed (happy/failure/
    pending all covered). Full `e2e/docs-site.spec.ts` on the same port -> 14 passed
    (4 pre-existing docs-site + 3 Level 01 outer + 1 six-level loop + 6 section-8),
    so no existing test regressed. Targeted unit proof unchanged: `npx tsx --test`
    on the mission docs suites (manifest, evaluator, progress, lessons, missions,
    docs-site) -> 74/74 pass. `npm run check:book-source` -> `Ecky content sources
    are current.`; `openspec validate interactive-ecky-learning-missions --strict`
    -> valid. Footprint is one test file, +440/-0 lines; no Section 9 / full suite
    was run; no source-control staging or commit was performed.

## 9. Integration Green And Browser Proof

- [x] 9.1 Run targeted mission/content unit tests after every green step.
- [x] 9.2 Run `npm run test:unit`.
- [x] 9.3 Run `npm run typecheck`, `npm run check:book-source`,
  `npm run build:book`, and `npm run build`.
- [x] 9.4 Run targeted `npx playwright test e2e/docs-site.spec.ts` on an alternate port
  if the default server is occupied.
- [x] 9.5 Run full `npm run test:e2e`.
- [x] 9.6 Inspect `/learn/ecky-ir` in a real browser at desktop and 390px widths;
  capture happy, wrong, pending, and revealed-solution evidence. Confirm no horizontal
  overflow, clipped editor feedback, console errors, ball/platform artifact, or theme
  drift.
- [x] 9.7 Run `cd src-tauri && cargo check`.
- [x] 9.8 Re-run
  `openspec validate interactive-ecky-learning-missions --strict`.
- [x] 9.9 Do not stage, commit, or alter source-control descriptions unless requested.

  - 9.1 evidence: `npx tsx --test` on the eleven mission/content suites
    (`eckyIrContent`, `eckyIrSource`, `eckyIrWorkedExamples`, `eckyIrGuide`,
    `eckyIrBook`, `eckyIrMissions`, `eckyIrDocsSite`, `eckyMissionManifest`,
    `eckyMissionEvaluator`, `eckyMissionProgress`, `eckyMissionLessons`) -> 91/91
    pass. (Mission suite grew from the 74 recorded in section 7/8 as lessons
    coverage expanded.)
  - 9.2 evidence: `npm run test:unit` (`tsx --test src/lib/**/*.test.ts`) ->
    383/383 pass, 0 fail.
  - 9.3 evidence: `npm run typecheck` (`svelte-check` + `tsc -p tsconfig.node.json`)
    -> 0 errors, 0 warnings. `npm run check:book-source` -> `Ecky content sources
    are current.` `npm run build:book` -> rebuilt
    `public/docs/ecky-ir-field-guide.epub|.html` + assets. `npm run build`
    (`vite build`) -> 332 modules transformed, built in 2.21s (the only output is
    the pre-existing >500kB chunk-size advisory, unrelated to this change).
  - 9.4 evidence: `PLAYWRIGHT_WEB_PORT=4244 npx playwright test e2e/docs-site.spec.ts`
    -> 14 passed (4 pre-existing docs-site + 3 Level 01 outer + 1 six-level loop +
    6 section-8) on the real `/learn/ecky-ir` route, isolated dev server.
  - 9.5 evidence: full `npm run test:e2e` run EXACTLY ONCE on `PLAYWRIGHT_WEB_PORT=
    4245` (reusing the running `dev:server` on 8787) -> 207 passed / 4 failed of
    211. Per protocol, only the failing specs were re-run (4246) and confirmed
    deterministic. Root-caused all four:
    (a) Three failures in `e2e/app.spec.ts` (inspector toggle, docs-window
    lessons, docs-snippet-opens-in-code) were caused by THIS change's authoritative
    Level 01 rename Marker -> Corner Bracket (documented in 2.2/3.1; `eckyIrBook`
    and `eckyIrGuide` unit tests were already updated for the same rename, but
    `e2e/app.spec.ts` was missed and was unmodified in the worktree). Applied the
    smallest in-scope, non-conflicting fix: renamed the seven `Level 01: Marker`
    references to `Level 01: Corner Bracket`, and updated the two content
    assertions tied to the removed ball-on-base model (Corner Bracket is
    intentionally imageless, so the `img[alt*="First Solid"]` check was replaced
    by a union-lesson pre check; `(sphere 10)` -> `(box 40 40 6)`). Reran
    `e2e/app.spec.ts` on 4247 -> 7/7 pass. This is test-only and consistent with
    the rename already applied elsewhere; no production/content JSON touched.
    (b) ONE remaining failure is OUT OF SCOPE: `e2e/context-controls.spec.ts:863`
    (Params measure mode) expects `.param-panel .param-list .param-field` count 0
    but gets 2. It is in the params/measure-mode domain (`src/lib/ParamPanel.svelte`
    is dirty from other in-flight work), NOT touched by this change (mission
    footprint is DocsSite.svelte + EckyMissionWorkbench.svelte + docs content /
    scripts / tests). No non-conflicting in-scope fix exists without entering
    another change's territory, so it is REPORTED, not fixed. Net: mission-relevant
    e2e is fully green (210/211); the single outstanding failure is unrelated to
    this change.
  - 9.6 evidence: real-browser (Chromium) inspection of `/learn/ecky-ir` on the
    running 4243 dev server at desktop (1280x800) and mobile (390x844). Drove all
    four states (pending, wrong/disconnected, happy/passing-repair,
    revealed-solution) at both widths; 8 viewport screenshots saved under
    `/tmp/ecky-9x/shots/` plus a visible browser-tools capture. Deterministic
    measurements: (1) NO console errors and NO page errors at either width;
    (2) NO horizontal overflow — `document.documentElement.scrollWidth ===
    innerWidth` and every major container (`.docs-shell`, `.docs-article`,
    `.mission-workbench`, `.mission-editor`, `.mission-feedback`) has
    `scrollWidth === clientWidth` at both 1280 and 390; (3) editor feedback NOT
    clipped — feedback right edge (1226 desktop / 336 mobile) is inside the
    viewport and the region's content is fully present; (4) NO ball/platform
    artifact — `body` text matches none of `ball on a base|ball-on-base|sphere on
    a platform|first solid: ball` at either width; (5) criterion feedback is
    exact on the wrong attempt (names the detached `(translate 60 0 0)` flange /
    no-overlap and unbalanced parens) and claims NO native success
    (no `compiled|valid solid|printable|watertight|exported`); the happy attempt
    reads `Structural practice complete. Native compile and preview remain
    separate proof.` with the worked solution revealing canonical
    `(part bracket`/`(union)` source and the native-proof section; (6) NO theme
    drift — Tactical Midnight tokens are identical at both widths
    (`--primary #4a8c5c`, `--secondary #c8a620` bronze, `--red #ff6b6b`,
    `--bg-300 #2a2a4a`, `--text #e0e0e0`, `--text-dim #888`), and the workbench
    reuses only established tokens.
  - 9.7 evidence: `cd src-tauri && cargo check` -> exit 0 (Finished `dev` profile
    in 0.89s). The only diagnostic is one pre-existing `unused import
    PROJECT_MANIFEST_FILE_NAME` warning in `src/thread_source_binding.rs`, which
    belongs to the separate thread-source-binding change, not this change (the
    mission touched zero Rust). Serialized after all JS/e2e work per instruction;
    load stayed benign (no pathological cargo/rustc/direct_occt/syspolicyd spike).
  - 9.8 evidence: `openspec validate interactive-ecky-learning-missions --strict`
    -> `Change 'interactive-ecky-learning-missions' is valid` (exit 0).
  - 9.9 evidence: No source-control operations of any kind were performed — no
    `git add`, `git commit`, `git stash`, `jj describe`, `jj commit`, archive, or
    branch operation. The only files written/edited in this window are the
    in-scope test fix (`e2e/app.spec.ts`) and this tracking file
    (`tasks.md`). All prior/newer edits in the shared worktree were preserved;
    nothing was reverted or bulk-formatted.
