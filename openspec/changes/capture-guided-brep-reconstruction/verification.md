# Verification: Capture-Guided BRep Reconstruction

## Deterministic fixture

- Source: `e2e/fixtures/partial-symmetric-mechanical-insert.ts`.
- Observed geometry: four triangles covering `x >= 0, y >= 0`.
- Calibration span: 40 mm.
- Expected completed BRep envelope: 80 x 60 x 18 mm.
- Completion contract: one authored quarter plus explicit X/Y mirrors.

## Browser evidence

Command:

```sh
npx playwright test e2e/capture-guided-brep-reconstruction.spec.ts
```

Result: 7 passed.

- Exact triangle picks persist as 11 numbered digest-bound landmarks.
- Landmark edit/delete/undo stays in guide draft; no model version is written.
- Ordered profile and binding selector edits invalidate readiness until revalidated.
- Insufficient evidence, degenerate frame, stale mesh, and source divergence block handoff with raw reasons.
- Completed comparison resolves exact owning thread/message/model identity.
- Reference scan and generated BRep load as separate meshes with independent visibility/opacity.
- Display-only deviation samples render in guide-local coordinates with tolerance colors and a separate toggle.
- Observed aggregate metrics, exact target residual, and inferred/unverified regions remain distinct.
- Computed browser styles prove hidden overflow, square guide controls, and inherited Tactical Midnight primary/secondary tokens.

## Known deterministic reconstruction gaps

Current proof covers exact anchors, calibration/frame, axis/plane fits, ordered
profile evidence, bounded neighborhood uncertainty, agent handoff, exact BRep
correspondence, and observed-region deviation. Primitive fit kernels exist;
only line/plane/circle/cylinder candidates are wired into guide recomputation.
Surface segmentation/adjacency, line/arc/spline profile reconstruction, a
complete dimension/constraint graph, bounded feature-plan synthesis, bypass
readiness, and candidate-only agent enforcement remain open. Tasks 11.2–11.10
remain incomplete; this change is not fully complete until those proof gates
pass.

## Frontend evidence

Commands:

```sh
npm run test:unit
npm run typecheck
```

Results: 367 tests passed; zero Svelte/TypeScript errors or warnings.

## Backend and exact-runtime evidence

Commands:

```sh
cd src-tauri
CARGO_INCREMENTAL=0 cargo test capture_ --lib
CARGO_INCREMENTAL=0 cargo test live_quarter_guided_source_uses_two_mirrors_and_diagnostics_leave_artifacts_immutable --lib -- --nocapture
CARGO_INCREMENTAL=0 cargo check
```

Results: 44 capture tests passed; live OCCT quarter test passed; cargo check passed.

Live OCCT proof:

- Source contains one authored quarter primitive and two explicit mirror operations.
- Exact runtime returns a valid solid.
- Deviation samples report `observedRegionOnly`.
- Model STL bytes, STEP bytes, and artifact-bundle digest remain unchanged after diagnostics.

## Change integrity

Commands:

```sh
git diff --check
openspec validate capture-guided-brep-reconstruction --strict
```

Results: clean diff; strict OpenSpec validation passed.
