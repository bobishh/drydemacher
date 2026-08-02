## 1. Outer Red: Observable Dock Behavior

- [x] 1.1 Add Playwright test for named toolbar, task-group order, stable full names, pressed state, and one roving Tab stop; run it and confirm expected failure.
- [x] 1.2 Add Playwright test for hidden-open, background-focus, focused-close launcher sequence; run it and confirm current second activation closes the background window.
- [x] 1.3 Add Playwright geometry assertion that Dialogue composer/actions do not intersect dock; run it and confirm current 58px overlap.
- [x] 1.4 Add Playwright disabled Draw test proving focusable raw reason and no action; run it and confirm native disabled behavior fails discoverability.
- [x] 1.5 Add compact-width Playwright case for containment, label/tooltip identity, and Projects `+ NEW` regression; confirm intended red reason.

## 2. Inner Loop: Metadata, State, And Icons

- [x] 2.1 Add failing unit test for dock metadata order, groups, stable accessible names, short labels, and icon ids.
- [x] 2.2 Implement typed dock metadata and one repo-native `DockIcon` component with consistent 24×24 square-stroke grammar.
- [x] 2.3 Add failing unit tests for `closed/open/focused/activeMode/disabled/busy/attention` state reduction and launcher activation outcomes.
- [x] 2.4 Implement pure dock state and launcher-action helpers; refactor repeated per-button conditionals while unit tests remain green.
- [x] 2.5 Inspect rendered dock icons and revise ambiguous Docs, Settings, and Terminal glyphs; detached Sketch remains outside the dock.

## 3. Inner Loop: Keyboard And Labels

- [x] 3.1 Add failing unit tests for rendered-control roving focus, wrap, Home/End, and conditional Terminal changes.
- [x] 3.2 Implement toolbar focus controller and `role=toolbar`/label/orientation semantics.
- [x] 3.3 Replace pseudo-element labels with real DOM short labels and overlay-host tooltips available on focus and hover.
- [x] 3.4 Use `aria-pressed`, `aria-disabled`, and accessible busy/attention copy for every applicable control; omit clipped inline/bottom marker glyphs.
- [x] 3.5 Re-run targeted toolbar Playwright test and complete red-green-refactor cycle.

## 4. Inner Loop: Dock-Safe Window Geometry

- [x] 4.1 Add failing `fitRectToViewport` tests for bottom safe inset, restored collision, drag-end clamp, and small available work area.
- [x] 4.2 Extend window geometry with safe insets without changing persisted layout schema.
- [x] 4.3 Measure dock shell with `ResizeObserver`; feed bottom inset into open, restore, drag, resize, and double-click fit paths.
- [x] 4.4 Keep dock/groups `overflow: hidden`; route tooltip visuals through dedicated overlay host.
- [x] 4.5 Re-run Dialogue overlap Playwright test and confirm composer/action rectangle stays outside dock rectangle.

## 5. Integration Green And Visual Proof

- [x] 5.1 Re-run all new dock/layout Playwright specs; verify happy paths plus disabled and attention/pending states.
- [x] 5.2 Run relevant window store/geometry and dock metadata unit tests, then `npm run test:unit`.
- [x] 5.3 Run `npm run typecheck` and targeted existing `e2e/app.spec.ts` plus `e2e/layout.spec.ts` regressions.
- [x] 5.4 Inspect real route at normal and compact widths in browser; capture final geometry/state evidence and confirm Tactical Midnight/square-border parity.
- [ ] 5.5 Run full Playwright suite before any requested commit-worthy checkpoint; do not stage or commit unless user asks.
