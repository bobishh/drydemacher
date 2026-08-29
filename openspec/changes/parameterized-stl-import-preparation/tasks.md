# Tasks: Parameterized STL Import Preparation

## 1. Contract and runtime red

- [ ] 1.1 Add failing compiler integration test for parameterized `import-stl`.
- [ ] 1.2 Add failing mesh-runtime integration test for bounded simplification and
  unchanged source bytes.
- [ ] 1.3 Add failing hybrid integration test for prepared import followed by
  explicit `solidify`.
- [ ] 1.4 Add camelCase frontend / snake_case Rust preparation contracts for the
  later External Shapes surface.

## 2. Language and Core IR

- [ ] 2.1 Add failing compiler tests for optional `import-stl` preparation
  keywords while preserving the one-path signature.
- [ ] 2.2 Require target and max error as a pair; validate target >= 4 and error
  > 0.
- [ ] 2.3 Add parameter-expression support and normalized
  `StlPreparationPolicy` to Core IR.
- [ ] 2.4 Update language surface/reference and lexer coverage for the new import
  keywords.

## 3. Indexed preparation runtime

- [ ] 3.1 Add deterministic in-process error-bounded simplifier dependency and
  version marker.
- [ ] 3.2 Preserve boundaries, protected vertices, winding, components, and
  manifold topology.
- [ ] 3.3 Measure achieved maximum/RMS deviation and reject output above the hard
  bound.
- [ ] 3.4 Return typed `targetNotReached` warning when topology/error prevents the
  requested count.
- [ ] 3.5 Keep raw STL immutable and emit an indexed prepared asset plus digest.

## 4. Source-anchored operation order

- [ ] 4.1 Carry raw mesh plus preparation policy through source-local mesh ops.
- [ ] 4.2 Resolve plane/surface anchors against raw source digest and raw triangle
  indices.
- [ ] 4.3 Apply preparation after Surface Trim boundary insertion/capping and
  protect resulting boundary/cap vertices.
- [ ] 4.4 Add regression proving preparation never invalidates stored raw anchors.

## 5. Cache, progress, and provenance

- [ ] 5.1 Add source/policy/protected-set/algorithm identity to immutable cache
  key.
- [ ] 5.2 Reuse concurrent identical preparation through singleflight.
- [ ] 5.3 Emit visible `import`, `validate`, `prepare`, `preview`, and `apply`
  stages with cancellation.
- [ ] 5.4 Record requested/achieved counts, error, digests, protected counts,
  algorithm version, and cache state in artifact provenance.

## 6. External Shapes Import UI

- [ ] 6.1 Add failing Playwright scenario on the real External Shapes route:
  select imported STL, preview bounded detail, apply, reopen, and observe
  canonical values.
- [ ] 6.2 Add failing Playwright failure scenario for stale source digest and raw
  backend error.
- [ ] 6.3 Add selected-source Detail section using Tactical Midnight tokens and
  bounded overflow.
- [ ] 6.4 Show Original/Prepared asset switch, requested controls, achieved
  metrics, warning, progress, and raw failure.
- [ ] 6.5 Preview without source mutation; Apply AST-patches exact import node with
  thread/message/source/node guards.
- [ ] 6.6 Reopen from canonical source and Reset by removing only preparation
  keywords.
- [ ] 6.7 Keep Crop/Guides anchor authority on raw source and label Prepared
  Preview as an overlay.

## 7. Hybrid and export integration

- [ ] 7.1 Route prepared indexed import directly to mesh Boolean and STL/3MF
  output without `solidify`.
- [ ] 7.2 Route the same prepared indexed asset to explicit `solidify` for BRep
  Boolean/faceted STEP.
- [ ] 7.3 Reject analytic STEP claims for simplified/solidified mesh provenance.
- [ ] 7.4 Verify warm identical renders execute no preparation or kernel work.

## 8. Proof gates

- [ ] 8.1 Run focused compiler/runtime/cache tests.
- [ ] 8.2 Run focused frontend unit tests and External Shapes Playwright scenarios.
- [ ] 8.3 Run `cd src-tauri && cargo check`.
- [ ] 8.4 Run `openspec validate parameterized-stl-import-preparation --strict`.
- [ ] 8.5 Prove one dense donor reaches its requested error bound, remains one
  manifold component, previews visibly, and commits without an external STL
  rewrite.
