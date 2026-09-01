# Proposal: Campaign Project Learning Flow

## Why

Campaigns currently look like a tutorial shell without tutorial substance.
The app loads one-line instructions from JSON, ignores the mission Markdown,
opens source steps with an empty preview, rerenders on incidental lifecycle
events, accepts stale drafts after lesson changes, and forgets the active
campaign surface on reload. Mission 1 then jumps from changing a translation
and Boolean to completing a multi-part configurable joint without teaching the
forms used by the challenge.

## What changes

- Make mission Markdown the canonical step prose. Keep the EDN manifest as
  ordering, source, reveal, and acceptance metadata only.
- Load packaged campaign definitions through a Rust service. Frontend receives
  typed summaries and one requested step payload; it never imports the campaign
  corpus or parses EDN.
- Treat campaigns as persistent Projects surfaces, not transient app views or
  design threads.
- Present one ordered step at a time. No clickable whole-book table of contents,
  fake download controls, local repository paths, or redundant open/apply flow.
- Bundle a verified canonical preview for every source-bearing step. Opening a
  step reads that artifact by canonical source digest without running a kernel.
- Keep `RENDER` explicit for edited source. Reuse successful edited-source
  artifacts by content/runtime identity.
- Version draft overrides by canonical source digest so changed lessons never
  reopen stale text.
- Restore active campaign, active step, and visible Projects window after reload.
- Route draft saves, navigation, and challenge checks through one atomic Rust
  transition intent over canonical run and packaged step state.
- Route campaign start/resume through one Rust projection intent. Rust selects
  canonical title, first step, definition version, current step, and active
  Project navigation from caller-supplied definition/run identity.
- Rewrite all six missions as worked-example -> explained mechanism -> bounded
  edit -> Core IR acceptance sequences. A challenge may use only forms already
  introduced by an earlier step in that mission or a named prerequisite.

## Out of scope

- Campaign chat threads, folders, model versions, or agent message history.
- Storing campaign status in `config.edn`, localStorage, or a project folder.
  Campaign definition EDN is repository content; learner state remains DB data.
- Running every canonical render when the learner opens a campaign.
- A public marketplace or remote campaign sharing protocol.
- Changing Ecky language semantics to accommodate tutorial content.
- Frontend `import.meta.glob` ownership of campaign Markdown, Ecky source, or
  preview assets.

## Success criteria

- Every visible step contains file-backed prose or a deliberate short action;
  no one-line text is duplicated across header and body.
- Every source-bearing template displays its bundled STL immediately.
- Opening, revisiting, typing, continuing, or reloading never starts a render.
- `RENDER` starts one render only for the current edited source; a warm identical
  edit returns the verified cache artifact without kernel execution.
- Reload returns to the same campaign and step. A visible Projects window stays
  visible with its saved geometry.
- Frontend never submits replacement completion, challenge-pass, current-step,
  or draft-override collections.
- Mission 1 teaches `params`, `let*`, `build`, `shape`, `result`, placement,
  `compound`, `union`, multiple `part` forms, shared clearance, and `if` before
  asking for the configurable enclosure joint.
- Six missions together cover the intended language corpus through their
  working models, not through a detached operation dump.
