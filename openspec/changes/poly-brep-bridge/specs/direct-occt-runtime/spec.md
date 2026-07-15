# Delta for direct-occt-runtime

## ADDED Requirements

### Requirement: Polyhedral BRep import

The system SHALL accept externally generated triangle meshes and wrap them as
OCCT polyhedral BRep solids so they can participate in exact boolean operations
alongside NURBS-based solids.

#### Scenario: Import clean mesh as poly BRep

- GIVEN a valid triangle mesh file (STL or OBJ) with closed topology
- WHEN the OCCT runner processes an `import_poly_mesh` command
- THEN a polyhedral BRep solid is created and stored in a shape slot
- AND the solid can be referenced by subsequent boolean commands

#### Scenario: Import empty mesh

- GIVEN a mesh file containing zero triangles
- WHEN the OCCT runner processes an `import_poly_mesh` command
- THEN the command fails with a descriptive error
- AND no partial shape is stored

### Requirement: Hybrid boolean operations

The system SHALL execute boolean operations (cut, fuse, common) over solids of
mixed representation — exact NURBS BRep and polyhedral BRep — in a single
OCCT operation.

#### Scenario: Cut exact solid with poly shell

- GIVEN an exact BRep solid (from extrude) and a polyhedral BRep shell (from
  displaced mesh)
- WHEN a `cut` boolean is executed
- THEN the result is a valid OCCT solid
- AND the result preserves exact faces where no intersection occurred

#### Scenario: Fuse exact solids with poly displacement

- GIVEN multiple exact BRep solids and a polyhedral displacement shell
- WHEN a `fuse` boolean is executed
- THEN the result is a single solid containing both exact and poly faces

### Requirement: Tessellation at partition boundary

The system SHALL tessellate OCCT exact solids into triangle meshes when
crossing from exact BRep operations to mesh-only operations, at a controllable
tessellation density.

#### Scenario: Tessellate extrude result

- GIVEN an exact BRep solid produced by the pre-boundary sub-tree
- WHEN the partition boundary is crossed
- THEN the solid is tessellated into a triangle mesh
- AND the mesh is passed to the mesh-only operation chain

### Requirement: STEP export with poly faces

The system SHALL export STEP files from hybrid solids where exact faces remain
NURBS-represented and polyhedral faces are exported as faceted BRep faces.

#### Scenario: Hybrid part STEP export

- GIVEN a hybrid solid with both exact and poly faces
- WHEN STEP export runs
- THEN the STEP file contains exact NURBS faces for non-displaced geometry
- AND the STEP file contains faceted faces for displaced geometry

#### Scenario: Poly-only part STEP suppression

- GIVEN a part whose geometry is entirely polyhedral (no exact faces)
- WHEN manifest generation runs
- THEN the part is marked as STL-only or poly-tagged in the manifest
