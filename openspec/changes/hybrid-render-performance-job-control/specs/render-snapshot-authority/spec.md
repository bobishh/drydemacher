# Delta for render-snapshot-authority

## MODIFIED Requirements

### Requirement: Render snapshot is one immutable aggregate

The system SHALL represent exact render inputs and outputs as one immutable
snapshot containing source identity, effective parameter identity, backend,
artifact bundle, and model manifest. Actor, MCP, and frontend boundaries SHALL
pass stable references or bounded projections instead of repeatedly cloning
and serializing the complete graph.

#### Scenario: Preview publishes one coherent snapshot

- GIVEN source and effective parameters render successfully
- WHEN a preview is published to MCP or frontend consumers
- THEN source, effective parameters, backend, artifact, and manifest share one
  snapshot identity
- AND consumers do not reconstruct the snapshot from independent stores

#### Scenario: Mismatched payload is rejected

- GIVEN design metadata names one render input
- AND artifact or manifest metadata names another model or backend
- WHEN snapshot validation runs
- THEN the snapshot is rejected before replacing the last good projection
- AND the raw error identifies conflicting fields

#### Scenario: Dense topology uses bounded projection

- GIVEN a snapshot contains dense triangle-derived topology
- WHEN MCP or frontend requests preview evidence
- THEN authored, tagged, and analytic targets are returned eagerly
- AND anonymous dense mesh targets use a lazy indexed query
- AND truncation metadata is explicit

#### Scenario: Truncated source cannot replace full source implicitly

- GIVEN a source query returned a truncated window
- WHEN a caller submits that window as a full macro replacement
- THEN the request is rejected without explicit truncation acknowledgement

