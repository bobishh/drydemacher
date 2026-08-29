# ast-control-provenance Specification

## ADDED Requirements

### Requirement: Generated Ecky ownership follows reachable AST dependencies

The system SHALL derive each generated Ecky part's parameter keys from parameters
reachable from that part's Core IR result. It SHALL NOT replace those keys with the
complete model parameter list.

#### Scenario: Disjoint parts retain disjoint controls

- **GIVEN** two generated parts reference disjoint parameter sets
- **WHEN** the runtime manifest is lowered
- **THEN** each part contains only its reachable parameter keys
- **AND** neither part claims the other part's keys.

#### Scenario: Unused build stage claims nothing

- **GIVEN** a named build stage references a parameter but is unreachable from the part
  result
- **WHEN** ownership is derived
- **THEN** that parameter is not assigned to the part or an active shape group.

### Requirement: Named build and feature groups preserve transitive provenance

The system SHALL project reachable named build stages and explicit features into stable
parameter groups and feature graph nodes. A stage's inferred dependencies SHALL include
parameters reached through referenced prior stages.

#### Scenario: Derived stage follows its base

- **GIVEN** named shape `rounded` references named shape `base` and parameter `radius`
- **AND** `base` references parameters `width` and `height`
- **WHEN** provenance is derived
- **THEN** `rounded` depends on `width`, `height`, and `radius`
- **AND** its generated group id remains stable across value-only edits.

#### Scenario: Explicit feature params stay primary

- **GIVEN** an explicit feature declares `:params (gap)` and its expression also depends
  on `width`
- **WHEN** controls are projected
- **THEN** `gap` is presented as the feature's primary control
- **AND** `width` remains in its inferred dependency safety set.

### Requirement: Generated Ecky control groups are derived, not authored Views

The system SHALL build generated Ecky parameter presentation from AST provenance and
SHALL NOT generate or persist `controlViews` for Ecky-native models.

#### Scenario: Unguided valid source remains groupable

- **GIVEN** valid Ecky source contains params and parts but no Views metadata
- **WHEN** it renders
- **THEN** deterministic model/part groups are present
- **AND** `controlViews` remains empty.

#### Scenario: Rerender rejects stale semantic carry-forward

- **GIVEN** an earlier Ecky preview contains LLM control metadata or an obsolete feature
  graph
- **WHEN** the source rerenders
- **THEN** the persisted manifest keeps the freshly compiled provenance
- **AND** earlier control primitives, relations, Views, and topology bindings are not
  carried into the new Ecky preview.

### Requirement: Exact topology controls require exact authored evidence

The direct-OCCT runtime SHALL map topology targets to parameter keys through authored
shape bindings or named topology tags. Mesh-native and unbound topology SHALL NOT invent
face-level dependencies.

#### Scenario: Authored face binding resolves narrow controls

- **GIVEN** a direct-OCCT face reports authored binding `bore`
- **AND** named shape `bore` depends on `boreDiameter`
- **WHEN** the face becomes a selection target
- **THEN** the target contains `boreDiameter`
- **AND** unrelated part parameters are absent.

#### Scenario: Unbound face remains non-editable

- **GIVEN** a topology face has no exact authored binding or tag provenance
- **WHEN** selection targets are lowered
- **THEN** its parameter key set is empty
- **AND** the runtime does not substitute all part or model keys.

### Requirement: Params renders deterministic ownership sections first

The Params surface SHALL render ownership sections directly below search. Shared
parameters SHALL appear once in default presentation, dense sections SHALL be compact,
and the complete model SHALL remain discoverable below the selected scope.

#### Scenario: Large macro avoids flat control dump

- **GIVEN** a generated Ecky model has 49 parameters across 8 parts
- **WHEN** Params opens without a selection
- **THEN** controls are partitioned into stable model/part ownership sections
- **AND** no single flat 49-control list is rendered
- **AND** dense sections begin collapsed.

#### Scenario: Part selection foregrounds its controls

- **GIVEN** grouped Params and a selected generated part
- **WHEN** the selection is received from the viewport
- **THEN** its owning section expands and becomes primary
- **AND** unrelated sections stay collapsed below it.
