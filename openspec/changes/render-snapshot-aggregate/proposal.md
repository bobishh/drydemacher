# Proposal: Render Snapshot Aggregate

## Intent

Give each authoring draft an actor that serializes state transitions while
parallel render and verification workers produce immutable results. Make one
immutable render snapshot the authority for source, effective parameters,
runtime artifact, manifest, and verification that belong together. Stop
reconstructing that fact from agent session caches, saved messages,
`last_design.json`, frontend working-copy state, parameter-panel state, and the
viewport runtime independently.

The immediate production symptom is parameter drift: an MCP parameter preview
renders one bore or height, while verifier diagnostics, Apply, or restored UI
can report or use another value. The root cause is not render math. One logical
aggregate has several writable state holders, no mailbox owner, and polymorphic
identifiers.

## Findings

- `DesignOutput.initialParams` is copied into `workingCopy.params` and
  `paramPanelState.params`; artifact and manifest live in a separate `session`
  store.
- One agent preview event performs seven ordered frontend mutations before
  writing another reconstructed `LastDesignSnapshot`.
- `SessionRenderPreview` is copied into a process-global map, `agent_drafts`,
  `AppState.last_snapshot`, `last_design.json`, a Tauri event, and frontend
  stores. None of these copies declares cache versus authority.
- `messageId` can mean saved history message, draft preview UUID, or the saved
  base message intercepted by the current draft.
- MCP verification receives a preview UUID, fails to find a saved message, and
  silently falls back to source defaults for diagnostic parameters.
- Preview structural feedback is not the same fact as explicit authored
  verification, but both are represented by the same feedback status. Commit
  therefore cannot prove that the exact snapshot was explicitly verified.
- Snapshot file writing/building exists in both `commands/session.rs` and
  `services/session.rs`.
- Frontend request queue permits concurrent work, Tokio executes render work on
  workers, and MCP calls can overlap, but preview publication has no monotonic
  draft revision. An older render can finish after a newer intent and overwrite
  it.

## Scope

- Define immutable `RenderSnapshot` identity over exact render inputs and
  outputs.
- Define one `AuthoringActor` per active draft/thread with monotonic revision and
  a mailbox-owned state machine.
- Keep render/verify workers parallel; publish their result only when actor and
  requested revision still match.
- Make draft and saved-version references distinct tagged contracts.
- Bind verification and commit to snapshot identity and artifact digest.
- Replace frontend multi-store ownership with one snapshot store plus derived
  projections.
- Treat memory maps, `last_design.json`, and UI projections as disposable
  caches or pointers, never authorities.
- First applied slice: verifier diagnostics resolve effective parameters from
  the matching preview draft before considering saved-version data or source
  defaults.

## Out Of Scope

- Rewriting render engines or `.ecky` compilation.
- Replacing history storage.
- Deleting compatibility stores in the first slice.
- Changing STL or STEP geometry.
- Making debug overlays exportable.

## Success Criteria

- A snapshot has one stable identity covering source digest, effective parameter
  digest, backend, artifact content hash, and manifest identity.
- Preview, verifier, Apply, commit, viewport, and restored session name the same
  snapshot or return an explicit stale/mismatch error.
- Explicit saved-version refs cannot be intercepted by an active draft.
- Frontend consumers cannot observe source/params from one snapshot and runtime
  artifact from another.
- Caches can be cleared without changing domain resolution.
- Preview verification diagnostics report the parameters used for that preview.
- Out-of-order worker completion cannot replace a newer draft revision.
