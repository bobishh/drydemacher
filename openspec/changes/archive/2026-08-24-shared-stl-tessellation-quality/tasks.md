# Tasks: Shared STL Tessellation Quality

Author tests first. Do not stage or commit without explicit user request.

## 1. Baseline and red tests

- [x] 1.1 Locate every Direct OCCT STL meshing policy and document current
  values, generated-source paths, and export consumers.
- [x] 1.2 Add a focused native fixture for sphere radius `3`, cylinder radius
  `2` × `6`, and box `4 × 8 × 6`.
- [x] 1.3 Add red/green regression coverage rejecting the old fixed `0.25 rad`
  policy and requiring the adaptive sphere lower bound.
- [x] 1.4 Add metric helpers/assertions for STL bytes, triangle count, bounds,
  connected components, non-manifold edges, and wall time.

## 2. Shared policy implementation

- [x] 2.1 Add named linear tolerance and adaptive angular policy values using
  `clamp(linear * 5 + 0.005, 0.005, 0.1)`.
- [x] 2.2 Update generated Direct OCCT export source to use the policy.
- [x] 2.3 Update the standalone/native runner path to use the equivalent policy.
- [x] 2.4 Add a drift test proving both paths emit/use the same values.

## 3. Green geometry proof

- [x] 3.1 Make the sphere fixture pass with denser tessellation and preserved
  bounds, one component, and zero non-manifold edges.
- [x] 3.2 Make the cylinder fixture pass without curved-wall regressions.
- [x] 3.3 Make the box fixture pass without unnecessary triangle growth.
- [x] 3.4 Prove multipart STL export preserves the denser source triangles.

## 4. Performance and artifact guardrails

- [x] 4.1 Capture current-policy fixture metrics in the focused native test.
- [x] 4.2 Enforce fixture wall-time ≤ `3 s`, sphere bytes ≤ `100,000`, and
  cylinder bytes ≤ `20,000`; box remains exact `684` bytes.
- [x] 4.3 Enforce exact 12-triangle box tessellation.
- [x] 4.4 Report measured fixture bytes, triangles, bounds, components, and
  wall-time in test output and failure messages.

## 5. Verification

- [x] 5.1 Run focused native OCCT tests.
- [x] 5.2 Run relevant Rust tests and frontend STL-normal tests.
- [x] 5.3 Run `cd src-tauri && cargo check`.
- [x] 5.4 Run `openspec validate shared-stl-tessellation-quality`.
- [x] 5.5 Inspect a generated radius-`3` sphere STL in the real viewer and
  confirm print export uses the same triangle count.
