# Tasks: Landing Parametric Case Workbench

## 1. OpenSpec

- [x] 1.1 Add proposal, design, tasks, and capability spec.
- [x] 1.2 Validate the change with `openspec validate landing-parametric-case-workbench --strict`.

## 2. Outer BDD

- [x] 2.1 Add Playwright coverage for the single-preset vignette and
  real STL readiness.
- [x] 2.2 Add Playwright coverage for full read-only source modal,
  keyboard close/focus restoration, copy, and source download.
- [x] 2.3 Add Playwright coverage for visible STL failure and retry.
- [x] 2.4 Add Playwright coverage for 390px overflow boundaries.

## 3. Manifest and shell

- [x] 3.1 Add a typed, truthful case showcase manifest using saved iPhone 17e
  STL/source artifact pairs.
- [x] 3.2 Add one bounded landing-only pattern vignette with a dominant viewer.
- [x] 3.3 Render a static device label for one preset and a native selector only
  when multiple complete presets exist. `selectedId` and the derived selected
  manifest record drive every selected-preset surface.
- [x] 3.4 Render only real selected-preset download actions.

## 4. Source inspector

- [x] 4.1 Add a pure shared Ecky lexer that returns ordered token spans/classes
  without importing CodeMirror or DOM APIs; cover representative comments,
  forms, numbers, strings, symbols, and nesting.
- [x] 4.2 Refactor `src/lib/eckyLanguage.ts` into a CodeMirror-only adapter over
  the pure lexer; preserve existing app token classes and desktop behavior.
- [x] 4.3 Replace the landing CodeMirror source surface with a static renderer
  driven by the pure lexer. Preserve complete source, line numbers, static
  syntax classes, scrolling, copy, download, Escape/backdrop close, and focus
  restoration.
- [x] 4.4 Remove CodeMirror and Lezer from landing source imports,
  `sites/landing/package.json`, lockfile, and Vite aliases/dedupe. Prove a
  clean landing install has no editor-runtime dependency.
- [x] 4.5 Add landing/browser coverage: CODE shows `(model)` in static markup,
  no editable textbox/editor runtime exists, and copy/download still return
  exact canonical source.

## 5. Viewer lifecycle

- [x] 5.1 Make `StlViewer` responsive while preserving combined assembly fit.
- [x] 5.2 Reset loading state and dispose prior geometry on parts changes.
- [x] 5.3 Ignore stale loader callbacks after cleanup.
- [x] 5.4 Show visible asset-specific failure state and retry.

## 6. Verification

- [x] 6.1 Run the full landing Playwright suite after the static renderer lands:
  12/12 green.
- [x] 6.2 Run `npm ci` and `npm run build` from `sites/landing` after removing
  editor-runtime dependencies. No CodeMirror/Lezer tree remains; build is
  warning-free at 592.08 kB JS / 160.78 kB gzip.
- [x] 6.3 Run desktop and 390px browser proof on the real route: real viewer,
  static source markup containing `(model`, no console errors, and zero
  body/root horizontal overflow at 390×844.
- [x] 6.4 Run `cd src-tauri && cargo check`.

## 7. Artifact-first pattern pivot

- [x] 7.1 Add outer RED coverage for hostile copy, two real patterns, three
  earlier versions, `SEE CODE`, and removal of fake dialogue/badges.
- [x] 7.2 Replace phone-preset/transcript manifest data with five provenance-
  backed iPhone 17e variant records.
- [x] 7.3 Replace session/proof/metadata chrome with pattern controls, dominant
  viewer, and earlier-version strip.
- [x] 7.4 Keep static source modal exact for every selected variant and rename
  the trigger to `SEE CODE`.
- [x] 7.5 Run full landing tests/build, desktop/mobile browser proof, strict
  OpenSpec validation, `cargo check`, deploy, and production browser proof.

## 8. A+ landing hardening

- [x] 8.1 Add outer RED coverage for unavailable-release truth, compact mobile
  navigation, viewport-native source dialog, social/crawler metadata, and
  reduced-motion behavior.
- [x] 8.2 Move the real case workbench into the hero and remove app Download and
  Releases actions while no packaged release exists.
- [x] 8.3 Convert source overlay to a native top-layer dialog with focus return,
  page-scroll locking, viewport bounds, and contained source scrolling.
- [x] 8.4 Stop idle viewer rendering; add reduced-motion behavior, discovery
  metadata, crawler assets, and production response-security headers.
- [x] 8.5 Run full landing/docs suites, build, browser proof, strict OpenSpec
  validation, `cargo check`, deploy, and production smoke proof.
