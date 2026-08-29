# Delta for Ecky Voronoi Profile

## ADDED Requirements

### Requirement: Exact bounded Voronoi cells are first-class profiles

The system SHALL accept `(voronoi-cell sites index width height inset)` as a
2D sketch-producing Ecky CAD op. The selected cell SHALL equal the intersection
of centered rectangular bounds and all Euclidean nearest-site half-planes.

#### Scenario: Selected cell compiles as a sketch

- GIVEN a finite point2 site list and valid dimensions
- WHEN `voronoi-cell` is passed to `extrude`
- THEN compilation succeeds
- AND the Core node is typed as a 2D sketch.

#### Scenario: Inset creates a constant rib allowance

- GIVEN a bounded cell and positive inset
- WHEN the cell is constructed
- THEN every bounds or bisector edge moves inward by that distance
- AND the operation does not scale the polygon about its site.

#### Scenario: Cell output is deterministic

- GIVEN identical ordered sites, bounds, index, and inset
- WHEN the cell is evaluated repeatedly
- THEN vertex coordinates and ordering are identical.

### Requirement: Invalid or degenerate diagrams fail explicitly

The system SHALL reject non-integral or out-of-range indices, non-positive
bounds, negative inset, duplicate sites, and cells collapsed below three
non-collinear vertices.

#### Scenario: Duplicate sites are rejected

- GIVEN two coincident sites
- WHEN either cell is constructed
- THEN authoring fails
- AND the diagnostic names duplicate sites.

### Requirement: Native exactness has no silent fallback

The native Direct OCCT backend SHALL expand `voronoi-cell` to an exact polygon
profile before BRep operations. Unsupported interop backends SHALL return a
native-only diagnostic.

#### Scenario: Native extrusion remains analytic

- GIVEN an extruded `voronoi-cell`
- WHEN Direct OCCT renders it
- THEN planning contains a polygon followed by extrusion
- AND no mesh conversion occurs.

#### Scenario: Interop rejects honestly

- GIVEN a model using `voronoi-cell`
- WHEN an unsupported interop backend lowers it
- THEN lowering fails with a native-only diagnostic
- AND no regular-polygon or mesh approximation is substituted.

#### Scenario: Mesh-origin poly-BRep composes with Voronoi BRep

- GIVEN a closed STL imported through `solidify(import-stl(...))`
- AND an analytic solid extruded from `voronoi-cell`
- WHEN Direct OCCT applies a boolean between them
- THEN the hybrid boolean succeeds through the existing poly-BRep bridge
- AND artifact provenance is faceted poly-BRep rather than falsely analytic.
