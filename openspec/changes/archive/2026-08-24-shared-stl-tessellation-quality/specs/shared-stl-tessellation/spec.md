# Shared STL Tessellation Quality Specification

## Purpose

Define one measured tessellation contract for binary STL generated from exact
Direct OCCT solids. The contract covers the STL consumed by both the viewer
and print/export workflows.

## ADDED Requirements

### Requirement: Shared named tessellation policy

The system SHALL use one named Direct OCCT STL tessellation policy for merged
preview STL and per-part STL output. The initial policy SHALL use linear
deflection `0.04 mm` and derive angular deflection as
`clamp(linear * 5 + 0.005, 0.005, 0.1)` radians. Minimum and maximum bounds
MUST be named policy values, not inline literals.

#### Scenario: Generated native export uses the selected policy

- GIVEN a supported exact OCCT model
- WHEN the generated native export source is emitted
- THEN its `BRepMesh_IncrementalMesh` call uses the named linear policy and the
  derived angular value
- AND the source does not contain the old `0.25 rad` angular default.

#### Scenario: Standalone native runner uses the selected policy

- GIVEN the standalone Direct OCCT runner builds an exact model
- WHEN it writes merged or per-part STL
- THEN its meshing call uses values equivalent to the selected policy
- AND the runner policy cannot silently drift from the generated export policy.

### Requirement: Curved small-part fidelity

The system SHALL produce materially denser STL tessellation for a sphere of
radius `3 mm` than the old `0.25 rad` policy, while preserving the analytic
shape's bounds and topology.

#### Scenario: Small sphere is print-safe and manifold

- GIVEN a native model containing `(sphere 3)`
- WHEN the shared STL is generated
- THEN the STL is binary and non-empty
- AND its triangle count is at least `600`
- AND its per-axis bounds remain within `0.02 mm` of the expected sphere bounds
- AND its connected-component count is unchanged
- AND its non-manifold edge count is zero.

#### Scenario: Cylinder receives the same curved-surface policy

- GIVEN a native model containing `(cylinder 2 6)`
- WHEN the shared STL is generated
- THEN its triangle count is within the recorded fixture band
- AND its radius and height bounds remain within `0.02 mm`
- AND its non-manifold edge count is zero.

### Requirement: Planar geometry non-regression

The system SHALL avoid unbounded tessellation growth for planar exact solids
when the curved-surface quality policy is tightened.

#### Scenario: Box remains a compact planar mesh

- GIVEN a native model containing `(box 4 8 6)`
- WHEN the shared STL is generated
- THEN its bounds are within `0.02 mm` of `[0, 0, 0]..[4, 8, 6]`
- AND its triangle count is exactly `12`
- AND its non-manifold edge count is zero.

### Requirement: Artifact and runtime budgets

The system SHALL enforce deterministic budgets for the fixture set: the native
three-fixture render SHALL complete within `3 s`, the sphere STL SHALL be no
larger than `100,000` bytes, the cylinder STL SHALL be no larger than `20,000`
bytes, and the box STL SHALL remain exactly `12` triangles / `684` bytes.

#### Scenario: Tessellation stays within budgets

- GIVEN the sphere, cylinder, and box fixture contract
- WHEN the selected policy generates the same fixtures
- THEN tests report triangle counts, bytes, bounds, topology, and wall time
- AND each metric remains within its configured budget.

#### Scenario: Budget regression is actionable

- GIVEN a fixture exceeds a configured size or time budget
- WHEN the quality test fails
- THEN the failure names the fixture, old value, new value, and configured
  budget
- AND the implementation does not silently fall back to the old coarse policy.

### Requirement: Export preserves generated triangles

The system SHALL preserve the generated shared STL triangles through multipart
STL export. Export may transform or repackage triangles, but SHALL NOT reduce
their count or apply a second coarse tessellation.

#### Scenario: Multipart export keeps dense per-part STL

- GIVEN a rendered multipart exact model with a curved part
- WHEN multipart STL export runs
- THEN the exported part contains the generated part's triangle count
- AND the exported part remains binary and manifold.

### Requirement: STEP remains analytic

The system SHALL leave analytic STEP generation independent from the STL
tessellation policy.

#### Scenario: Tightening STL does not facet STEP

- GIVEN an exact OCCT sphere or cylinder
- WHEN the selected STL policy changes within its approved range
- THEN STEP export remains available and analytic
- AND only STL triangle generation changes.
