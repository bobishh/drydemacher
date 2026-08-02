## Context

The campaign source is projected from
`docs/books/ecky-ir/ecky-ir-corpus.md` into
`public/tutorials/ecky-campaign.md`. Split level views and EPUB reuse the same
projection. `DocsSite.svelte` fetches the campaign Markdown on `/learn/ecky-ir`,
parses each `##` section, and renders one selected level. It can copy, download, or,
inside the desktop workbench, open the first code fence. It has no exercise model,
attempt state, feedback, solution comparison, or progress.

Existing content has two different strengths:

- Levels 01–04 explain operations through complete examples.
- Levels 05–06 contain production-scale source and real staged reasoning.

The redesign keeps that book structure but adds a learning loop around it. It does
not convert the dry operation reference into a course and does not add a decorative
3D mission object.

Research basis:

- Sweller and Cooper's worked-example research found novice learning improved when
  study used worked solutions instead of equivalent means-ends problem solving.
- Renkl, Atkinson, Maier, and Staley found a smoother transition when solution steps
  fade from complete examples through increasingly incomplete examples to independent
  problems.
- Parsons and Haden introduced code-reordering tasks to practice program structure
  while reducing syntax burden. Ecky missions use the same constraint principle for
  early decision/completion tasks, but do not require drag-and-drop blocks in this
  increment.
- Karpicke and Roediger found repeated retrieval produced stronger delayed retention
  than repeated study. Every level therefore ends with a less-scaffolded transfer
  task rather than another reread.

References:

- https://doi.org/10.1080/00461528509529059
- https://doi.org/10.1207/S1532690XCI2001_3
- https://dl.acm.org/doi/10.5555/1151869.1151890
- https://doi.org/10.1126/science.1152408

## Goals / Non-Goals

**Goals:**

- Make each level require observable learner decisions and authored code.
- Move from explained example to completion/repair to independent transfer.
- Give immediate, criterion-specific feedback without pretending a token check is a
  compiler or geometry validator.
- Make worked solutions easy to inspect after an attempt, including why they work and
  when an alternative is preferable.
- Reuse canonical book projects, staged `.ecky` files, and existing app handoff.
- Preserve Tactical Midnight, square borders, and bounded layouts.

**Non-Goals:**

- A generic LMS, account system, cloud progress sync, grading, certificates, points,
  streaks, avatars, or gamified world map.
- A decorative sphere/platform, mascot mission stage, or fake 3D preview.
- Running native Ecky, OCCT, STL export, or arbitrary code in the public browser.
- Replacing the language reference or embedding substantive lessons in Svelte.
- Full Parsons drag-and-drop, collaborative classrooms, or AI-generated grading.

## Artifact Model

### Canonical written source

The book corpus remains canonical for chapter prose and long-form worked examples:

`docs/books/ecky-ir/ecky-ir-corpus.md`

Level 01 changes there from `First Solid: Ball on a Base` to a useful corner-bracket
lesson. Generated campaign Markdown, split chapters, and EPUB must remain projections,
not independent edits.

### Canonical interactive source

One JSON file per mission lives under:

`docs/books/ecky-ir/missions/<level-slug>.json`

An index defines order and schema version:

`docs/books/ecky-ir/missions/manifest.json`

Publication copies validated, normalized mission data to:

`public/tutorials/ecky-missions.json`

The public bundle uses idiomatic `camelCase`. Required mission fields:

- `id`, `sectionSlug`, `title`, `artifact`, `schemaVersion`;
- `briefing`, `objective`, `transferPrompt`;
- `workedExample`: source plus ordered decision annotations;
- `decision`: prompt, options, option feedback, and accepted option ids;
- `practice`: mode (`completion` or `repair`), starter source, criteria, hints;
- `solution`: source, reasoning steps, and named alternatives with trade-offs;
- `handoff`: suggested filename and whether in-app open is supported.

Criteria use a bounded declarative vocabulary, never executable regular expressions
from content:

- `containsForm`;
- `containsSymbol`;
- `excludesForm`;
- `minFormCount`;
- `balancedDelimiters`;
- `removedPlaceholder`.

Each criterion has stable id, learner-facing requirement, pass feedback, and exact
failure feedback. The evaluator returns all criterion results; it never collapses a
provider/runtime error into `Check API Key` or another generic message.

## Variables

- Content format: canonical Markdown plus canonical JSON mission files.
- Storage location: `docs/books/ecky-ir/`; generated web artifacts under
  `public/tutorials/`.
- Routing: interactive campaign on `/learn/ecky-ir` and `/ecky-ir`; dry reference on
  `/docs/ecky-ir`.
