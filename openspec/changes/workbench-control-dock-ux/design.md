## Context

Current dock uses eight 44px icon buttons inside a 58px shell. Supplied Retina
capture matches this CSS geometry. Live route inspection at 1280×720 found:

- Dock: x=437, y=646, 406×58.
- Default Dialogue window after clamping: x=300, y=460, 980×260.
- Dock overlaps Dialogue by its full 58px height, covering composer/actions.
- Window launchers expose visible state only through CSS class/color; no
  `aria-pressed` exists. Settings has no active class.
- Dock is a generic container with eight tab stops, not a named toolbar.
- DRAW uses native `disabled`, making its reason difficult to discover by keyboard.
- Archived grouping intent and implementation differ: Draw is mixed into primary
  windows; Projects and Docs are marked utility.
- CSS pseudo-elements carry labels. They are pointer/focus popovers, not durable
  visible content.

Constraints: Tactical Midnight, square borders, bronze `--primary` / `--secondary`,
major-container `overflow: hidden`, per-thread window persistence, dedicated terminal
surface, no separate agent status bar.

Research basis:

- WAI toolbar pattern: named toolbar, one Tab stop, arrow navigation, Home/End.
- WAI toolbar example: `aria-pressed` for toggles; focusable `aria-disabled` controls
  when disabled-function discoverability matters; labels available on focus and hover.
- Existing app BDD rule: real route, happy path plus failure/pending state.

## Goals / Non-Goals

**Goals:**

- Make tool identity, grouping, state, and activation predictable.
- Prevent dock/window collisions at every supported viewport.
- Keep fast pointer use while making keyboard and assistive use first-class.
- Redraw ambiguous icons inside the repo using one deterministic vector grammar.
- Preserve current window contents, data flow, persistence, and theme.

**Non-Goals:**

- New workbench tools or new floating-window framework.
- Raster/generated icon assets or third-party icon dependency.
- New agent-status strip, terminal output in app logs, or navigation redesign outside
  workbench.
- Changing Projects-owned creation.

## Decisions

### 1. Task grouping and order

Order normal window launchers by work sequence:

`Projects · Parameters · Dialogue · Code · Docs · Library | Draw · Terminal? · Settings`

Left group opens persistent workspace windows. Right group contains transient modes,
conditional terminal, and app configuration. Divider has separator semantics and no
focus stop.

Alternative: preserve current order. Rejected because current divider communicates a
false primary/utility model and places Projects after editing actions.

### 2. Repo-native icon component

Create one dock metadata model and one icon component. Every icon uses the same 24×24
view box, square line caps/joins, optical 20×20 bounds, 1.75–2px stroke, and no emoji or
raster. Redraw meanings:

- Projects: folder plus stacked project leaf.
- Parameters: three constraint sliders.
- Dialogue: speech panel with baseline.
- Code: source brackets plus slash.
- Docs: open reference book, not upload page.
- Draw: pencil/annotation stroke.
- Terminal: framed prompt mark.
- Settings: toothed gear, not sun/brightness.

Alternative: generated images. Rejected: blurry across scale, harder active-state
recoloring, larger asset pipeline, weaker consistency with established inline vector UI.

### 3. Explicit control state model

Every dock control derives one state: `closed`, `open`, `focused`, `activeMode`,
`disabled`, `busy`, or `attention`. Window buttons set `aria-pressed` from visibility;
Draw sets it from draw mode. `data-state` drives visuals:

- open: primary border and text;
- focused/activeMode: primary fill and border;
- busy: accessible state copy;
- attention: secondary border/glow plus accessible copy;
- disabled: reduced contrast plus `aria-disabled="true"`, still focusable.

ARIA state remains authoritative. The dock intentionally avoids extra inline/bottom
state glyphs; they created clipped visual noise at the control edge. Settings uses the
same visible/focused logic as other windows.

Alternative: CSS classes per button. Rejected because current drift already left
Settings without active styling and semantics.

### 4. Deterministic launcher mechanics

For window launchers:

1. hidden → open, clamp, focus;
2. visible but not focused → bring to front only;
3. visible and focused → close on activation.

Draw remains a direct mode toggle. Terminal attention activation opens/focuses terminal
and clears attention only when existing terminal policy says input was handled.

This prevents an attempt to retrieve a window from closing it because it happened to be
visible behind another window.

### 5. Toolbar keyboard contract

Dock root becomes `role="toolbar"`, `aria-label="Workbench tools"`, horizontal. Roving
tabindex leaves one enabled/focusable entry in page Tab order. Left/Right wrap; Home/End
jump; Enter/Space activate. Disabled-but-discoverable controls participate in arrow
navigation but do not activate. Escape dismisses a visible tooltip.

Use real DOM label/tooltip elements; remove pseudo-element label content. At wide
viewports, short labels remain visible. At compact widths, labels collapse while
focused/open control name remains visible and every control retains accessible name.

### 6. Dock-safe workspace geometry

Measure dock shell with `ResizeObserver` and expose a bottom safe inset:

`dock height + bottom offset + interaction gap`.

Pass safe insets into `fitRectToViewport`; use available work area, not full viewport,
for open, restore, drag-end, double-click fit, and resize. Existing saved rectangles are
clamped at runtime and only persisted through current layout flow. Dock and major groups
use `overflow: hidden`; tooltip layer renders outside via a dedicated overlay host.

Alternative: raise window z-index above dock. Rejected because it hides navigation and
does not fix covered window controls. Alternative: auto-hide dock. Rejected because it
damages navigation predictability.

### 7. BDD delivery slices

Outer tests land first:

1. toolbar semantics, order, labels, keyboard navigation;
2. open → focus → close mechanics and state exposure;
3. Dialogue composer plus one pending/disabled control remains unobscured;
4. compact-width containment and tooltip behavior;
5. Projects `+ NEW` regression.

Unit loops cover metadata/state reduction, roving focus, and inset-aware geometry.

## Risks / Trade-offs

- [More dock height on wide label mode] → Reserve measured inset; compact label mode
  preserves viewport area.
- [Roving tabindex surprises existing tests] → Keep accessible names stable; update
  tests to assert toolbar behavior, not eight independent Tab stops.
- [Restored layouts shift upward] → Clamp only when collision exists; preserve saved
  rectangle data and current remember-layout contract.
- [Conditional terminal changes focus indices] → Derive focus order from rendered,
  enabled metadata each time.
- [Tooltip clipping conflicts with overflow mandate] → Render tooltip in existing
  overlay layer, not outside dock overflow boundary.

## Migration Plan

1. Add failing Playwright scenarios.
2. Introduce metadata/icon component without behavior change.
3. Add state reducer and launcher activation rule.
4. Add toolbar focus controller and DOM tooltips.
5. Add measured safe inset to window geometry.
6. Run targeted unit/e2e, full frontend suite, then real-route visual proof.

Rollback is frontend-only: old dock markup can return without data migration. Saved
window layout schema remains unchanged.

## Open Questions

- Final wide-mode short labels after visual QA: full names versus `FILES`, `PARAM`,
  `TALK`, `SKETCH`, `CODE`, `DOCS`, `DRAW`, `TERM`, `SET`.

## References

- https://www.w3.org/WAI/ARIA/apg/patterns/toolbar/
- https://www.w3.org/WAI/ARIA/apg/patterns/toolbar/examples/toolbar/
