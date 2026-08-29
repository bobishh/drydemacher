# project-sync-performance Specification

## ADDED Requirements

### Requirement: Settled project-folder edits enter apply within two seconds

The project mirror SHALL use filesystem change notifications with a trailing debounce
of at most one second. A settled `model.ecky` change SHALL be detected and start apply
within two seconds; CAD render duration is reported separately.
A bounded fallback poll SHALL repair missed notifications without becoming the primary
latency path.

#### Scenario: One settled edit produces one apply

- **GIVEN** an exported project folder with a clean source binding
- **WHEN** `model.ecky` is written and remains unchanged for one second
- **THEN** apply starts within two seconds of the write
- **AND** repeated filesystem notifications for that write produce one durable version.

#### Scenario: Missed notification is repaired

- **GIVEN** a platform filesystem event is missed
- **WHEN** fallback polling observes a new settled source digest
- **THEN** the same guarded apply path runs
- **AND** no duplicate retry runs after the digest becomes clean.

### Requirement: Semantic-only source changes reuse direct-OCCT geometry

The direct-OCCT runtime SHALL separate evaluated geometry identity from semantic tag and
manifest identity. When geometry identity is unchanged, it SHALL reuse stored BRep and
topology and SHALL rebuild semantic tags plus manifest without invoking native geometry
execution.

#### Scenario: Tag-face edit reuses geometry

- **GIVEN** a rendered Ecky model with stored BRep and topology
- **WHEN** only a `tag-face` declaration changes
- **THEN** geometry and topology digests remain unchanged
- **AND** tagged anchors and selection targets reflect the new declaration
- **AND** the native runner invocation count does not increase.

#### Scenario: Authored bindings do not disable part reuse

- **GIVEN** a part contains named build shapes exported as authored bindings
- **WHEN** the same evaluated geometry renders again
- **THEN** the part geometry cache remains eligible
- **AND** authored bindings are reapplied to reused topology.

#### Scenario: Runner binary change stays cold once

- **GIVEN** cached geometry was produced by a different native-runner binary digest
- **WHEN** the model renders after the runner update
- **THEN** old geometry cache is rejected and one cold render runs
- **AND** the next identical or semantic-only render reuses the new geometry artifact.

### Requirement: Project preview belongs to current head

The project card SHALL NOT present an older version image as the current head preview.
Preview identity SHALL include the owning message id. If current head has no image, the
card SHALL show an explicit placeholder or stale state.

#### Scenario: Head without image does not inherit old raster

- **GIVEN** an older successful version has `imageData`
- **AND** the current head has a rendered artifact but no `imageData`
- **WHEN** the project card renders
- **THEN** the older raster is not shown as current
- **AND** the card visibly reports missing or stale preview state.

### Requirement: Intentional print layout does not fail folder apply

Disconnected-part evidence SHALL remain visible, but SHALL be nonblocking when explicit
render state declares separated print layout. Other structural and authored verification
failures SHALL remain blocking.

#### Scenario: Separated print layout applies

- **GIVEN** `assembly-preview=false` explicitly selects separated print layout
- **AND** verification reports only `PART_DISCONNECTED`
- **WHEN** project-folder apply verifies the preview
- **THEN** apply succeeds
- **AND** disconnected-part evidence remains attached to the version.

#### Scenario: Real failure remains blocking

- **GIVEN** verification reports missing artifacts, invalid geometry, non-manifold output,
  or authored verify failure
- **WHEN** project-folder apply verifies the preview
- **THEN** apply fails with the raw verifier summary.