- Backend ownership: none for browser structural checks; existing Ecky runtime owns
  compile, preview, and export truth.
- Frontend ownership: fetch, render, attempt edit, deterministic structural
  evaluation, hint/reveal state, app handoff, and local progress.
- Editing model: one bounded plain-text Ecky editor per active mission; no hidden
  contenteditable or Svelte-embedded source.
- Feedback model: pending before check, criterion failures after check, complete only
  when every structural criterion passes.
- Solution model: explicit reveal after an attempt, with an intentional override
  action for a learner who is stuck.
- Progress storage: versioned local storage keyed by mission id; source attempts,
  completed criteria, selected decision, hint depth, and solution-reveal state only.
- Testing surface: pure evaluator/unit tests and Playwright on real campaign routes.
- Runtime constraints: Svelte 5, static fetchable artifacts, no new dependency, no
  native runtime claim in browser.
- Export/handoff: exact `.ecky` copy/download; existing `OPEN IN CODE` callback where
  the app supplies it.

## Decisions

### 1. Book spine, practice layer

Keep six ordered chapters. Inside each selected level, render:

1. `BRIEF` — artifact and observable outcome.
2. `STUDY` — one annotated worked example.
3. `DECIDE` — choose between plausible modeling approaches and read consequence
   feedback.
4. `COMPLETE` or `REPAIR` — edit scaffolded source, then run structural checks.
5. `TRANSFER` — a less-scaffolded variation stated before solution reveal.
6. `DEBRIEF` — worked solution, decision rationale, and alternative trade-offs.

The long chapter remains below the workbench as reference. The workbench does not hide
or duplicate the chapter prose.

Alternative: rename chapters as missions and add checkboxes. Rejected because it
measures reading, not solution production.

Alternative: start every level with an open-ended blank editor. Rejected because
novices spend working memory on syntax search instead of the target modeling relation.

### 2. Replace the ball-on-base example

Level 01 becomes `Corner Bracket`: union one horizontal foot and one vertical flange,
with deliberate overlap producing one printable part. The worked example identifies
root, part naming, transform/placement, and union. Practice asks the learner to repair
disconnected solids. Transfer asks for a parametric reinforcement or mounting feature
without reverting to copied shape blocks.

No sphere-on-platform render, challenge, or solution remains in campaign Level 01.
The dry operation reference may still document `sphere` as a language primitive.

### 3. Faded guidance across levels

Scaffolding decreases across the campaign:

- Level 01: near-complete source with one placement/union defect.
- Level 02: missing cutter placement and final difference relation.
- Level 03: copied ribs must become one repeat rule.
- Level 04: incomplete data-to-cutter pipeline.
- Level 05: choose and complete one checkpoint from the existing four-stage holder.
- Level 06: repair a shared fit relation across mating parts, then explain export
  versus preview placement.

Within a level, hints reveal one concept at a time. A hint does not paste the complete
solution. Later levels may omit syntax hints already practiced earlier.

### 4. Honest two-tier verification

`CHECK STRUCTURE` runs only the pure browser evaluator. Its result copy names what was
checked: forms, symbols, repeated structure, placeholders, and delimiter balance.

It must not say `compiled`, `valid solid`, `printable`, `watertight`, or `exported`.
When the desktop callback exists, `OPEN ATTEMPT IN CODE` hands the exact attempt to the
real authoring surface for compile and preview. Otherwise `COPY ATTEMPT` and
`DOWNLOAD .ECKY` provide exact handoff.

Alternative: send attempts to a new server execution endpoint. Rejected because it
introduces arbitrary-code execution, deployment, and trust boundaries unrelated to
the instructional increment.

### 5. Solutions are first-class content

Every mission ships one canonical worked solution and at least one alternative. The
solution view pairs source with ordered reasoning:

- decision made;
- invariant or fit relation protected;
- why placement/boolean/repeat was chosen;
- what the structural check proves;
- what still requires native compile/preview;
- trade-off of each alternative.

Reveal is not a CSS-obscured answer. The shell conditionally renders solution data
after the learner checks once or confirms `SHOW SOLUTION ANYWAY`. Progress records the
reveal so return visits are deterministic.

### 6. Decision feedback before code feedback

Each mission contains one choice among plausible approaches. Selecting an option
immediately shows specific consequence feedback. Wrong choices remain selectable and
visible; feedback explains failure mode rather than resetting the learner.

Early choices may constrain structure, similar to a Parsons problem, but this change
uses accessible native buttons instead of drag-and-drop. Keyboard and touch behavior
therefore remain simple and testable.

### 7. Versioned local progress

Progress key:

