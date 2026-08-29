# Foreign Component Import Specification

## ADDED Requirements

### Requirement: Recursive Foreign Component Discovery

The system SHALL recursively discover supported FCStd and STEP files below
each configured library root and SHALL persist the exact selected root.

#### Scenario: Workspace-level root contains nested parts

- **GIVEN** a selected root contains supported files several directories below it
- **WHEN** the library is searched
- **THEN** nested parts are returned with root-relative category paths
- **AND** selecting one result does not replace the configured root with its parent

### Requirement: Durable Imported Component Identity

Import SHALL persist a typed imported-component descriptor, content-addressed
managed donor and runtime assets, extracted evidence, bindings, parameters, and
runtime identity on the created thread version.

#### Scenario: Import creates pending authoring work

- **WHEN** a supported foreign part imports successfully
- **THEN** its visible version contains a `(freecad-component ...)` descriptor
- **AND** the calculated STL remains a managed Viewer/runtime asset, not source
- **AND** the copied donor and evidence remain associated with the version
- **AND** authoring state becomes `analysisPending`
- **AND** no synthetic user chat message or hidden LLM request is created

### Requirement: Explicit Materialization Strategies

Imported and generated versions SHALL share history, cache inspection, and atomic
Viewer handoff while using their correct materializer.

#### Scenario: Trusted-unit CAD imports

- **WHEN** FCStd or STEP import produces a valid printable artifact with known units
- **THEN** final bounds are shown in millimetres
- **AND** STL export is available while authoring remains pending or failed

#### Scenario: Cached imported project reopens

- **GIVEN** the version runtime preview exists for its content hash
- **WHEN** the project opens or becomes active again
- **THEN** Viewer loads that preview directly
- **AND** no render or FreeCAD process starts

#### Scenario: Imported runtime asset is missing

- **GIVEN** an imported version references a runtime asset that is missing
- **WHEN** the version opens
- **THEN** the app reports the exact missing asset
- **AND** it does not invoke Ecky render or reinterpret the component as STL source

### Requirement: Atomic Viewer Handoff

Project switching SHALL retain the last committed scene until the target artifact
is ready.

#### Scenario: Target cache inspection is pending

- **GIVEN** project A is visible
- **WHEN** project B is selected and its runtime files are being inspected
- **THEN** project A remains visible
- **AND** no empty Viewer frame or render preloader is shown
- **WHEN** project B loads successfully
- **THEN** Viewer replaces A with B atomically

### Requirement: Honest Foreign Evidence Code Mode

Code SHALL expose stored extracted evidence under a read-only `SUMMARY` tab and
the typed imported-component descriptor under an editable `COMPONENT` tab.

#### Scenario: Imported STEP opens Code

- **WHEN** the imported version has evidence
- **THEN** Code opens `SUMMARY` and shows the evidence passed to the agent
- **AND** `COMPONENT` shows `(freecad-component ...)` with identity, bindings, and parameters
- **AND** Apply and Commit Version are absent from `SUMMARY`
- **AND** Apply and Commit Version use `apply_imported_model` in `COMPONENT`
- **AND** no binary dump or source lookup through a message output is required

#### Scenario: Imported source action stays unambiguous

- **WHEN** the user selects Open CAD
- **THEN** the system opens the copied FCStd or STEP donor for the active imported message

#### Scenario: Agent commits verified Ecky

- **WHEN** normal agent authoring validates, previews, and commits an Ecky version
- **THEN** `COMPONENT` shows editable `.ecky`
- **AND** normal Apply and Commit Version actions become available

### Requirement: Structured Agent Consumption

The normal agent authoring context SHALL expose pending import state and a single
reference to token-bounded stored evidence without requiring a chat command.

#### Scenario: Agent observes pending import

- **WHEN** an agent starts or resumes on the imported thread
- **THEN** it receives source identity, measurements, evidence, and authoring state
- **AND** it may author through the normal inspect, validate, preview, commit flow
- **AND** evidence is not duplicated into chat history

### Requirement: Complete Sliceable Evidence Access

Stored foreign-component evidence SHALL remain complete. Token budgets SHALL
bound individual agent reads through stable pagination and exact detail lookup,
not truncate the evidence corpus.

#### Scenario: Large assembly exceeds one response budget

- **GIVEN** imported evidence contains 620 parts
- **WHEN** the agent reads the part index with a bounded limit
- **THEN** the response reports `total=620`, returned count, and `nextCursor`
- **AND** subsequent cursors can enumerate all 620 parts in stable order
- **AND** no part is permanently omitted

#### Scenario: Agent needs one part's full evidence

- **WHEN** the agent requests exact detail by `partId`
- **THEN** the response returns that part's complete stored evidence
- **AND** unrelated parts consume no response budget

#### Scenario: Source changes during pagination

- **WHEN** a continuation cursor's source digest differs from current evidence
- **THEN** continuation fails with a source-drift diagnostic
- **AND** no page from a different source snapshot is returned

#### Scenario: User opens Code for a large assembly

- **WHEN** the read-only CAD report renders
- **THEN** every imported part is available to the user
- **AND** agent token limits do not truncate the UI report

### Requirement: Honest Import Suggestions

The UI SHALL present import-derived facts, provenance, and warnings without an
uncalibrated numeric confidence.

#### Scenario: Deterministic bounds heuristic proposes bindings

- **WHEN** the heuristic creates a review proposal
- **THEN** the UI identifies its heuristic provenance
- **AND** no fixed or fabricated percentage is displayed
