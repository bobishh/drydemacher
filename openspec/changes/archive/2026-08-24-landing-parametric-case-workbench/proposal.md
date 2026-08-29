# Proposal: Landing Parametric Case Workbench

## Intent

Replace the fake app vignette with a direct, artifact-first iPhone 17e pattern
showcase: real case exports, real prior versions, inspectable `.ecky` source,
and direct downloads. No invented dialogue or pseudo-product telemetry.

## Scope

- Add one bounded, non-draggable workbench frame to the iPhone case section.
- Keep the real STL viewport dominant and interactive.
- Add a manifest-backed pattern surface for one fixed iPhone 17e device. Two
  current pattern exports are selectable; three real earlier exports form a
  compact version strip.
- Add a `SEE CODE` action that opens the full saved Lisp/Ecky source in a
  read-only, syntax-highlighted macro inspector.
- Offer only downloads backed by real static files for the selected preset:
  source, STL parts, and optional bundle.
- Harden the landing STL viewer for responsive layout, preset changes, pending
  state, stale loads, and visible asset-specific failures.
- Lead with the real workbench in the hero and keep navigation compact.
- Remove app-download/release actions until a packaged release exists.
- Add production social metadata, crawler files, reduced-motion behavior, and
  baseline response-security headers.

## Current Truth

The landing publishes two current pattern exports and three earlier exports
copied from saved Ecky history artifacts. Every record pairs one case-body STL
with the exact saved `.ecky` source that produced its version. Device remains
iPhone 17e; selection never pretends to be phone-model selection.
No packaged application release exists. The landing therefore links source and
documentation, while artifact download actions remain limited to real case
files in the showcase.

## Out of Scope

- Redesigning the actual Ecky desktop application.
- Reusing draggable/resizable app windows or Tauri stores on the landing page.
- Live agent chat, prompt submission, voice, attachments, queueing, Apply, or
  Commit behavior.
- Compiling `.ecky` or generating STL in the browser.
- Runtime wall, rim, material, or geometry controls.
- Publishing unverified phone models or synthetic download targets.
- Invented chat/session transcripts, parameter-count badges, verification-count
  badges, or triangle-count marketing copy.

## Approach

Build landing-only Svelte presentation components around a typed static
manifest. Reuse the app's Tactical Midnight tokens and canonical Ecky syntax
lexer, not its backend-connected window components or editor runtime. Extract
the lexical scan and token classes into a pure shared module; keep the app's
CodeMirror language as an adapter over that module, while the landing renders
the same tokens as static markup. Import canonical source as a Vite raw asset
and URL so displayed, copied, and downloaded content cannot drift into
handwritten Svelte text.

## Proof

- Landing Playwright acceptance tests start red and cover the vignette, real
  viewer readiness, source modal, copy/download behavior, keyboard close/focus
  restoration, narrow layout, and visible STL failure state.
- A clean standalone landing install and `npm run build` succeed without
  CodeMirror or `@lezer/*` landing dependencies.
- Browser proof covers desktop and 390px mobile layouts on the real landing
  route.
