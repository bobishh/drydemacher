# Surface Trim for External Shapes Specification

## ADDED Requirements

### Requirement: General Surface-Following Trim

The system SHALL provide Surface Trim in the External Shapes Crop step for
selecting a region of an arbitrary external triangle mesh using a closed contour
that follows the source surface.

#### Scenario: User traces a non-planar boundary

- **GIVEN** an imported or captured source mesh is selected
- **WHEN** user activates `TRACE SURFACE` and places at least three ordered points
- **THEN** each point is anchored to source digest, triangle, and barycentrics
- **AND** paths between points remain on the mesh surface
- **AND** points are not reduced to one fitted plane

#### Scenario: Surface Trim is active

- **THEN** box crop, plane crop, guided reconstruction, and conflicting selection
  handlers are inactive
- **AND** raw source mesh remains visible
- **AND** no OCCT render occurs for pointer-hover path previews

### Requirement: Deterministic Intelligent Path

The system SHALL compute deterministic shortest or feature-aware surface paths
between hard points and SHALL expose the resulting contour before Apply.

#### Scenario: Cursor moves after a committed point

- **WHEN** a valid surface hit changes
- **THEN** Viewer requests a throttled path preview
- **AND** latest preview id wins over stale responses
- **AND** Viewer displays computed path without changing canonical source

#### Scenario: Geometry contains a crease

- **GIVEN** feature path mode is active
- **WHEN** two candidate paths have similar length
- **THEN** versioned feature weights prefer the relevant crease path
- **AND** equal costs resolve by stable mesh indices

### Requirement: Valid Closed Surface Loop

The system SHALL close and validate the complete ordered contour before allowing
region selection or Apply.

#### Scenario: User closes a valid loop

- **WHEN** final-to-first path creates one non-self-intersecting partitioning loop
- **THEN** Viewer marks loop Closed
- **AND** system requests a region-to-keep seed

#### Scenario: Loop is invalid

- **WHEN** loop is duplicate, disconnected, self-intersecting, non-manifold, or
  does not partition the selected component
- **THEN** closure is rejected with raw reason and involved segments
- **AND** canonical source and current successful model remain unchanged

### Requirement: Explicit Retained Region

The system SHALL use a source-anchored seed to choose the retained region and
SHALL not infer keep side from camera direction, largest area, or component size.

#### Scenario: User selects region to keep

- **GIVEN** a valid closed loop
- **WHEN** user clicks one surface point in a bounded region
- **THEN** Viewer previews only the seed-containing region as retained
- **AND** other regions and disconnected components are excluded

#### Scenario: Keep seed is invalid

- **WHEN** seed is stale, on the boundary, outside source identity, or cannot
  identify exactly one region
- **THEN** selection is rejected with raw reason
- **AND** Apply remains disabled

### Requirement: Exact Triangle Cut

The system SHALL refine the surface contour through crossed triangles and SHALL
not restrict the applied boundary to selecting whole input triangles.

#### Scenario: Contour crosses triangle interiors

- **WHEN** valid contour traverses a coarse triangulated surface
- **THEN** crossed edges and triangles are split at deterministic boundary points
- **AND** retained source faces preserve orientation
- **AND** emitted boundary matches visible preview within documented tolerance

#### Scenario: Cut produces invalid topology

- **WHEN** cut output has unresolved duplicate, inverted, or non-manifold elements
- **THEN** Apply fails with measured topology report
- **AND** system does not silently heal or save a derived STL

### Requirement: Explicit Cap Policy

The system SHALL require an explicit Open, Flat, or Surface Fill cap policy and
SHALL never silently substitute one policy for another.

#### Scenario: User chooses Open

- **THEN** output retains its boundary and is reported open
- **AND** later `solidify` rejects it unless another canonical operation closes it

#### Scenario: User chooses Flat

- **WHEN** boundary fits one plane within tolerance and its projection is valid
- **THEN** system reports RMS and maximum deviation
- **AND** emits an oriented planar triangulated cap

#### Scenario: Flat boundary is unsuitable

- **WHEN** deviation exceeds tolerance or projection self-intersects
- **THEN** Flat fails with measured reason
- **AND** system does not fall back to Surface Fill

#### Scenario: User chooses Surface Fill

- **WHEN** non-planar boundary admits a valid constrained patch
- **THEN** system emits oriented patch triangles
- **AND** rejects foldovers or non-manifold output explicitly

### Requirement: Canonical Surface Trim Source

The system SHALL represent applied segmentation as one canonical `surface-trim`
operation in bound `model.ecky` and SHALL preserve immutable input STL bytes.

#### Scenario: User applies Surface Trim

- **GIVEN** source snapshot, loop, keep seed, and cap output validate
- **WHEN** user selects Apply
- **THEN** backend inserts one `surface-trim` node around exact imported mesh node
- **AND** node contains source digest, ordered anchors, keep seed, path mode, and
  cap mode
- **AND** Viewer-only state and generated STL paths are not geometry authority

#### Scenario: User starts a second trim on the same imported source

- **GIVEN** schema v1 source already wraps the exact `import-stl` node in one
  `surface-trim`
- **WHEN** user requests another new contour for that source
- **THEN** system requires Edit or Remove of the existing trim
- **AND** does not nest source-indexed anchors against derived mesh topology
- **AND** trims on separate imported sources remain independent

#### Scenario: User reloads task

- **WHEN** bound model contains applied Surface Trim nodes
- **THEN** Crop lists each node with point count, path mode, and cap mode
- **AND** controls are reconstructed from canonical source

#### Scenario: User edits one applied trim

- **WHEN** user changes contour, seed, path mode, or cap mode and applies
- **THEN** backend replaces exact selected AST node
- **AND** unrelated nested operations remain byte-equivalent

#### Scenario: User removes one applied trim

- **WHEN** user selects Remove
- **THEN** backend replaces exact wrapper with its shape child
- **AND** imported source and unrelated operations remain intact

### Requirement: Mesh-to-BRep Continuity

The system SHALL execute Surface Trim in indexed-mesh runtime before `solidify`
and SHALL allow valid closed output to feed later BRep operations.

#### Scenario: Closed trimmed mesh is solidified

- **GIVEN** Surface Trim output is closed, oriented, and manifold
- **WHEN** canonical model evaluates `solidify(surface-trim(import-stl ...))`
- **THEN** hybrid planner executes trim before mesh-to-BRep conversion
- **AND** later difference, union, pocket, or thread operations receive the
  resulting solid

#### Scenario: Trim is not solidifiable

- **WHEN** output is open, unoriented, or non-manifold
- **THEN** failure identifies exact topology defect
- **AND** planner does not claim successful BRep conversion

### Requirement: Source Identity and Atomic Failure

The system SHALL guard preview and mutation against source digest and target
snapshot drift.

#### Scenario: Source changed after points were placed

- **WHEN** current imported bytes no longer match anchor digest
- **THEN** preview and Apply reject stale evidence
- **AND** raw expected and actual digest are available in failure detail
- **AND** file, successful preview, and history remain unchanged

#### Scenario: Apply backend fails

- **WHEN** cutting, capping, rendering, or source persistence fails
- **THEN** UI displays backend error body
- **AND** canonical source, current successful render, and history version remain
  unchanged
