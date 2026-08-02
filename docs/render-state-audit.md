# Render/State Audit

Scope: `src-tauri/src/services/render.rs`, `src-tauri/src/commands/render.rs`,
`src-tauri/src/models.rs`, and direct OCCT runner dispatch.

## Findings

- Global render singleflight map is active-flight scoped, not a result cache.
  `RenderFlightOwner::complete` and `Drop` both remove the exact `(key, flight)`
  entry and notify waiters. Existing tests cover owner cancellation, raw failure,
  retry, imported-STL identity changes, and public overlap. No leak found in this
  path.
- `AppState::render_lock` is held across `render_model_unlocked` and the
  selector-rebind retry. This serializes the full kernel operation plus artifact
  finalization/cache eviction. Narrowing it now risks concurrent OCCT/FreeCAD
  access and changes the tested overlap contract. Treat as deliberate broad
  serialization until kernel ownership is split.
- Render config is snapshotted before waiting on `render_lock`; this avoids a
  request changing identity while queued. Keep this ordering invariant.
- Optional direct OCCT is unambiguous: compile/runtime absence returns `Ok(None)`
  and dispatch continues. Required native OCCT errors use explicit validation
  messages; runner rejection does not request generated-C++ fallback. Existing
  runner tests pin this boundary.
- Several non-render state mutex accesses still use bare `unwrap` (for example
  `commands/render.rs` FreeCAD library search and `mcp/runtime.rs` runtime
  registry). Poison recovery is inconsistent, but changing policy is cross-cutting
  and not a low-risk render fix.

## Next safe slice

Add a focused test seam around `render_lock` ownership, then move only cache
eviction and non-kernel artifact bookkeeping outside the lock if the kernel API
can prove those operations are independent. Separately inventory bare mutex
`unwrap` calls and choose one documented poison policy before replacing them.
