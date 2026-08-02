## Context

The campaign uses file-backed Markdown plus six JSON mission records. `DocsSite`
loads the active chapter and mission. `EckyMissionWorkbench` owns interaction. Current
implementation renders all phases and the full chapter together. Level 01 stores the
same source in `workedExample.source` and `solution.source`. Level 03 remains a copied
rib repair. No canonical Corner Bracket render exists.

## Goal

Make the campaign teach one decision at a time and show real geometry. Use a useful
dovetail fit as the first physical-relations mission.

## Artifact model

- Chapter prose: `docs/books/ecky-ir/ecky-ir-corpus.md`.
- Mission content: `docs/books/ecky-ir/missions/*.json`.
- Runnable source: `docs/books/ecky-ir/examples/*.ecky`.
- Render source of truth: canonical `.ecky` file.
- Derived artifacts: STL from native Ecky; PNG from existing Three/WebGL browser
  renderer; published under book assets.
- Web bundle: generated `public/tutorials/ecky-missions.json`.
- Offline book: generated EPUB, explicitly separate from interactive campaign.

## Variables

- Content format: Markdown, JSON, Ecky source, generated STL/PNG.
- Storage: canonical `docs/books/ecky-ir`; generated `public/` and `target/book`.
- Route: `/learn/ecky-ir` and campaign aliases.
- Backend ownership: native Ecky owns geometry truth.
- Frontend ownership: active-phase state, reveal flow, image display, offline-book CTA.
- Editing model: existing textarea and structural checks.
- Fit model: one named `fit_clearance` binding shared by dovetail male/female sides.
- Testing: pure content invariants plus real-route Playwright.
- Runtime: ARM64 Ecky/direct OCCT; Three/WebGL browser capture; no OpenSCAD.
- Export: production parts remain separate; preview-only placement never enters
  exported geometry.

## Decisions

### 1. One phase, one task

Only the active phase renders. Initial phase is BRIEF. Buttons switch mounted content,
not scroll position. PRACTICE retains attempt state while hidden. DEBRIEF becomes a
sixth phase only after existing reveal rules allow it.

### 2. Example-answer separation

`workedExample.source` and `solution.source` MUST differ materially. STUDY uses an
analogous artifact or relation. It may demonstrate overlap, clearance, or a boolean,
but cannot contain the practice answer or exact edit. BRIEF states problem and outcome,
not construction recipe. Hints reveal concepts progressively, never replacement text.

### 3. Campaign/book boundary

Interactive campaign does not render the complete chapter answer beneath the
workbench. It offers an explicit optional `LESSON NOTES` surface or link. EPUB and
reference retain file-backed prose. Header action reads `OFFLINE BOOK · EPUB`, includes
format semantics, and uses secondary styling.

### 4. Real Corner Bracket render

Canonical Corner Bracket `.ecky` source renders through native Ecky/direct OCCT to
STL. Existing Three/STLLoader browser code frames and lights the actual mesh, then
captures deterministic PNG. Build/source checks bind source and published asset.
Campaign `<img>` exposes descriptive alt text and non-zero natural dimensions.

OpenSCAD, AI image generation, diagram substitutes, and hand-drawn fake geometry are
forbidden.

### 5. Dovetail Fit reuses production geometry

Level 03 becomes `Dovetail Fit`. Source is extracted from the proven dovetail/rail
fit subsystem in
`docs/books/ecky-ir/examples/ecky-integrated-film-adapter-open-helicoid-v9.ecky`.
The campaign fixture may reduce surrounding mechanism complexity, but MUST preserve
the existing profile and fit relation rather than design another dovetail.

It presents two exportable parts:

- nominal male dovetail rail;
- female body with dovetail channel enlarged from the same named `fit_clearance`.

Profile geometry remains reusable. Any repeated geometry uses `repeat` or `instance`.
Preview assembly transforms are diagnostics only and remain outside production export
geometry. Practice repairs a broken shared-clearance relation. Transfer changes fit
without editing multiple offsets. Rib repetition moves to optional reference notes.

Existing Ecky runtime, direct-OCCT, and film-adapter tests own CAD correctness. This
change MUST NOT duplicate tests for polygon, extrude, difference, rail validity, or
general dovetail geometry. One native CLI/MCP validation proves the extracted fixture
still compiles/renders. Campaign tests cover only projection, source provenance,
named-clearance teaching, and visible asset loading.

### 6. Minimal outer-loop coverage

Follow one-slice BDD literally. Add one failing real-route Playwright scenario, confirm
its exact failure, implement that slice, turn it green, then start the next slice.
Do not pre-author the full future acceptance matrix. Pure content tests are allowed
only for invariants not already owned by runtime/model tests.

## Rejected paths

- Another broad six-level rewrite before Level 01 is green.
- Showing all phases and using navigation as decoration.
- Worked example identical to solution.
- Static EPUB presented as downloadable interactive campaign.
- OpenSCAD or other GUI renderer.
- Anonymous dovetail offsets.
- Fake PNG unrelated to committed Ecky source.
- Re-testing existing dovetail CAD operations in campaign unit or browser tests.
- Nine speculative RED browser tests written before the first slice turns green.

## Proof plan

1. RED/GREEN: one active phase plus example-answer separation.
2. RED/GREEN: real Corner Bracket asset.
3. RED/GREEN: explicit EPUB CTA.
4. RED/GREEN: production-derived Dovetail mission. Existing CAD tests are reused;
   one route scenario proves campaign projection and asset.
5. Audit remaining levels with content invariant tests.
6. Desktop and 390px browser proof: initial, phase switch, wrong/pending, solution,
   images, no duplicate chapter, no console errors, no horizontal overflow.

## Rollback

Each slice is independent. Phase shell may revert without deleting canonical content.
Generated assets may revert with their source references. Dovetail replaces only
Level 03 campaign projection; language reference remains intact.
