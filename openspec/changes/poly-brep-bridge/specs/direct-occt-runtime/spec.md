# Delta for direct-occt-runtime

## ADDED Requirements

### Requirement: Polyhedral BRep import

The system SHALL accept externally generated triangle meshes and wrap them as
OCCT polyhedral BRep solids so they can participate in exact boolean operations
alongside NURBS-based solids.

#### Scenario: Import clean mesh as poly BRep

- GIVEN a valid STL triangle mesh with closed topology
- WHEN the OCCT executor processes `solidify(import-stl(path))`
- THEN a polyhedral BRep solid is created and stored in a shape slot
- AND the solid can be referenced by subsequent boolean commands

#### Scenario: Import empty mesh

- GIVEN a mesh file containing zero triangles
- WHEN the OCCT executor processes `solidify(import-stl(path))`
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

### Requirement: Direct OCCT surface operation authority

The system SHALL execute supported BRep surface operations in Direct OCCT
whenever the operand is analytic BRep or solidified poly BRep. The system MUST
NOT route analytic `chamfer` or `fillet` through the Rust mesh evaluator as a
silent fallback.

#### Scenario: Analytic chamfer

- GIVEN an analytic BRep shape and a supported `chamfer` operation
- WHEN the Direct OCCT runtime executes the plan
- THEN `BRepFilletAPI_MakeChamfer` is used
- AND the generated preview STL is only a tessellation of the analytic result
- AND the STEP export remains analytic BRep

#### Scenario: Solidified mesh-origin chamfer

- GIVEN a validated mesh-origin island has been converted through
  `solidify(import-stl(...))`
- AND the selected edge set passes mesh-origin surface-op admission
- WHEN a supported `chamfer` operation consumes that island
- THEN Direct OCCT applies the chamfer to the solidified poly BRep
- AND the STEP export is marked as faceted poly BRep

#### Scenario: Dense poly BRep chamfer rejected

- GIVEN a solidified mesh-origin poly BRep with a broad chamfer selector
- AND the selector resolves above the configured selected-edge limit
- WHEN Direct OCCT plan validation runs
- THEN the plan is rejected before `BRepFilletAPI_MakeChamfer`
- AND the error includes selected-edge count, limit, and selector
- AND no alternate mesh evaluator route is attempted

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
