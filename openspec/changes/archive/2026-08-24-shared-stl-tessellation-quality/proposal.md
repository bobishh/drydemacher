# Proposal: Shared STL Tessellation Quality

## Problem

Direct OCCT creates the exact sphere as an analytic BRep, then tessellates it
into the same binary STL consumed by the viewer and STL export. The current
angular deflection is `0.25 rad` (~14°). That produces visible facets on small
curved parts such as a sphere with radius `3`, and the same facets are present
in the file sent to a printer.

The current quality policy is duplicated across native OCCT paths. A viewer
normal pass can change shading, but cannot recover lost STL geometry.

## Goal

Make the shared STL tessellation suitable for small curved parts while keeping
render cost and artifact size measured and bounded.

## What changes

- Replace the fixed angular literal with an adaptive policy derived from the
  linear deflection and bounded by explicit minimum/maximum angles. The first
  policy follows FreeCAD's proven shape:
  `clamp(linear * 5 + 0.005, 0.005, 0.1)` radians.
- Keep linear deflection explicit and named rather than relying on positional
  literals.
- Apply the policy to every Direct OCCT STL writer path used for runtime
  preview and STL export.
- Add regression coverage for a small sphere, cylinder, and box.
- Record triangle-count, STL-size, wall-time, bounds, manifold, and component
  evidence for the quality change.

## Out of scope

- Changing analytic STEP geometry or STEP tessellation.
- Creating a separate low-quality preview STL.
- Replacing STL with a different print/export format.
- Fixing Three.js material, lighting, camera, or normal policy except where a
  test proves the shared STL change is insufficient.
- Silent post-export simplification.

## Success criteria

- A radius-`3` sphere renders with materially denser tessellation than the
  current `0.25 rad` baseline and no topology regression.
- The exported STL contains that same denser tessellation.
- Box and ordinary CAD workloads do not incur unbounded triangle, byte, or
  wall-time growth.
- Direct OCCT and generated-runner paths use the same named quality policy.
