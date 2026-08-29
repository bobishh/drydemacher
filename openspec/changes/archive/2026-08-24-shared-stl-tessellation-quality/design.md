# Design: Shared STL Tessellation Quality

## Current path

1. Direct OCCT builds an analytic `TopoDS_Shape`.
2. `BRepMesh_IncrementalMesh` tessellates the shape.
3. The native writer serializes the triangulation to binary STL.
4. The runtime bundle exposes that STL as `model_stl_path`.
5. Viewer loading and multipart STL export consume the resulting triangles.

The relevant implementations are the generated source emitter in
`src-tauri/src/ecky_cad_host/direct_occt_executor.rs` and the native runner in
`src-tauri/native/direct_occt_runner.cpp`. Both now carry equivalent named
policy functions at their language boundary; source tests pin the resolved
value so the two paths cannot drift silently.

## Tessellation policy

Introduce a named policy function, not a second magic angular constant:

- `linear_deflection_mm = 0.04` unless benchmark evidence requires changing
  it;
- `angular_deflection_rad(linear) = clamp(linear * 5 + 0.005, 0.005, 0.1)`;
- named `min_angular_deflection_rad = 0.005` and
  `max_angular_deflection_rad = 0.1` bounds;
- a comment stating that angular deflection controls small curved-feature
  faceting and is part of the print STL contract.

This mirrors FreeCAD's `defaultAngularDeflection` policy: derive angular
resolution from linear tolerance, cap it for performance, and floor it to
avoid pathological mesh growth. With the current `0.04 mm` linear tolerance,
the policy resolves to `0.1 rad`, improving the old `0.25 rad` default without
hardcoding a per-model angle. The formula remains an Ecky policy and must be
benchmarked against the sphere fixture.

build123d exposes linear and angular tolerances as explicit export parameters
(`0.001` and `0.1` defaults); it does not provide the adaptive formula. Ecky
keeps the derived policy at its OCCT boundary while preserving an explicit
override seam for future export-specific policies.

The generated C++ source and standalone native runner MUST receive equivalent
values. Tests MUST fail if one path drifts from the other.

Do not use viewer normal smoothing as a geometry fix. It may hide or expose
facets, but the STL triangles remain the print geometry.

## Measurement fixtures

Use deterministic native OCCT fixtures:

- sphere radius `3` — primary regression;
- cylinder radius `2`, height `6` — curved wall control;
- box `4 × 8 × 6` — planar regression control.

For each fixture collect:

- triangle count;
- binary STL byte size;
- wall time for native render;
- per-axis bounds;
- connected-component count;
- non-manifold edge count.

The sphere check MUST meet the measured lower bound of `600` triangles under
the adaptive policy. Exact triangle counts are not part of the contract
because OCCT versions may partition faces differently; the policy unit test
also rejects the old `0.25 rad` literal.

## Budgets

Use these deterministic CI guardrails for the fixture contract:

- complete three-fixture native render: no more than `3 s` wall time;
- sphere STL: no more than `100,000` bytes;
- cylinder STL: no more than `20,000` bytes;
- box STL: exactly `12` triangles and `684` bytes;
- all fixture bounds: unchanged within `0.02 mm` per axis; this remains below
  the `0.04 mm` linear tessellation tolerance and matches measured sphere
  extrema on the native runner;
- all fixture component counts: unchanged;
- all fixture non-manifold counts: zero.

These are measured from the checked-in `sphere 3`, `cylinder 2 6`, and
`box 4 8 6` fixtures. If a guardrail fails, the implementation must report
measured values. It must not silently restore the old coarse policy.

## Test shape

Follow the existing BDD sequence:

1. Add a failing regression test proving the old angular policy is present or
   produces insufficient sphere tessellation.
2. Change the named policy and generated source.
3. Run focused native tests and compare old/new artifact metrics.
4. Run the full relevant Rust test set and `cargo check`.

Tests should cover both source generation and an actual native STL artifact.
String-only tests alone are insufficient.

## Compatibility

- STEP remains analytic and unchanged.
- STL topology stays binary and manifold.
- Multipart export continues to repackage the generated STL triangles without
  reducing them.
- Existing mesh-native authoring keeps its own authored mesh density; this
  policy applies to OCCT exact-shape tessellation only.
