## Why

The Ecky campaign currently labels book chapters as missions but does not make the
reader solve anything. A level presents finished code, a render, and a clear
condition; the UI can only copy or download the first code block. The first level's
ball-on-base artifact is especially weak: it demonstrates syntax but gives no useful
design decision, completion task, feedback, or transfer.

The repository already contains strong worked projects and staged source files.
Those assets need an instructional shell that moves a novice from explained examples
to partially completed models and then to independent solutions.

## What Changes

- Keep the six-level book as the campaign spine under `/learn/ecky-ir`; keep
  `/docs/ecky-ir` as the dry language reference.
- Replace Level 01's ball-on-base marker with a useful two-solid corner bracket.
- Give every campaign level a file-backed learning sequence:
  - concise briefing and observable goal;
  - annotated worked example;
  - one decision checkpoint with consequence feedback;
  - one completion or repair exercise;
  - one less-scaffolded transfer task;
  - progressive hints;
  - criterion-specific feedback;
  - explicit worked solution, reasoning, and alternative trade-offs.
- Add a typed mission manifest and build-time publication path. Substantive lesson
  content, starter code, checks, hints, and solutions remain outside Svelte.
- Add a Tactical Midnight mission workbench to the campaign reader. It owns loading,
  editing, step navigation, structural checks, solution reveal, and local progress.
- Label browser checks honestly as structural checks. Do not claim compilation,
  geometry validity, or export success without the real Ecky runtime.
- Connect runnable code to the existing in-app `OPEN IN CODE` path when available;
  provide exact copy/download handoff on standalone web routes.
- Add BDD browser proof for a completed attempt plus wrong and untouched/pending
  states. Add unit proof for manifest parsing, evaluation, and progress restoration.

## Capabilities

### New Capabilities

- `interactive-ecky-learning`: File-backed, scaffolded Ecky campaign missions with
  worked examples, practice, feedback, solutions, and transfer.

### Modified Capabilities

- None. The language reference and Ecky runtime contracts remain unchanged.

## Impact

- Canonical content: `docs/books/ecky-ir/ecky-ir-corpus.md` and new
  `docs/books/ecky-ir/missions/` files.
- Publication: `scripts/ecky_ir_content.ts`, campaign build/source checks, and
  `public/tutorials/` generated artifacts.
- Frontend: `src/lib/DocsSite.svelte`, a dedicated mission workbench component, and
  pure mission manifest/evaluator modules under `src/lib/docs/`.
- Tests: new unit tests and `e2e/docs-site.spec.ts` campaign scenarios.
- No Rust, Tauri boundary, database, configuration schema, STL/STEP export, or static
  reference-site redesign is required.
- No source-control staging or commit is part of this change.
