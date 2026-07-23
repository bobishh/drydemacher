## ADDED Requirements

### Requirement: Raster references share the stable sketch draft

Raster reference work SHALL update the active stable sketch draft rather than
creating hidden threads or versions. This includes reference selection, contour
extraction, contour review, and subsequent preview.

#### Scenario: Re-extract contour updates draft in place

- **GIVEN** a sketch draft with one raster-derived Front contour
- **WHEN** threshold settings change and extraction reruns
- **THEN** the same draft identity receives the new contour candidates
- **AND** no thread or committed version is created.

#### Scenario: Failed extraction preserves reviewed sketch

- **GIVEN** a reviewed raster-derived sketch and successful last preview
- **WHEN** a later extraction attempt fails
- **THEN** the last reviewed `SketchDocument` and preview remain available
- **AND** failure evidence is attached to the current draft.

### Requirement: Raster provenance survives draft save and restore

Saved sketch drafts SHALL persist raster reference identity, calibration,
extraction settings, selected contour identity, and derived primitive provenance
needed to reproduce or audit the draft.

#### Scenario: Reload restores raster-derived draft

- **GIVEN** a saved draft with calibrated raster references and reviewed
  contours
- **WHEN** app reload restores that draft
- **THEN** reference state and editable sketch primitives are restored
- **AND** source provenance remains distinguishable from hand-authored and
  BRep-derived primitives.
