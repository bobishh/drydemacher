## Why

The bottom control dock has strong Tactical Midnight styling but weak semantics and
small-window behavior. Icon meaning, grouping, toggle state, focus behavior, and
window/dock collision now make core tools harder to discover and predict than the
underlying functionality warrants.

## What Changes

- Reorder controls by user task: workspace windows together; transient actions and
  utilities together.
- Replace ambiguous inline glyphs with one repo-native, square-stroke SVG icon set.
- Expose persistent button labels without depending on pointer hover; keep compact
  tooltips as secondary detail.
- Represent window and mode toggles with semantic pressed, disabled, busy, and
  attention states. Do not rely on green color alone.
- Make the dock a named horizontal toolbar with roving keyboard focus, arrow-key
  navigation, Home/End support, and visible focus.
- Reserve a dock-safe workspace region. Adapt floating windows at short heights so
  content and close controls never sit behind the dock.
- Define deterministic launcher mechanics: hidden window opens and focuses; visible
  unfocused window focuses; focused window closes only on explicit second activation.
- Preserve Projects-owned `+ NEW`, dedicated agent terminal UX, existing Tactical
  Midnight tokens, square borders, and per-thread window-layout persistence.
- Add browser proof for happy paths plus disabled, pending/attention, keyboard, and
  compact-window states.

## Capabilities

### New Capabilities

- `workbench-window-management`: Dock-aware floating-window focus, activation, and
  compact-workspace behavior.

### Modified Capabilities

- `workbench-navigation`: Clear icon/label hierarchy, semantic state, task grouping,
  keyboard navigation, and responsive dock containment.

## Impact

- Frontend: `src/App.svelte`, dock icon components, `windowStore`, window geometry,
  focus handling, and Tactical Midnight CSS.
- Tests: `e2e/app.spec.ts`, `e2e/layout.spec.ts`, window-store/geometry unit tests,
  and compact viewport browser coverage.
- No backend contract, persistence format, generated raster asset, or database change.
