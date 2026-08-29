## MODIFIED Requirements

### Requirement: Component definition and instantiation

The surface SHALL support
`(define-component name (signature...) interface... body)` and instantiation
with lexically scoped parameters, defaults, and keyword overrides. Interface
clauses MAY declare named local ports before the body. A component invocation
SHALL still produce geometry in the component's canonical local frame; source-
native placement SHALL wrap that invocation through the component-placement
mate form rather than exposing parent/world coordinates to the body.

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

#### Scenario: Port declarations share component parameter scope

- **GIVEN** a component port origin or fit value references a signature parameter
- **WHEN** the component is instantiated with an override
- **THEN** geometry and port metadata evaluate under the same lexical bindings
- **AND** the solved placement uses the overridden port frame

#### Scenario: Output parts can expose target ports

- **GIVEN** an output-role `part` declares named ports before its geometry body
- **WHEN** another component instance references one of those ports
- **THEN** the port resolves by stable part and port ids
- **AND** the declaration does not create manufacturing geometry

## ADDED Requirements

### Requirement: Component interface spelling roundtrips

The parser and emitter SHALL preserve authored port, port-reference, mate,
orientation, roll, offset, and mirror spelling without converting the source to
raw `translate`/`rotate` forms.

#### Scenario: Mate source survives parse and emit

- **WHEN** source containing a mated component instance is parsed and re-emitted
- **THEN** its component ids, port ids, modifiers, and source ordering remain
  semantically identical
- **AND** generated placement transforms do not replace the authored mate syntax
