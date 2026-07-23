# image-geometry-authoring Specification

## Purpose
TBD - created by archiving change mesh-native-image-authoring. Update Purpose after archive.
## Requirements
### Requirement: Deterministic planar heightfield geometry

The `.ecky` language SHALL provide a planar `heightfield` operation that turns
a referenced raster image into a dimensioned closed relief mesh using explicit
width, depth, relief height, base thickness, and inversion inputs.

#### Scenario: Grayscale image produces closed relief

- **GIVEN** a readable grayscale or color raster and valid physical dimensions
- **WHEN** `heightfield` renders
- **THEN** luminance is sampled deterministically into surface elevation
- **AND** side walls and base close the result
- **AND** structural verification reports zero boundary and non-manifold edges.

#### Scenario: Same inputs produce stable geometry

- **GIVEN** unchanged image bytes, dimensions, sampling settings, and inversion
- **WHEN** heightfield renders twice
- **THEN** both mesh digests match.

#### Scenario: Invalid dimensions fail before image meshing

- **GIVEN** non-positive width, depth, relief height, or base thickness
- **WHEN** heightfield validation runs
- **THEN** it fails with the invalid field and value
- **AND** no output mesh replaces the last good preview.

### Requirement: Image asset state is explicit

Image-driven geometry SHALL distinguish missing/pending assets from decoding or
runtime failures. Referenced images SHALL use existing staged local asset paths;
large raster bytes SHALL NOT be embedded into `.ecky` source.

#### Scenario: Image parameter has no selected file

- **GIVEN** a heightfield image parameter with no selected path
- **WHEN** the parameter UI opens or preview is requested
- **THEN** UI shows a pending image-selection state
- **AND** no backend render request runs for incomplete geometry.

#### Scenario: Selected file cannot decode

- **GIVEN** a selected path whose bytes are not a supported image
- **WHEN** image decoding runs
- **THEN** preview fails while preserving the last good artifact
- **AND** raw decoder error body/path context is visible.

#### Scenario: Staged image path resolves

- **GIVEN** an image attachment staged by the existing prompt/session pipeline
- **WHEN** source or parameters reference that staged path
- **THEN** heightfield generation reads that asset without copying it through an
  untracked database or scratch-file write.

### Requirement: Vision references produce inferred source, not asserted geometry

When a vision-capable LLM receives reference images, the system SHALL treat its
output as inferred `.ecky` source subject to normal compile, preview, structural
verification, and accepted-CAD gates.

#### Scenario: Reference photo generates parametric source

- **GIVEN** a reference photo and a prompt requesting a parametric approximation
- **WHEN** a vision-capable model responds
- **THEN** it returns source for the active authoring context
- **AND** artifact claims come from render/verification results rather than the
  model response.

#### Scenario: Single photo lacks hidden dimensions

- **GIVEN** one perspective photo without scale or hidden-side evidence
- **WHEN** inferred source is presented
- **THEN** UI identifies it as an inferred approximation
- **AND** it is not labeled reconstructed or accepted CAD solely from vision
  output.

### Requirement: Orthographic raster tracing produces editable sketch intent

Sketch Workspace SHALL accept front, top, and side raster references, extract
bounded high-contrast contour candidates, and convert selected contours into an
editable `SketchDocument` carrying raster-derived provenance and scale metadata.

#### Scenario: Three clean views produce contour candidates

- **GIVEN** calibrated front, top, and side line-art images with closed exterior
  contours
- **WHEN** contour extraction runs
- **THEN** each view presents one or more selectable closed contour candidates
- **AND** selected candidates become normal sketch primitives with source-image
  provenance.

#### Scenario: User reviews contours before reconstruction

- **GIVEN** raster-derived contours have been selected
- **WHEN** user adjusts points, scale, or closure and applies them
- **THEN** the editable `SketchDocument` updates
- **AND** existing preview-hull/candidate reconstruction receives that document
  only after review.

#### Scenario: Raster has no usable closed contour

- **GIVEN** a noisy or low-contrast view with no bounded closed candidate
- **WHEN** extraction runs
- **THEN** that view reports raw threshold/contour evidence
- **AND** candidate reconstruction remains pending
- **AND** no fake contour is synthesized by an LLM.

### Requirement: Raster reconstruction preserves existing acceptance gates

Raster-derived sketches SHALL use existing silhouette preview, candidate search,
projection replay, BRep render, and accepted-CAD validation. Raster provenance
SHALL NOT weaken those gates.

#### Scenario: Raster-derived hull remains preview-only

- **GIVEN** reviewed raster-derived orthographic sketches
- **WHEN** preview-hull rendering succeeds
- **THEN** result is labeled preview hull or mesh draft
- **AND** accepted CAD remains pending until exact artifact and projection
  validation pass.

#### Scenario: Exact candidate passes normal gates

- **GIVEN** raster-derived sketches whose selected candidate produces STEP and
  matching hidden-line projections
- **WHEN** candidate acceptance runs
- **THEN** accepted-CAD evidence records all contributing view/image provenance
- **AND** the same topology and projection requirements used for authored vector
  sketches pass.
