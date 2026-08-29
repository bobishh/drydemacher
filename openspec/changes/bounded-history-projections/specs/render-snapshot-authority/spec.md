# Delta for render-snapshot-authority

## MODIFIED Requirements

### Requirement: Render snapshot is one immutable aggregate

The system SHALL represent exact render inputs and outputs as one immutable
snapshot containing source identity, effective parameter identity, backend,
artifact bundle, and model manifest. Actor, MCP, and frontend boundaries SHALL
pass its stable identity and bounded projections rather than clone or serialize
the complete aggregate by default.

#### Scenario: Preview publishes one coherent snapshot

- **GIVEN** source and effective parameters render successfully
- **WHEN** a preview is published to MCP or frontend consumers
- **THEN** source, effective parameters, backend, artifact, and manifest share
  one snapshot identity
- **AND** consumers do not reconstruct the snapshot by merging independent
  stores.

#### Scenario: Mismatched payload is rejected

- **GIVEN** design metadata names one render input
- **AND** artifact or manifest metadata names another model or backend
- **WHEN** snapshot validation runs
- **THEN** the snapshot is rejected before replacing the last good projection
- **AND** the raw error identifies the conflicting fields.

#### Scenario: Background thread preview completes

- **GIVEN** a preview completes for a thread that is not active in one view
- **WHEN** the authoring actor publishes completion
- **THEN** the view receives only snapshot identity, revision, and compact status
- **AND** it does not receive artifact bundle, model manifest, source, image
  bytes, or dense topology.

#### Scenario: Active view hydrates preview

- **GIVEN** a compact preview event names the active thread
- **WHEN** the view needs to show that preview
- **THEN** it hydrates one coherent bounded snapshot projection by identity
- **AND** it does not send the same aggregate back to backend persistence.

#### Scenario: Dense topology is inspected

- **GIVEN** a snapshot contains anonymous dense edge, face, or triangle targets
- **WHEN** core preview detail is hydrated
- **THEN** core detail contains explicit counts and a dense-topology reference
- **AND** dense targets are returned only through bounded indexed pages on demand.
