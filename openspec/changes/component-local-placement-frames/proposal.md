## Why

Reusable `.ecky` components currently expand as geometry but do not expose a
source-native local coordinate frame or named attachment interfaces. Components
therefore leak assembly/world coordinates into their bodies; moving a latch from
a front wall to a side wall requires recalculating axes, offsets, handedness, and
hook geometry instead of changing one placement relation.

## What Changes

- Give every `define-component` a canonical local frame: authored geometry is
  local by default and never needs enclosure/world coordinates for placement.
- Add named component ports with explicit origin and orthonormal axes, insertion
  direction, compatibility type, and optional fit metadata.
- Add source-native component instances and mates that solve one rigid placement
  transform from a component port to a target port.
- Support explicit orientation controls at the mate: aligned/opposed normals,
  axial rotation, offset, and handedness/mirror. No Euler-angle guessing at call
  sites.
- Reuse the installed-package assembly port-frame solver and placement evidence;
  do not create a second solver for inline `.ecky` components.
- Preserve current inline expansion and transform forms. Existing source remains
  valid; local frames and mates are additive.
- Emit placement and mate evidence into manifests and diagnostics so rendered
  orientation can be inspected and verified.
- Preserve declared ports through component extraction, copy-inline reuse, and
  live package references.

## Capabilities

### New Capabilities

- `component-placement`: Local component frames, named ports, rigid mate solving,
  handed placement, diagnostics, and backend-independent placement evidence.

### Modified Capabilities

- `component-surface`: `define-component` gains interface declarations and
  source-native instantiation/mating without leaking assembly coordinates into
  component geometry.
- `component-library`: Extraction and package reuse preserve declared component
  ports and their local frames.

## Impact

- `.ecky` parser, expanded AST, typechecker, emitter, stable node ids, and Core IR
  lowering boundary.
- Existing `plane`/`location`/`place` forms and installed-component assembly
  frame solver.
- Direct OCCT, portable Core planning, FreeCAD, mesh preview, STEP/STL export, and exploded
  preview placement parity.
- ArtifactBundle/ModelManifest placement evidence and physical-context errors.
- MCP authoring guide, generated language reference, component extraction, and
  component package manifests.
- Compiler, solver, backend parity, extraction, and end-to-end macro fixtures.
