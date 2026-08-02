# Tasks: Frontend Decomposition

## T0 — Compile gate (from plan Slice 0)
- [x] Clean baseline: `cargo check` + `npm run typecheck` (drive the 32 standing TS errors to 0 — they are oversight/drift, not intentional) + `npm run test:unit` green before any cut.
  Evidence reused from the fresh final integration window: `npm run
  typecheck` completed with 0 errors / 0 warnings, `npm run test:unit`
  completed 383/383, and the later multipart-export slice re-ran
  `cd src-tauri && cargo check` successfully after the newest Rust edits.
  Full suites were not repeated.

## T1 — Component-test harness
- [x] Add vitest + @testing-library/svelte + jsdom/happy-dom; wire a `test:component` script (and into CI).
- [x] Prove the harness: one component mounted in isolation with a passing render assertion.
  Evidence: added `vitest@^4.1.10` + `@testing-library/svelte@^5.4.2` + `jsdom@^29.1.1`
  as devDependencies; added the `npm run test:component` (`vitest run`) script and a
  "Run Frontend Component Tests" step in the `test-frontend` CI job. BDD red-first:
  `npm run test:component` failed with `vitest: command not found` before install,
  then green 3/3 after. Harness config is a standalone `vitest.config.ts` (jsdom,
  scoped `include: tests/component/**` so it never grabs the tsx unit suite or
  e2e). Proof component: `tests/component/PreviewFrame.test.ts` mounts
  `src/lib/PreviewFrame.svelte` (smallest pure-presentational component) and
  asserts the empty/ready/loading prop-driven render branches.

## T2 — Slice 1: ViewportWorkspace extraction (from plan)
- [x] Component test for `ViewportWorkspace.svelte` (fork/export/code action presence + disabled states).
  Evidence: BDD red-first — `npx vitest run tests/component/ViewportWorkspace.test.ts` failed with
  `Failed to resolve import "../../src/lib/workbench/ViewportWorkspace.svelte"` (component did not
  exist yet). Added the minimal presentational seam `src/lib/workbench/ViewportWorkspace.svelte`
  (no Tauri/stores/Viewer; callbacks + disabled/visibility props only) so the test can mount it.
  Disabled-state logic mirrors App.svelte's inline action strip verbatim: fork=`busy`,
  export=`!canExport || busy || hasSketchPreview` (visible only when `showExport`), code=`busy`.
  Green: focused file 9/9, then full `npm run test:component` 12/12 (2 files). App.svelte untouched;
  wiring the seam into App.svelte + migrating `qa.spec.ts` assertions remain the next T2 tasks.
- [x] Extract per plan; keep state/handlers in `App.svelte`.
  Evidence: `App.svelte` composes `ViewportWorkspace` for the viewport overlay and
  passes only derived state and callbacks. The action component remains free of
  Tauri/store/viewer ownership. Browser proof: the fork confirmation and export
  chooser flows pass against the real Vite route.
- [x] Move single-component presence assertions out of `qa.spec.ts` into the component test; e2e keeps cross-domain flow only.
  Evidence: `tests/component/ViewportWorkspace.test.ts` owns the isolated action
  presence/disabled-state contract. `e2e/qa.spec.ts` no longer asserts EXPORT
  visibility before clicking; it verifies the cross-domain chooser and export
  outcomes. Its dialog mock now matches the current Tauri `plugin:dialog|confirm`
  command, keeping fork flows executable.

## T3 — Following slices (viewerRuntime, agentRuntime, modelIo, WorkbenchWindows, DialogueWindowContent)
- [x] Each slice ships component/unit tests for its seam; presence/wiring assertions migrate out of e2e per `frontend-testing` spec.
  Progress — viewerRuntime: BDD red was `viewerRuntime.test.ts` failing because
  `./viewerRuntime` did not exist. `createViewerLoadRuntime` now owns the
  visible/hidden load waiters, nonce settlement, timeout cleanup, and raw load
  failure rejection; App owns UI callbacks and recovery policy. Focused unit
  proof: 3/3. Real-route proof: `qa.spec.ts` generated-model happy path and
  render-pending transmutation path 2/2. Aggregate remains open pending the
  remaining four named slices.
  Progress — agentRuntime: BDD red was `agentRuntime.test.ts` failing because
  `./agentRuntime` did not exist. `createAgentRuntime` now owns active-thread
  polling guard/refresh and wake-stop-restart target routing; App retains agent
  UI state, terminal protocol, and dialogue wiring. Focused unit proof: 2/2.
  Real-route proof: passive MCP queue happy path and raw queue-error state 2/2.
  Progress — modelIo: BDD red was `modelIo.test.ts` failing because
  `./modelIo` did not exist. `createModelIo` now owns 3MF, multipart STL, STL,
  STEP, and FCStd export routing plus raw provider-error presentation; App owns
  chooser visibility and derived export inputs. Focused unit proof: 1/1.
  Real-route proof: STEP export happy path plus two unavailable/error states 3/3.
  Progress — DialogueWindowContent: BDD red was the component test failing to
  resolve `DialogueWindowContent.svelte`. The extracted component owns the
  remember-layout toolbar, complete `PromptPanel` render/wiring block, keyed
  thread remount, and dialogue layout CSS. App retains all state and callbacks
  through typed props plus the bindable active version. Focused component proof:
  2/2. Typecheck: 0 errors / 0 warnings. Real-route proof: passive MCP queue
  happy, slow-pending, and raw queue-error paths 3/3.
  Progress — WorkbenchWindows: BDD red was the component test failing to resolve
  `WorkbenchWindows.svelte`. The extracted component now owns the complete
  bottom dock plus Projects, Library, Params, Settings, Activity, Dialogue,
  Docs, and Terminal window shells. App supplies typed state/callback props and
  named content snippets, preserving domain ownership. Component proof: 2/2;
  full component suite: 31/31; typecheck: 0 errors / 0 warnings. Real-route
  proof: dock open/focus 2/2 and pending/raw-error flows 2/2. Pure dock-presence
  assertions moved from `layout.spec.ts` to the component test.

## T4 — Proof
- [ ] Component suite green in CI alongside unit + e2e.
- [x] Net e2e pure-presence assertions decrease; `App.svelte` trends toward thin shell.
  Evidence: dock presence assertions for PROJECTS/PARAMS/DIALOGUE moved from
  `layout.spec.ts` into `WorkbenchWindows.test.ts`; e2e retains viewport and
  window-store behavior. Current cumulative `App.svelte` worktree diff is 813
  deletions / 218 additions while the extracted seams own their local markup or
  runtime policy.

## Notes
- Slice map and target module list: `docs/app-svelte-decomposition-plan.md`.
- Recorded because button-name e2e checks and standing type errors are oversights
  under active code churn, not a chosen tradeoff — fold the cleanup into the cuts.
