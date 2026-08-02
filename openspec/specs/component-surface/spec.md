# component-surface Specification

## Purpose
TBD - created by archiving change component-unification. Update Purpose after archive.
## Requirements
### Requirement: Unified component entity

The authored surface SHALL represent `model`, `part`, `feature`, and
`define-component` forms as one component entity with a role
(`root`/`output`/`library`), a preserved spelling, an optional parameter
signature, and a body.

#### Scenario: Model parses as root component

- GIVEN an existing `.ecky` model authored with `(model (params ...) (part ...))`
- WHEN the source compiles
- THEN the resulting CoreProgram is identical to the pre-change output
- AND stable node keys are byte-identical to the pre-change derivation.

#### Scenario: Part parses as output component

- GIVEN a `(part id label expr)` clause
- WHEN the source compiles
- THEN the part produces exactly one CorePart with today's key/label/root
  behavior
- AND output-role components remain the only topology part boundaries.

### Requirement: Spelling preservation on emit

The emitter SHALL write each component back with its original authored
spelling.

#### Scenario: Roundtrip keeps spellings

- GIVEN source authored with `model`, `part`, and `feature` clause heads
- WHEN the source is parsed and re-emitted
- THEN every clause head re-emits with its original spelling
- AND no clause is rewritten to `component` or `define-component`.

### Requirement: Component definition and instantiation

The surface SHALL support `(define-component name (signature...) body)` and
instantiation `(name :key value ...)` with lexically scoped parameters,
defaults, and keyword overrides.

#### Scenario: Defaults and overrides

- GIVEN a component with signature entry `(number pin_d 8 :min 4 :max 12)`
- WHEN instantiated as `(knuckle)` and `(knuckle :pin_d 6)`
- THEN the first expansion binds `pin_d` to 8 and the second to 6.

#### Scenario: Closed body

- GIVEN a `define-component` body referencing a variable not in its signature
- WHEN the source compiles
- THEN compilation fails with an error naming the free variable and the
  component.

#### Scenario: Unknown keyword rejected

- GIVEN an instantiation passing a keyword not present in the signature
- WHEN the source compiles
- THEN compilation fails with an error naming the keyword and listing the
  component signature.

#### Scenario: Recursive instantiation rejected

- GIVEN components that instantiate themselves directly or in a cycle
- WHEN the source compiles
- THEN compilation fails with a deterministic cycle/depth error and does not
  hang.

### Requirement: Compile-time inline expansion

Component instantiation SHALL expand inline into the existing CoreProgram
shape before planning, with fresh node ids and the call site recorded as
source anchor.

#### Scenario: Core IR unchanged

- GIVEN a model whose parts are built from nested component instantiations
- WHEN the source compiles
- THEN the resulting CoreProgram uses only existing Core IR constructs
- AND `ecky_core_ir` public structs are unchanged by this feature.

#### Scenario: Both compile paths agree

- GIVEN the same component-using source
- WHEN compiled via the expanded-AST path and via the Steel runtime path
- THEN both produce identical CorePrograms.

### Requirement: Verify clauses travel with components

`verify` clauses authored inside a component definition SHALL expand once per
instantiation with tags namespaced by the instantiating part key.

#### Scenario: Per-instance verify tags

- GIVEN a component containing `(verify (tag fit) ...)` instantiated by parts
  `hinge_a` and `hinge_b`
- WHEN the model compiles and renders
- THEN structural verification reports checks tagged `hinge_a/fit` and
  `hinge_b/fit`.

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

### Requirement: STEP live reference is a static shape component

A locked STEP-backed live component SHALL bind as an opaque zero-argument shape
alias. Existing authored placement, transforms, selectors, booleans, and export
SHALL consume that shape.

#### Scenario: STEP alias composes with authored geometry

- **WHEN** STEP component `bracket` is imported as `mount`
- **THEN** `(mount)` produces one native BRep shape operand
- **AND** authored transforms and booleans can consume it

#### Scenario: STEP alias rejects geometry arguments

- **WHEN** `(mount :width 20)` or another positional/keyword argument is passed
  to a STEP alias
- **THEN** compilation fails with a static-component signature diagnostic
- **AND** no argument is ignored

#### Scenario: Persisted source contains no installed path

- **WHEN** host resolution materializes a package-local STEP path
- **THEN** only ephemeral compiler source/native plan contains that path
- **AND** persisted authored source retains package coordinate plus alias

