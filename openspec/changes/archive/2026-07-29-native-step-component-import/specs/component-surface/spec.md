## ADDED Requirements

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
