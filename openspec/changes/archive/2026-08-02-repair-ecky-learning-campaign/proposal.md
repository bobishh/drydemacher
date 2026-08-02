## Why

The interactive Ecky campaign passes tests but fails as teaching material. Level 01
has no visible model render. Its worked example exposes the practice solution. Phase
buttons highlight a label while every phase remains visible. Campaign prose repeats
the same answer below the workbench. The header calls a static EPUB download
`DOWNLOAD CAMPAIGN`. Level 03 teaches a ribbed plate instead of a useful physical fit.

The previous change mixed implementation and product decisions. This repair defines
small observable slices before more code is written.

## What Changes

- Render exactly one active mission phase at a time.
- Keep BRIEF problem-oriented and short.
- Require STUDY and SOLUTION to use materially different source and artifacts.
- Remove simultaneous long-form answer duplication from the campaign reader while
  preserving file-backed book/reference content.
- Publish a real Corner Bracket image generated from canonical Ecky geometry through
  the native Ecky runtime and existing Three/WebGL browser pipeline.
- Rename the EPUB action to `OFFLINE BOOK · EPUB`; present it as secondary.
- Replace Level 03 ribbed plate with a practical Dovetail Fit mission extracted from
  the existing production film-adapter dovetail subsystem. Do not invent or retest a
  second dovetail implementation.
- Audit Levels 02, 04, 05, and 06 only after the four concrete repairs are green.

## Capabilities

### New Capabilities

- `ecky-learning-campaign-quality`: Observable teaching, rendering, and offline-book
  boundaries for the interactive campaign.

## Impact

- Canonical content: `docs/books/ecky-ir/` Markdown, mission JSON, examples, assets.
- Published content: `public/tutorials/`, generated `public/docs/` book artifacts.
- Shell: `src/lib/DocsSite.svelte`, `src/lib/EckyMissionWorkbench.svelte`.
- Tests: focused mission repair unit tests and real-route Playwright.
- Existing CAD runtime/model tests remain authoritative for dovetail geometry. This
  change tests only campaign projection and visible teaching behavior.
- No database, config, Tauri payload, or source-control schema change.