`eckyLearningProgress:v1`

Stored data is bounded to mission ids in the currently loaded manifest. Invalid,
unknown-version, or malformed data is ignored without blocking content. `RESET
MISSION` clears only the active mission after confirmation through a second explicit
activation; it does not touch app configuration or `app_config_dir/config.edn`.

### 8. Frontend boundaries

Introduce:

- `src/lib/docs/eckyMissionManifest.ts`: types, validation, normalization;
- `src/lib/docs/eckyMissionEvaluator.ts`: pure structural evaluator;
- `src/lib/docs/eckyMissionProgress.ts`: pure serialization and restoration;
- `src/lib/EckyMissionWorkbench.svelte`: interaction shell only.

`DocsSite.svelte` owns campaign manifest fetch and active-section-to-mission mapping.
All major workbench containers use `overflow: hidden`; editor, feedback list, and
solution source own explicit internal scrolling. Styles reuse `--primary`,
`--secondary`, existing Tactical Midnight backgrounds, and square borders.

## Rejected Paths

- Mockup-only mission cards without editable attempts.
- Content strings, source, answers, or criteria embedded in Svelte.
- One giant manifest handwritten under `public/` with no canonical source validation.
- Automatic AI grading or generic feedback.
- Browser claims based on matching the canonical source byte-for-byte.
- Reward currency, XP, streak pressure, world map, decorative platform, or sphere.
- Making the static `/docs/` operation reference carry interactive course state.
- Copy-pasted repeated CAD structures inside new worked solutions.

## BDD Delivery Slices

### Slice 1: Level 01 learning loop

Outer test starts red:

- Given `/learn/ecky-ir` opens at Level 01
- Then Corner Bracket, BRIEF/STUDY/DECIDE/PRACTICE/TRANSFER labels, starter editor,
  pending structural state, and no ball-on-base content are visible.
- When a disconnected attempt is checked
- Then placement/connection criteria fail with exact feedback and no compile claim.
- When a passing repair is checked
- Then every criterion passes, progress becomes complete, and solution/debrief becomes
  available.

Inner unit loops implement manifest parsing, form scanning, feedback, and component
states.

### Slice 2: solution and alternative

Outer test starts red:

- Given one failed attempt
- When solution is revealed
- Then full worked source, ordered reasoning, one alternative, and native-runtime
  limitation are visible.

### Slice 3: all six levels and fading

Outer test starts red:

- Given each campaign level is selected
- Then exactly one matching mission loads and has a distinct starter, transfer prompt,
  solution, and at least one alternative.
- Later levels expose fewer scaffold placeholders than earlier levels.

### Slice 4: app handoff and standalone fallback

Outer tests prove exact attempt handoff through `OPEN ATTEMPT IN CODE` when callback
exists and exact copy/download actions when it does not.

### Slice 5: persistence and malformed state

Unit and browser tests prove restore after reload, active-mission reset, and graceful
fallback for corrupt or obsolete local progress.

## Risks / Trade-offs

- [Structural checks can be gamed] → Label scope explicitly; require native handoff for
  compile/geometry truth.
- [Solutions reduce productive struggle] → Require an attempt or explicit override;
  show reasoning and transfer prompt, not source alone.
- [Manifest prose drifts from book] → Validate section slugs, ids, order, filenames,
  and generated projection during `check:book-source`.
- [Six levels create large initial scope] → Ship shared engine through Level 01 first,
  then author each mission as a content-driven slice; no level counts complete without
  its own browser selection proof.
- [Local progress schema changes] → Version key and ignore incompatible records.
- [Nested workbench crowds small screens] → One active phase at a time below 760px;
  bounded containers and explicit inner scroll.

## Migration Plan

1. Add failing campaign Playwright scenarios.
2. Change canonical Level 01 chapter and regenerate campaign/book projections.
3. Add mission JSON schema, six canonical mission files, projection, and validation.
4. Add pure evaluator and progress unit loops.
5. Add workbench shell and integrate it only for campaign routes.
6. Complete six mission content slices with worked solutions and alternatives.
7. Run targeted unit/e2e, full frontend suites, book/source checks, build, and visual
   proof at desktop and mobile widths.
8. Run `cd src-tauri && cargo check` before reporting implementation success, even
   though this change should not modify Rust.

Rollback removes the campaign workbench and mission publication artifact. Markdown
chapters remain readable. Local progress is isolated under one versioned key and may be
left inert or removed by a later migration.

## Open Questions

- None blocking. First implementation may use a plain textarea. Reusing the existing
  CodeMirror adapter is allowed only if it does not couple mission content to editor
  runtime or break the static route.
