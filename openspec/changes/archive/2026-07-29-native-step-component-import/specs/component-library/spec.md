## ADDED Requirements

### Requirement: Provenance-backed STEP component admission

A live STEP component SHALL reference package-local `.step`/`.stp` bytes whose
digest matches installed inventory and SHALL carry package geometry provenance.

#### Scenario: Valid STEP payload resolves

- **WHEN** locked STEP bytes and declared `analyticBrep`,
  `facetedPolyBrep`, or `hybrid` provenance match installed evidence
- **THEN** component resolution returns a StepAsset payload

#### Scenario: Missing provenance requires repackaging

- **WHEN** legacy STEP component lacks geometry provenance
- **THEN** live resolution fails with repackaging guidance
- **AND** does not infer `analyticBrep` from `.step`

#### Scenario: Payload mutation blocks resolution

- **WHEN** STEP bytes differ from locked payload digest
- **THEN** resolution fails before native execution

### Requirement: STEP dependency-lock evidence

STEP component lock entries SHALL record `payloadKind=step`, payload digest,
and declared geometry representation.

#### Scenario: STEP lock controls cache identity

- **WHEN** equal authored source resolves against different STEP bytes or
  representation evidence
- **THEN** dependency lock digests differ
- **AND** render artifacts cannot be shared
