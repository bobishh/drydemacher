## ADDED Requirements

### Requirement: Exploration references normal immutable versions

Exploration cycle state SHALL reference versions created by the normal lossless
append path. It SHALL NOT redefine version creation, head selection, status, success
filtering, or result attachment.

#### Scenario: Cycle build follows normal append semantics

- **GIVEN** cycle BUILD changes version-owned source
- **WHEN** the source is persisted
- **THEN** normal version append creates exactly one immutable version before checks
- **AND** normal head advances to that append independent of outcome
- **AND** exploration stores the returned version ref.

#### Scenario: Cycle selection is not promotion

- **GIVEN** a cycle compares versions A and B
- **WHEN** DECIDE records B as the chosen cycle result
- **THEN** A and B retain their identities, statuses, evidence, and append order
- **AND** no extra version is created.

### Requirement: Cycle grouping does not hide failed versions

The system SHALL preserve all exploration-created versions in normal history. Cycle
grouping and successful/printable filters SHALL be projections only.

#### Scenario: Failed exploration remains addressable

- **GIVEN** versions B and C were created during one cycle and both failed
- **WHEN** the cycle stops
- **THEN** B and C remain addressable in version history with exact source and errors
- **AND** stopping the cycle does not discard, squash, or reclassify them.

### Requirement: Version and viewport heads remain distinct projections

Exploration orchestration SHALL preserve latest-append version head semantics while
allowing the active viewport to retain the newest eligible successful render.

#### Scenario: New red version does not erase last good render

- **GIVEN** version A has a successful render and is visible
- **WHEN** newer version B is appended and render fails
- **THEN** version head is B
- **AND** viewport may continue showing A
- **AND** UI and cycle context identify both refs explicitly.
