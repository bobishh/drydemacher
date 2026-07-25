# animal-cap-catalog Specification

## Purpose
TBD - created by archiving change animal-cap-catalog. Update Purpose after archive.
## Requirements
### Requirement: Canonical Animal Cap Manifest

The system SHALL derive desktop and landing animal-cap catalogs from one
schema-versioned manifest. It SHALL NOT maintain independent hand-authored
product lists.

#### Scenario: Published entry is complete

- GIVEN an animal entry is marked `published`
- WHEN catalog validation runs
- THEN source provenance, immutable source hash, named fit recipe, green
  verification metadata, Ecky source, output STL, and preview paths all exist

#### Scenario: Candidate remains honest

- GIVEN an animal source is licensed but has no verified generated artifact
- WHEN catalog projections run
- THEN the entry may remain a candidate
- AND neither desktop nor landing offers a generated STL for it

### Requirement: Preserved Animal Geometry

Animal source geometry SHALL use uniform scale and rigid transforms only before
the named valve bore is subtracted.

#### Scenario: Fit recipe is declared

- GIVEN a generated animal cap
- WHEN its catalog recipe is inspected
- THEN scale is one scalar value
- AND bore profile, axis, mouth, and axis height use named fields
- AND no anonymous per-axis deformation exists

### Requirement: MCP-Owned CAD Publication

Catalog tooling SHALL NOT write app history or SQLite. A generated artifact
becomes publishable only after MCP preview and green structural verification.

#### Scenario: Generation is green

- GIVEN an MCP preview has zero non-manifold edges and passes authored checks
- WHEN the version is committed through MCP
- THEN its thread, message, model, and verification metadata may be recorded in
  the manifest

#### Scenario: Generation is red

- GIVEN verification fails
- WHEN projection generation runs
- THEN the entry cannot be published

### Requirement: Desktop Catalog Projection

The desktop app SHALL expose the engine subset from the canonical manifest
through a typed Rust command with camelCase boundary fields.

#### Scenario: Catalog loads

- GIVEN one published entry has `engine: true`
- WHEN Packages opens
- THEN the entry label, species, fit profile, source license, and artifact state
  are visible

#### Scenario: Catalog read fails

- GIVEN manifest parsing or path resolution fails
- WHEN Packages opens
- THEN the raw backend error is visible
- AND no generic API-key message replaces it

### Requirement: Landing Catalog Projection

The landing SHALL expose only `landing: true` published entries generated from
the canonical manifest.

#### Scenario: Published animal loads

- GIVEN the Pug entry is published for landing
- WHEN `/#animal-caps` renders
- THEN its real STL loads in the viewer
- AND source/STL downloads target the manifest-backed static assets

#### Scenario: Asset load fails

- GIVEN the animal STL request fails
- WHEN the viewer settles
- THEN pending state ends
- AND the failed asset is identified
- AND retry is available

