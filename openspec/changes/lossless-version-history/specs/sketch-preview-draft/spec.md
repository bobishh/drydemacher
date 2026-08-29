# Delta for sketch-preview-draft

## MODIFIED Requirements

### Requirement: Sketch preview uses a stable draft identity

The system SHALL retain one stable active sketch scope while representing every
changed draft content snapshot as an immutable version under that scope.

#### Scenario: Preview rerender versions changed draft

- GIVEN a sketch draft scope is active
- WHEN the user changes sketch input and reruns preview
- THEN exactly one immutable version is appended with changed draft content
- AND the stable draft scope remains associated with that version
- AND head advances before preview completes.

#### Scenario: Fresh sketch session starts a new draft

- GIVEN no active sketch draft scope exists
- WHEN the user previews changed sketch content
- THEN the system creates a new active sketch scope
- AND appends its first content version
- AND attaches preview success or raw failure evidence to that version.

### Requirement: Sketch draft can be saved explicitly

The system SHALL provide an explicit save action that marks the current draft
version for later restore. Saving content already appended SHALL NOT create a
content-identical duplicate version.

#### Scenario: Save draft marks current version

- GIVEN an active sketch draft version exists
- WHEN the user selects Save Draft without changing content
- THEN the system marks that version as the restore target
- AND keeps the current preview available
- AND appends no duplicate content version.

#### Scenario: Save draft can fork a fresh scope

- GIVEN an active sketch draft version exists
- WHEN the user selects Save Draft with new scope enabled
- THEN the system creates a fresh scope referencing the current version content
- AND later changes append under the fresh scope
- AND existing version history remains recoverable.

### Requirement: Sketch draft can be discarded explicitly

The system SHALL represent discard of a persisted draft as an appended tombstone
version. Discard SHALL clear the active preview without deleting or rewriting
prior draft versions. Clearing never-persisted scratch state may remain local.

#### Scenario: Discard draft appends tombstone

- GIVEN an active persisted sketch draft version exists
- WHEN the user selects Discard Draft
- THEN a discard tombstone is appended and becomes head
- AND the active preview is cleared
- AND prior draft versions remain recoverable.

### Requirement: Preview and persistence stay separated

The system SHALL persist every changed draft snapshot before preview while
keeping renderability as version result metadata. Preview failure SHALL NOT
erase the version or move head to an older successful preview.

#### Scenario: Preview failure does not lose changed content

- GIVEN the user changes sketch content
- WHEN preview fails
- THEN the changed snapshot already exists as an immutable failed version
- AND that failed version is head
- AND successful-preview filtering may return an earlier version without
  relabeling it as head.

### Requirement: Raster references share the stable sketch draft

Raster reference work SHALL retain the active sketch scope while appending every
changed reference selection, extraction, contour review, or preview input as an
immutable draft version.

#### Scenario: Re-extract contour appends changed draft

- GIVEN a sketch draft with one raster-derived Front contour
- WHEN threshold settings change and extraction reruns
- THEN one version containing changed settings and contour candidates is appended
- AND the same draft scope remains active.

#### Scenario: Failed extraction preserves reviewed sketch

- GIVEN a reviewed raster-derived sketch and successful earlier preview
- WHEN a changed extraction attempt fails
- THEN the failed snapshot is retained as newer head with raw evidence
- AND the reviewed version remains queryable through history and the successful
  filter.
