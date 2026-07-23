## ADDED Requirements

### Requirement: Unified Source-Aware Authoring Graph

The system SHALL project AST nodes, parameters, dependencies, constraints,
features, outputs, viewer targets, and manipulation handles from canonical
`.ecky` source using stable identities and content digests.

#### Scenario: Geometry selection resolves source context

- **GIVEN** rendered geometry has exact manifest provenance
- **WHEN** author selects a stable face, edge, part, or control target
- **THEN** system resolves owning feature and stable AST node
- **AND** it exposes upstream parameters and affected output targets

#### Scenario: Missing provenance stays explicit

- **GIVEN** selected derived geometry has no exact source binding
- **WHEN** source context is requested
- **THEN** target remains selectable but non-editable
- **AND** raw non-editable reason is visible
- **AND** frontend does not synthesize editable status

### Requirement: Synchronized Spatial And Source Lenses

The system SHALL use shared selection state between Three.js spatial lens and
AST/parameter source lens.

#### Scenario: Source selection highlights geometry

- **WHEN** author selects source-backed parameter, AST node, or feature
- **THEN** corresponding stable viewer targets highlight
- **AND** viewport shows only focused dependency trace

#### Scenario: Geometry selection focuses source

- **WHEN** author selects source-backed model target
- **THEN** source lens focuses owning AST node and relevant params
- **AND** current source draft remains unchanged

### Requirement: Source-Backed Direct Manipulation

The system SHALL expose manipulation handles only when backend emits exact
source binding, manipulation frame, editable operation, and digest guards.

#### Scenario: Supported handle previews source patch

- **GIVEN** selected feature has exact editable handle binding
- **WHEN** author drags handle
- **THEN** world-space delta converts through emitted frame
- **AND** guarded AST patch validates source and node digests
- **AND** accepted draft rerenders affected geometry
- **AND** history remains unchanged until explicit Apply/Commit

#### Scenario: Stale handle patch fails safely

- **GIVEN** source or node digest changed after handle projection
- **WHEN** author drags stale handle
- **THEN** backend rejects patch
- **AND** source and accepted model stay unchanged
- **AND** raw backend error remains attached to handle context

### Requirement: Derived Vertices Are Not Generic Edit Handles

The system SHALL NOT expose arbitrary tessellation or BRep-derived vertices as
movable source controls.

#### Scenario: Derived vertex has no exact source owner

- **WHEN** author selects derived vertex without emitted source binding
- **THEN** vertex is read-only
- **AND** system reveals nearest owning feature or upstream params when known
- **AND** no geometry or source mutation occurs

### Requirement: LLM Uses Guarded Authoring Path

The system SHALL use LLM output as candidate source-edit intent, not as
authoritative projection or direct renderer mutation.

#### Scenario: Language intent resolves exact candidate

- **WHEN** LLM maps author request to exact source-backed operation
- **THEN** candidate uses same inspect, validate, preview, and commit guards as
  direct manipulation

#### Scenario: Language intent is ambiguous

- **WHEN** multiple source bindings could satisfy request
- **THEN** system presents candidates for confirmation
- **AND** no source or geometry mutation occurs before selection
