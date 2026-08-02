## ADDED Requirements

### Requirement: Copy-inline and live-reference modes remain distinct

The system SHALL preserve MCP/UI `component_import` as a copy-inline vendoring
operation and SHALL expose `(import-component ...)` as a separate live package
reference. Neither mode SHALL silently produce the other.

#### Scenario: Copy-inline component remains self-contained

- **WHEN** a component is inserted through MCP/UI `component_import`
- **THEN** full component source is inserted into the authored model
- **AND** no `import-component` declaration or dependency lock is created

#### Scenario: Live reference remains external and locked

- **WHEN** source declares `import-component`
- **THEN** persisted authored source retains the package coordinate and alias
- **AND** package source is not copied into persisted authored source
- **AND** render requires resolved dependency-lock evidence

### Requirement: Explicit source component reference

The authored surface SHALL support top-level `import-component` with literal
package id, exact package version, component id, and model-local alias. The
declaration SHALL produce no geometry until its alias is instantiated.

#### Scenario: Exact source component binds alias

- **WHEN** source imports `bike.kit@1.2.0:cage` as `holder`
- **THEN** `(holder)` instantiates that exported package component
- **AND** the declaration itself creates no part

#### Scenario: Incomplete or dynamic coordinate fails

- **WHEN** any coordinate field/alias is missing or computed dynamically
- **THEN** host pre-resolution fails before ordinary compilation with a
  field-specific literal-required diagnostic

### Requirement: Live-reference alias namespace

Import aliases SHALL be valid callable Ecky symbols unique across imports,
local components, helpers, and reserved CAD forms. Collisions SHALL fail
without declaration-order shadowing.

#### Scenario: Duplicate alias names both coordinates

- **WHEN** two package components import as the same alias
- **THEN** resolution fails and names both canonical package identities

#### Scenario: Local binding collision fails

- **WHEN** an import alias matches a local component, helper, or reserved form
- **THEN** resolution fails before package source materialization

### Requirement: Host pre-resolution precedes pure compile

Live imports SHALL resolve through the host-owned component import runtime,
which produces ephemeral compiler source, a dependency lock, and import-span
evidence before calling the unchanged source-only compiler.

#### Scenario: Package-aware compile returns evidence

- **WHEN** valid imported source compiles through the host entrypoint
- **THEN** the result contains CoreProgram, dependency lock, and node-origin
  evidence
- **AND** existing compiler APIs receive only resolved compiler source

#### Scenario: Raw compiler does not perform filesystem lookup

- **WHEN** unresolved import source is passed directly to
  `compile_to_core_program(&str)`
- **THEN** it returns a host-resolution-required diagnostic
- **AND** performs no package filesystem access

#### Scenario: Existing source remains stable

- **WHEN** source has no live imports
- **THEN** stable keys, emitted spelling, CoreProgram digest, and rendering
  remain unchanged
