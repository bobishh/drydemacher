## Why

`component-package-imports` establishes exact live references, package
integrity, lock storage, and provenance for Ecky source components. STEP-backed
components need a separate native geometry adapter: they must enter Direct OCCT
as BRep through `STEPControl_Reader`, never through the compatibility FreeCAD
pipeline or a STEP→STL→`solidify` round-trip.

## What Changes

- Depend on the contracts and host resolver seam from
  `component-package-imports`.
- Extend resolved component payloads and dependency-lock entries with a static
  STEP asset kind.
- Add `OcctOp::ImportStep` and native STEP reader/validation execution.
- Bind STEP aliases as opaque zero-argument shape components consumable by
  placement, selectors, booleans, topology, and export.
- Preserve package-carried representation provenance; never infer
  `analyticBrep` from `.step`.
- Keep FreeCAD STEP import/assembly as compatibility UX only, outside this
  authored runtime path.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `component-surface`: Allow a locked STEP payload alias to behave as a static
  authored shape.
- `component-library`: Define STEP package importability, provenance, and lock
  payload evidence.
- `direct-occt-runtime`: Add native STEP read, validation, topology, and
  provenance behavior without FreeCAD or `solidify`.

## Impact

- Component package definition/header provenance fields.
- `component_import_runtime` resolved payload enum and lock payload kind.
- Direct OCCT op enum, normalizer/planner, generated executor, runner stages,
  SDK required headers, topology reporting, and runtime provenance merge.
- Focused Rust executor and end-to-end package integration tests. No new UI.
