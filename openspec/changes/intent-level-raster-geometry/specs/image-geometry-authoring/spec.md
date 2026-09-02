# image-geometry-authoring Specification

## MODIFIED Requirements

### Requirement: Intent-level raster extrusion and protrusion

The `.ecky` language SHALL accept raster image sources through `extrude` and
`protrude`. `extrude` SHALL interpret raster coverage as a traced constant-height
profile. `protrude` SHALL interpret raster coverage as continuous local elevation.
`heightfield` SHALL NOT appear in the public authoring surface.

#### Scenario: Dark raster artwork extrudes as a profile

- **GIVEN** a readable raster with dark opaque artwork on a light background
- **WHEN** `extrude` receives the image, physical width/depth, threshold, foreground,
  and positive height
- **THEN** coverage is deterministically traced into closed regions
- **AND** only those regions extrude to the requested height
- **AND** no rectangular image carrier is present.

#### Scenario: One physical dimension preserves source aspect ratio

- **GIVEN** a raster whose pixel width and height differ
- **WHEN** `extrude` or `protrude` receives exactly one of `:width` or `:depth`
- **THEN** the runtime derives the omitted physical dimension from the source pixel
  aspect ratio
- **AND** the raster is not stretched.

#### Scenario: Two physical dimensions contain by default

- **GIVEN** a raster whose source aspect ratio differs from the requested physical box
- **WHEN** `extrude` or `protrude` receives both `:width` and `:depth`
- **THEN** the runtime scales the raster uniformly to fit inside that box
- **AND** centers unused space on the constrained axis
- **AND** the raster is not stretched.

#### Scenario: Stretching is explicit

- **GIVEN** a raster whose source aspect ratio differs from the requested physical box
- **WHEN** `extrude` or `protrude` specifies `:fit stretch`
- **THEN** the runtime maps the raster into the exact requested width and depth
- **AND** non-uniform scaling is an explicit authoring choice.

#### Scenario: Physical size is absent

- **WHEN** raster `extrude` or `protrude` receives neither `:width` nor `:depth`
- **THEN** validation requires at least one physical dimension
- **AND** no output replaces the last good preview.

#### Scenario: Transparent pixels are empty

- **GIVEN** an RGBA raster containing transparent RGB values
- **WHEN** raster extrusion or protrusion decodes the image
- **THEN** alpha zero contributes no material regardless of RGB or foreground polarity
- **AND** partial alpha contributes proportional coverage before thresholding or height
  mapping.

#### Scenario: Light foreground is explicit

- **GIVEN** light artwork on a dark background
- **WHEN** the operation specifies `:foreground light`
- **THEN** light luminance contributes positive coverage
- **AND** alpha remains non-inverted.

#### Scenario: Raster protrusion starts at the local base plane

- **GIVEN** a grayscale raster and positive protrusion height
- **WHEN** `protrude` renders at local `Z=0`
- **THEN** continuous coverage maps into elevation from zero through requested height
- **AND** no backing rectangle rises above the local base plane
- **AND** the emitted mesh remains closed and structurally valid.

#### Scenario: Same raster inputs produce stable geometry

- **GIVEN** unchanged image bytes, dimensions, polarity, threshold, and height
- **WHEN** raster extrusion or protrusion renders twice
- **THEN** both geometry digests match.

#### Scenario: Dense raster is a mixed Boolean tool

- **GIVEN** a closed raster extrusion used in any operand position of a
  multi-operand Boolean whose consumers reach the part root
- **WHEN** the native runtime combines it with analytic geometry
- **THEN** the indexed raster remains in memory
- **AND** analytic peers enter one mesh-domain Boolean closure
- **AND** no STL-to-OCCT solidification seam is introduced
- **AND** the resulting mesh has zero boundary or non-manifold edges.

#### Scenario: Diagonal raster contacts remain manifold

- **GIVEN** foreground regions that meet only at one pixel corner and contours
  containing holes or long stair-step boundaries
- **WHEN** raster extrusion builds its indexed mesh
- **THEN** point contacts receive a sub-pixel bevel without merging the islands
- **AND** cap triangulation preserves every outer and hole boundary edge
- **AND** the indexed mesh has zero boundary, non-manifold, or winding-mismatch edges.

#### Scenario: Invalid raster options fail before geometry replacement

- **GIVEN** non-positive width/depth/height, threshold outside zero through one, or an
  unknown foreground or fit value
- **WHEN** validation runs
- **THEN** it reports the exact invalid option
- **AND** no output replaces the last good preview.

### Requirement: Image asset state is explicit

Image-driven geometry SHALL distinguish missing/pending assets from decoding, tracing,
or runtime failures. Referenced images SHALL use staged local asset paths; large raster
bytes SHALL NOT be embedded in `.ecky` source.

#### Scenario: Intent-level image operation has no selected file

- **GIVEN** an image parameter referenced by raster `extrude` or `protrude` with no
  selected path
- **WHEN** parameter UI opens or preview is requested
- **THEN** UI shows a pending image-selection state
- **AND** no backend render request runs for incomplete geometry.

#### Scenario: Selected file cannot decode or trace

- **GIVEN** a selected path whose bytes are unsupported or whose threshold produces no
  closed profile
- **WHEN** image decoding or tracing runs
- **THEN** preview fails while preserving the last good artifact
- **AND** raw path/decoder/tracing evidence is visible.

#### Scenario: Legacy heightfield remains readable but non-public

- **GIVEN** durable historical source containing `heightfield`
- **WHEN** the compatibility compiler reads it during the deprecation window
- **THEN** existing geometry semantics remain available
- **AND** public references, prompts, completions, and newly authored repository models
  use `extrude` or `protrude` instead.
