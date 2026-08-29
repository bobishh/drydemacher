# Change: Replace public heightfield syntax with intent-level raster geometry

## Why

`heightfield` exposes one mesh implementation instead of the author's CAD intent.
It forces callers to manage a rectangular backing slab even when they want a logo
extrusion or a relief applied at a local base plane. Raster and vector inputs should
share `extrude`/`protrude`; decoding and meshing belong behind the language boundary.

## What Changes

- Extend `extrude` to accept raster image paths as well as vector/analytic sketches.
- Add `protrude` for continuous image-driven relief from a local base plane.
- Define alpha as coverage, explicit dark/light foreground polarity, deterministic
  raster thresholding, and physical width/depth calibration.
- Keep `heightfield` as a deprecated legacy compiler/runtime input while removing it
  from the public agent surface and authored repository models.
- Migrate examples, missions, fixtures, pending-image UX, generated references, and
  the active hashishene model to intent-level operations.

## Impact

- Affected spec: `image-geometry-authoring`.
- Affected code: Core IR signatures, mesh lowering, raster tracing/sampling, backend
  partitioning, language surface, pending-image detection, docs, examples, tests.
- Compatibility: existing saved `heightfield` sources remain readable during the
  deprecation window; newly generated source no longer uses it.
