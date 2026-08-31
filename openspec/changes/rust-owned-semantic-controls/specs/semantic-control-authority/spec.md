# Delta for semantic-control-authority

## ADDED Requirements

### Requirement: Rust owns control-view mutation

The system SHALL load and mutate the canonical manifest in Rust when the
workbench saves or deletes a control view. Frontend SHALL NOT submit a complete
replacement manifest for these actions.

#### Scenario: Manual view save is one intent

- GIVEN an imported model with persisted semantic primitives
- WHEN user saves a manual view
- THEN frontend sends one control-view save intent
- AND Rust assigns manual source, validates references, persists canonical state,
  and returns the resulting manifest.

#### Scenario: Manual view delete cleans references

- GIVEN a manual view referenced by advisories
- WHEN user deletes that view
- THEN Rust removes the view and advisory references atomically
- AND frontend projects the returned manifest.

#### Scenario: Ecky view persistence is rejected

- GIVEN an Ecky-native manifest
- WHEN any caller attempts to save a persisted control view
- THEN Rust rejects the intent
- AND `controlViews` remains empty.

### Requirement: Semantic callers share mutation rules

MCP and Tauri semantic control-view callers SHALL use one Rust mutation service.

#### Scenario: Actor source remains explicit

- GIVEN equivalent save intents from MCP and workbench
- WHEN the shared service normalizes each view
- THEN MCP result has `source=llm`
- AND workbench result has `source=manual`.

### Requirement: Workbench projects pending and raw failure

The workbench SHALL suppress duplicate control-view mutations while one intent is
pending and SHALL display the backend error body on failure.

#### Scenario: Slow save remains single-flight

- GIVEN a control-view save command is pending
- WHEN user attempts another save
- THEN no second command is submitted
- AND save action remains visibly busy.

#### Scenario: Backend rejection stays visible

- GIVEN backend rejects a control-view save
- WHEN command resolves with an error
- THEN workbench displays that raw backend error body
- AND composer draft remains available for correction.

### Requirement: Rust resolves semantic control values

The system SHALL resolve semantic primitive values in Rust against one exact
version target. Frontend SHALL NOT compute primitive binding transforms, clamps,
relation propagation, or AST-provenance parameter keys.

#### Scenario: Linked numeric control produces one canonical patch

- GIVEN an editable semantic primitive with scaled bindings and enabled relations
- WHEN workbench submits target identity, primitive identity, and next value
- THEN Rust applies scale, offset, and binding bounds
- AND Rust propagates each reachable relation at most once
- AND frontend stages only the returned canonical parameter patch.

#### Scenario: Ecky provenance control resolves declared parameter

- GIVEN a generated Ecky model exposes `ast-param:width`
- WHEN workbench submits a value for that primitive
- THEN Rust verifies `width` is a declared non-frozen UI field
- AND returns a patch for exactly `width`
- AND no persisted semantic primitive or frontend-built identifier is required.

#### Scenario: Legacy derived control identity is canonical

- GIVEN a legacy model has a declared UI field but no persisted primitive
- WHEN workbench submits its derived primitive identity
- THEN Rust regenerates identity from declared field key
- AND rejects unknown or ambiguous identities
- AND frontend does not choose parameter binding from the identifier.

#### Scenario: Invalid semantic target changes nothing

- GIVEN an unknown, locked, stale, or type-invalid semantic primitive input
- WHEN Rust resolves the intent
- THEN it returns the exact validation error
- AND returns no partial parameter patch.

### Requirement: Rust owns remaining semantic manifest mutation

Primitive, advisory, relation, and imported enrichment-proposal edits SHALL be
submitted as tagged intent. Rust SHALL load canonical state, validate ownership
and references, apply dependent cleanup or binding rebuild, persist, and return
canonical state. Frontend SHALL NOT construct a replacement manifest for these
actions.

#### Scenario: Manual primitive save owns canonical fields

- GIVEN imported model controls and a workbench primitive draft
- WHEN user saves primitive
- THEN Rust assigns manual identity, source, and order
- AND optional view attachment is created or updated atomically
- AND frontend projects returned manifest.

#### Scenario: Manual primitive delete cleans references

- GIVEN manual primitive referenced by views, rules, links, targets, or measurements
- WHEN user deletes primitive
- THEN Rust deletes only manual-owned primitive
- AND dependent references are removed atomically
- AND resulting manifest validates.

#### Scenario: Advisory and relation ownership is protected

- GIVEN generated or inherited semantic entity
- WHEN workbench attempts manual delete
- THEN Rust rejects non-manual ownership
- AND canonical manifest remains unchanged.

#### Scenario: Imported proposal acceptance rebuilds bindings

- GIVEN imported FCStd enrichment proposals
- WHEN proposal status changes
- THEN Rust derives aggregate enrichment status
- AND Rust rebuilds proposal-owned groups, part editability, target editability,
  and canonical warning state
- AND frontend receives resulting manifest without computing those facts.

#### Scenario: Imported proposal batch is atomic

- GIVEN multiple imported FCStd proposals
- WHEN caller submits one batch of proposal status entries
- THEN Rust validates every proposal id and status before mutation
- AND applies all statuses before one aggregate status and binding rebuild
- AND any unknown entry rejects the complete batch without persistence.

#### Scenario: Ecky semantic persistence is rejected

- GIVEN Ecky-native AST-derived controls
- WHEN workbench submits any persisted semantic edit
- THEN Rust rejects intent
- AND AST remains sole semantic authority.
