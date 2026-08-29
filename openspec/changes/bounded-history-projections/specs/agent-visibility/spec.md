# Delta for agent-visibility

## ADDED Requirements

### Requirement: Transient activity retention is bounded

The system SHALL distinguish durable conversation turns from transient agent
activity. Activity catch-up and frontend retention SHALL use sequence cursors
and explicit count/byte limits without removing finished or interrupted-turn
transcript from durable history.

#### Scenario: Long session reconnects

- **GIVEN** a session produced more activity than one catch-up budget
- **WHEN** the frontend reconnects
- **THEN** it receives one bounded newest activity page and continuation metadata
- **AND** it does not receive every activity event in one IPC response.

#### Scenario: Activity retention compacts

- **GIVEN** transient activity exceeds retention policy
- **WHEN** old unpinned activity is compacted
- **THEN** dropped count and oldest available cursor are explicit
- **AND** pinned errors and durable finished/interrupted turn content remain
  inspectable.
