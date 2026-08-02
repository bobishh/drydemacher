## ADDED Requirements

### Requirement: Native STEP shape import

Direct OCCT SHALL import host-resolved STEP component bytes through
`STEPControl_Reader` and validate transfer status, roots, shape kind, and BRep
validity before publishing a shape slot.

#### Scenario: Valid STEP solid enters plan

- **WHEN** `import-step` receives a readable valid solid
- **THEN** Direct OCCT publishes a shape slot
- **AND** native placement, booleans, topology, STL, and STEP export can consume
  it

#### Scenario: Multiple solid roots remain compound

- **WHEN** transfer yields multiple solids
- **THEN** Direct OCCT preserves a solid-containing compound
- **AND** does not fuse roots implicitly

#### Scenario: Invalid transfer publishes nothing

- **WHEN** read fails, zero roots transfer, output is null, BRep is invalid, or
  payload is shell-only
- **THEN** native execution fails with exact import/validation stage cause
- **AND** publishes no partial slot or export

### Requirement: Native STEP path has no compatibility fallback

The package component STEP path MUST NOT invoke FreeCAD, convert STEP to STL,
or call `solidify`.

#### Scenario: STEP uses reader directly

- **WHEN** a STEP-backed live component renders
- **THEN** generated execution uses `STEPControl_Reader`
- **AND** no FreeCAD process, `StlAPI_Reader`, or `solidify` runs

#### Scenario: STL bridge remains separate

- **WHEN** an STL mesh enters downstream BRep operations
- **THEN** its path remains `solidify(import-stl(path))`
- **AND** STEP import does not share that step

### Requirement: STEP representation truth

Direct runtime SHALL propagate locked package representation evidence and
merge all contributor representations conservatively.

#### Scenario: Analytic placement remains analytic

- **WHEN** trusted analytic STEP is only placed with analytic authored geometry
- **THEN** bundle, manifest, and STEP export report `analyticBrep`

#### Scenario: Faceted input is never relabeled analytic

- **WHEN** STEP payload declares `facetedPolyBrep`
- **THEN** output reports `facetedPolyBrep` or `hybrid`
- **AND** never reports `analyticBrep`

#### Scenario: Mixed composition is hybrid

- **WHEN** analytic authored geometry combines with faceted/mixed STEP
- **THEN** bundle, manifest, and STEP export report `hybrid`

### Requirement: Imported STEP topology remains native

Imported STEP faces and edges SHALL flow through the existing Direct OCCT
topology reporter with component-origin evidence.

#### Scenario: Package ports resolve without FreeCAD

- **WHEN** package ports target locked imported face/edge evidence
- **THEN** Direct OCCT topology contains resolvable targets
- **AND** port validation uses no FreeCAD-generated topology
