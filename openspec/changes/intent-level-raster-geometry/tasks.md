## 1. Outer Red

- [x] 1.1 Add render integration for raster `extrude`; confirm current sketch-type failure.
- [x] 1.2 Add render integration for raster `protrude`; confirm missing-operation failure.
- [x] 1.3 Add pending-image Playwright scenario for intent-level operations.

## 2. Raster Semantics

- [x] 2.1 Add coverage tests for alpha-zero, partial alpha, dark foreground, and light foreground.
- [x] 2.2 Reuse bounded deterministic contour extraction for raster extrusion.
- [x] 2.3 Lower protrusion through internal heightfield geometry with closure below local Z=0.
- [x] 2.4 Preserve raw decoder/tracing errors and topology budgets.

## 3. Language and Backend Contract

- [x] 3.1 Extend `extrude` signatures for sketch-or-image inputs and raster options.
- [x] 3.2 Add typed `protrude` operation and backend partitioning.
- [x] 3.3 Remove `heightfield` from public operation manifests while retaining legacy parsing.
- [x] 3.4 Rename pending-image detection and status copy around image geometry.

## 4. Migration

- [x] 4.1 Migrate repository models, missions, examples, and editor fixtures.
- [x] 4.2 Migrate current hashishene project model to raster `extrude`.
- [x] 4.3 Regenerate public agent/reference artifacts without public `heightfield` syntax.

## 5. Aspect-safe raster sizing

- [x] 5.1 Specify one-dimension aspect preservation, default contain, and explicit stretch.
- [x] 5.2 Cover width-only `extrude`, depth-only `protrude`, centered contain, and missing-size validation.
- [x] 5.3 Resolve omitted dimensions and contain offsets from decoded source pixel dimensions.
- [x] 4.4 Retain legacy syntax only in compatibility tests and archived specifications.

## 5. Proof

- [x] 5.1 Green focused compiler/runtime/raster tests.
- [x] 5.2 Green pending happy/failure Playwright on an alternate port.
- [x] 5.3 Green `cargo check`, formatting, and strict OpenSpec validation.
