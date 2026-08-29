# Design: Landing Parametric Case Workbench

## Goal

Make the case section an artifact-first pattern comparison. Keep geometry and
saved versions primary. Remove app-simulation chrome that implies dialogue or
runtime state the landing does not possess.

## Artifact model

Each published case variant is one typed, file-backed manifest record:

- stable id and display label;
- canonical `.ecky` source text and download URL;
- one or more real STL part URLs with labels and viewer colors;
- kind (`pattern` or `earlier`), display label, and short factual note;
- exact saved artifact id for provenance.

Five records bind saved iPhone 17e case-body STLs to their exact saved source.
Two records populate the current pattern selector. Three populate the earlier
version strip. Incomplete records are not presented.

## Variables

- Content format: `.ecky`, STL, typed TypeScript manifest.
- Storage: canonical source under `model-runtime/examples`; landing STL/bundle
  assets under `sites/landing/src/models`; presentation under
  `sites/landing/src/showcase`.
- Serving path: landing `/`, section `/#case-study`, Vite-generated static asset
  URLs.
- Backend ownership: none. First release is static and performs no generation.
- Frontend ownership: pattern/version selection, viewer lifecycle, modal lifecycle,
  clipboard, and download links.
- Editing model: full source is read-only; no Apply or Commit controls.
- Testing surface: landing Playwright config on the real Vite route plus browser
  visual proof.
- Export format: existing STL parts, canonical `.ecky`, optional prebuilt ZIP.
- Runtime constraints: standalone landing package, responsive WebGL canvas,
  viewport-bounded native dialog, no Tauri imports.

### Shared lexer and standalone source renderer

The canonical lexer SHALL become a pure Ecky tokenization module: source text
in, ordered token spans/classes out. It imports neither CodeMirror nor DOM APIs.

- Desktop app: `eckyLanguage.ts` becomes a CodeMirror adapter over the pure
  lexer. Existing desktop highlighting behavior stays source-compatible.
- Landing: static source renderer consumes the same pure tokens and emits
  escaped markup with line numbers; it is not contenteditable and creates no
  editor runtime.

The landing package SHALL not declare or resolve `codemirror`,
`@codemirror/*`, or `@lezer/*`. Its clean install and build are independent of
the desktop editor dependency graph.

## Decision

### One bounded artifact frame

The section renders one square-cornered frame with fixed device identity,
pattern selection, dominant viewer, and compact earlier-version strip. It does
not render chat, dialogue, app status badges, parameter/verification/triangle
counters, docks, prompts, or window-manager controls.

### Honest pattern and version affordances

Device label always reads iPhone 17e. Two current pattern buttons update STL,
source, labels, and downloads atomically. Three earlier-version buttons use the
same selection contract. No option exists without a real STL/source pair.

### Read-only macro inspector

`SEE CODE` opens a native top-layer dialog bounded by the viewport. It displays the entire raw
canonical source with line numbers and syntax classes emitted by the shared
pure Ecky lexer. The landing renderer is static markup, never a CodeMirror
editor. Available actions are Close, Copy Code, and Download `.ecky`. Escape
and backdrop close restore focus to `SEE CODE`. While open, page scrolling is
locked and keyboard focus stays inside the dialog.

### Truthful hero and release boundary

The real iPhone 17e workbench is the hero proof, immediately following concise
pre-release copy. Navigation contains documentation and source only. The page
must not render an app Download or Releases action while the repository has no
packaged release. Real case STL and `.ecky` downloads remain available.

### Runtime and delivery quality

WebGL viewers render on initialization, resize, interaction, or asset-state
changes rather than running an idle animation loop. Reduced-motion preference
stops mascot animation and disables smooth scrolling. Production HTML includes
canonical/social metadata; crawler files are served with correct content types;
Nginx supplies baseline CSP, HSTS, frame, MIME, referrer, and permissions
headers.

The landing may reuse the pure lexer but must not reuse `CodeModal.svelte` or
the CodeMirror adapter, which own mutation/Tauri behavior and editor runtime.

### Real downloads only

Download anchors are rendered directly from manifest assets and include the
`download` attribute. Optional insert or bundle actions are absent when their
files are absent. The client never synthesizes an export or claims a download
will be generated later.

### Viewer state machine

The viewer owns `loading`, `ready`, and `error` states per parts key. Switching
parts resets state and disposes the prior scene. Late callbacks from stale loads
cannot update the active scene. Failure removes the pending indicator and shows
the failed asset name/body visibly. A retry action starts a fresh load.

## Rejected paths

- Full recreation of the desktop screenshot: exposes current UI density and
  makes every surface visually primary.
- Direct reuse of `Window.svelte`: fixed positioning, drag/resize, and window
  store behavior are false affordances on the landing page.
- Direct reuse of `PromptPanel.svelte`: thread state, attachments, voice, queue,
  and persistence exceed the static showcase boundary.
- Fake phone options, duplicated STL files, fake bundles, or unverified status.
- Handwritten source snippets or JS-shaped pseudo-Ecky.
- In-browser Ecky compiler or render service in this change.

## Proof plan

1. Outer red Playwright test: current single-preset vignette, real mesh, no fake
   phone selector.
2. Outer red Playwright test: `CODE` opens full source; copy/download and
   Escape/focus behavior work.
3. Outer red Playwright test: failed STL exits pending and exposes raw asset
   context with retry.
4. Outer red Playwright test: 390px layout has no horizontal overflow and native
   dialog stays inside the viewport with focus containment and scroll locking.
5. Add lexer unit coverage proving static-token output equals adapter-token
   classification for representative Ecky source.
6. Add a landing acceptance case proving `CODE` displays `(model)` in static
   source markup with no editable textbox/editor runtime.
7. Run full landing Playwright suite, clean standalone install/build, desktop
   browser proof, and mobile browser proof.
8. Prove unavailable release CTAs stay absent, mobile navigation stays one line,
   social/crawler metadata resolves, and reduced-motion behavior is static.

## Follow-up: actual app UI

The screenshot exposes a separate product problem: overlapping floating windows
give Dialogue, Macro Inspector, viewer, dock, and supporting controls equal
visual weight. That redesign needs its own OpenSpec after this landing change.
The landing vignette deliberately edits that density instead of declaring it the
target application layout.
