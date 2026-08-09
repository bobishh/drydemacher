# External Shapes Workbench Specification

## ADDED Requirements

### Requirement: Unified External Shapes Workflow

The system SHALL present imported and captured external geometry through one
task-owned External Shapes workbench with ordered Import, Capture, and Crop
steps. Guided BRep authoring, reconstruction, and validation SHALL remain
subordinate to Capture rather than appear as peer workflows for every external
shape.

#### Scenario: User opens External Shapes

- **WHEN** user activates Dock item `EXT`
- **THEN** window title is `EXTERNAL SHAPES`
- **AND** ordered workflow steps are available
- **AND** no duplicate Capture Dock item exists

#### Scenario: User changes away from Capture

- **GIVEN** phone Capture view is visible
- **WHEN** user selects Crop or Guides
- **THEN** camera/pairing view is removed from active content
- **AND** selected source mesh Viewer remains available to that step

### Requirement: On-Demand Phone Trust Setup

The system SHALL keep certificate installation details hidden during ordinary
pairing and SHALL reveal them through an explicit disclosure or actionable trust
failure.

#### Scenario: Pairing is pending

- **GIVEN** no trust failure is active
- **WHEN** Capture step opens
- **THEN** pairing action/state is visible
- **AND** certificate QR and Settings trust path are collapsed

#### Scenario: User requests trust setup

- **WHEN** user expands `PHONE TRUST SETUP`
- **THEN** certificate QR, exact trust URL, and Settings path become visible
- **AND** capture session identity remains unchanged

#### Scenario: Trust setup fails

- **WHEN** backend or browser returns a TLS/certificate error
- **THEN** raw error remains visible
- **AND** disclosure exposes relevant trust setup without replacing error body

### Requirement: Bound Imported Shapes

The system SHALL discover external mesh nodes from the task's canonical bound
`model.ecky` and SHALL preview the selected source mesh directly without first
running solidification or the hybrid BRep pipeline.

#### Scenario: Current model contains one imported STL

- **GIVEN** bound source contains one `import-stl` node
- **WHEN** External Shapes opens
- **THEN** Import lists that STL and selects it automatically
- **AND** Viewer loads the raw STL asset directly
- **AND** Crop and Guides retain that selected source

#### Scenario: Current model contains multiple imported meshes

- **WHEN** Import discovers multiple `import-stl` nodes
- **THEN** each node remains separately selectable by AST node identity
- **AND** no file chooser duplicates an already-bound source

#### Scenario: Bound imported file is missing

- **WHEN** an `import-stl` path cannot be read
- **THEN** Import shows the exact path and missing-file state
- **AND** Crop cannot silently use another mesh

### Requirement: Canonical Plane Crop

The system SHALL represent an applied plane crop as composable canonical
`clip-plane` source, never only as Viewer or derived-mesh state.

#### Scenario: User defines one crop plane

- **GIVEN** source mesh identity is locked
- **WHEN** user selects three non-collinear surface points and applies kept side
- **THEN** backend derives finite normalized plane coordinates
- **AND** bound `model.ecky` contains one `clip-plane`
- **AND** rendered crop matches selected side

#### Scenario: User defines two crop planes

- **GIVEN** first plane crop exists
- **WHEN** user adds and applies second plane
- **THEN** source contains two nested `clip-plane` operations
- **AND** neither plane is stored as a hidden special-case crop

#### Scenario: User edits an applied crop plane

- **GIVEN** one or more canonical `clip-plane` operations exist
- **WHEN** user selects Edit on one applied cut, picks three replacement points,
  chooses kept side, and applies
- **THEN** backend replaces that exact `clip-plane` AST node
- **AND** all other cuts retain their order and values
- **AND** Cancel leaves canonical source unchanged

#### Scenario: User removes an applied crop plane

- **GIVEN** multiple canonical `clip-plane` operations exist
- **WHEN** user selects Remove on one applied cut
- **THEN** backend replaces that exact wrapper with its source child
- **AND** remaining cuts and imported source stay intact

#### Scenario: Plane evidence is degenerate

- **WHEN** three points are duplicate or collinear, or normal is non-finite/zero
- **THEN** Apply is rejected with raw geometric reason
- **AND** canonical source and current preview remain unchanged

### Requirement: Source-Anchored Plane Picker

The system SHALL collect crop-plane points by raycasting the visible external
mesh and binding each hit to source mesh content digest, triangle index, and
barycentric coordinates.

#### Scenario: User picks a plane while rotating scan

- **WHEN** user clicks three scan locations across one or more camera views
- **THEN** stored anchors remain in source coordinates
- **AND** display transforms do not alter derived plane
- **AND** Viewer shows point count, plane preview, and kept side
- **AND** Viewer shows a normal arrow labelled Above
- **AND** kept-side controls use Above Plane and Below Plane terminology rather
  than positive and negative

#### Scenario: Plane picker is active

- **THEN** box crop overlay and transform handles are disabled
- **AND** Undo, Flip, Apply, and Cancel actions have tooltips
