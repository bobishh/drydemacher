# sketch-preview-draft Specification

## Purpose
TBD - created by archiving change sketch-preview-draft-lifecycle. Update Purpose after archive.
## Requirements
### Requirement: Sketch preview uses a stable draft identity
The system SHALL represent sketch preview as a stable draft entity instead of a
new thread or version record on every render.

#### Scenario: Preview rerender refreshes draft in place
- GIVEN a sketch draft is already active
- WHEN the user changes sketch input and reruns preview
- THEN the system updates the same draft identity
- AND does not create a new thread for the rerender.

#### Scenario: Fresh sketch session starts a new draft
- GIVEN no active sketch draft exists
- WHEN the user previews a sketch
- THEN the system creates a new active sketch draft
- AND stores the rendered artifact bundle on that draft.

### Requirement: Sketch draft can be saved explicitly
The system SHALL provide an explicit save action that persists the active sketch
draft snapshot for later restore.

#### Scenario: Save draft persists current preview
- GIVEN an active sketch draft exists
- WHEN the user selects Save Draft
- THEN the system writes the draft snapshot to backend draft storage
- AND keeps the current preview available for continued work.

#### Scenario: Save draft can fork a fresh scope
- GIVEN an active sketch draft exists
- WHEN the user selects Save Draft with new scope enabled
- THEN the system writes the draft snapshot under a fresh scope id
- AND keeps the current preview available for continued work.

### Requirement: Sketch draft can be discarded explicitly
The system SHALL provide an explicit discard action that removes the active
sketch draft without persisting it.

#### Scenario: Discard draft clears active preview
- GIVEN an active sketch draft exists
- WHEN the user selects Discard Draft
- THEN the system clears the active sketch draft
- AND no new thread or version is created
- AND no saved draft snapshot remains for the active scope or fallback scope.

### Requirement: Preview and persistence stay separated
The system SHALL keep render preview separate from persistence so a render does
not implicitly save, commit, or fork history.

#### Scenario: Preview does not auto-persist
- GIVEN the user reruns sketch preview several times
- WHEN the preview result updates
- THEN the system does not create a new committed version for each rerender
- AND persistence only changes on explicit save or discard.

### Requirement: Draft restore prefers active scope then fallback
The system SHALL restore a sketch draft for the current scope when present and
fall back to the shared draft snapshot when the active scope has no saved draft.

#### Scenario: Reload restores saved draft
- GIVEN a sketch draft was saved
- WHEN the app reloads
- THEN the system restores the saved draft for the active scope or shared fallback
- AND the sketch preview status shows the draft as saved.

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

