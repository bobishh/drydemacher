# component-library Specification

## Purpose
TBD - created by archiving change component-unification. Update Purpose after archive.
## Requirements
### Requirement: Component extraction from existing parts

The system SHALL extract an existing part subtree into a closed
`define-component` via the compiler's binding resolution, producing
copy-inline source plus a header.

#### Scenario: Params become signature

- GIVEN a part whose body references model params `pole_od` and `clearance`
- WHEN `component_extract` runs against that part
- THEN the produced component signature contains both params with their
  metadata (defaults, min/max/step, labels) preserved.

#### Scenario: Scalar bindings become defaults

- GIVEN a part body referencing a scalar outer `let*` binding
- WHEN extraction runs
- THEN the binding becomes a signature entry whose default is its current
  evaluated value.

#### Scenario: Blocked extraction is explicit

- GIVEN a part body referencing a non-scalar outer binding (e.g. a shape)
- WHEN extraction runs
- THEN extraction fails with a blocker report naming the binding; no partial
  component is produced.

#### Scenario: Extracted component recompiles

- GIVEN any successful extraction
- WHEN the produced source is wrapped in a minimal model and instantiated
- THEN it compiles and plans without error.

### Requirement: Header contract

Each stored component SHALL carry a header with name, param manifest, tags,
provenance (threadId, messageId, sourceDigest), and referenced
named-constraint keys.

#### Scenario: Provenance recorded

- GIVEN extraction from a thread version
- WHEN the header is produced
- THEN it contains the thread id, message id, and source digest of the origin.

### Requirement: Library search returns headers only

`component_search` SHALL scan stored headers and return compact results
without component bodies; `component_get` SHALL return full copy-inline
source for one named component.

#### Scenario: Compact search results

- GIVEN a library with stored components
- WHEN an agent calls `component_search` with a query
- THEN results contain name, one-liner, param keys, and tags
- AND contain no body source.

#### Scenario: Copy-inline get

- GIVEN a stored component name
- WHEN an agent calls `component_get`
- THEN the response contains self-contained `.ecky` source pasteable into any
  model, including the component's verify clauses.

### Requirement: Exact installed source component resolution

The library SHALL resolve live references only by exact package id, exact
package version, and exact component id. Package version SHALL be the resolver
version; component version SHALL remain interface metadata.

#### Scenario: Exact coordinate resolves

- **WHEN** installed package `bike.kit@1.2.0` exports component `cage`
- **THEN** resolving `bike.kit@1.2.0:cage` returns that exact source export

#### Scenario: Different installed version is not selected

- **WHEN** source requests `bike.kit@1.2.0:cage` but only `1.3.0` is installed
- **THEN** resolution fails naming the requested canonical identity
- **AND** no version fallback or network lookup occurs

### Requirement: Source export contract

An importable source component SHALL come from a `source` visibility package
with Ecky source containing an exported top-level `define-component`.
`entrySymbol` SHALL select the export; a valid Ecky-symbol component id SHALL
be the backward-compatible fallback.

#### Scenario: Explicit export resolves

- **WHEN** component `cage` declares `entrySymbol: cage-v2` and source contains
  `(define-component cage-v2 ...)`
- **THEN** that definition is materialized under the model-local alias

#### Scenario: Arbitrary model is not inferred

- **WHEN** package source contains a model but no selected component export
- **THEN** live resolution fails with extraction/repackaging guidance
- **AND** does not infer the first part

#### Scenario: Transitive live import is rejected

- **WHEN** selected package source itself contains `import-component`
- **THEN** resolution fails with a transitive-dependency-unsupported diagnostic

### Requirement: Canonical package payload digest

The installer SHALL compute package digest from the decoded inner payload's
validated regular-file entries using domain prefix
`ecky-package-payload-v1\0`, normalized path-byte ordering, and length-delimited
path/content bytes. Outer envelope files and runtime integrity metadata SHALL
not enter the digest.

#### Scenario: Exact digest file set

- **WHEN** package digest is computed
- **THEN** raw inner `ecky-package.json` and every inner source/asset regular
  file are included
- **AND** outer `ecky-header.json`, outer `ecky-payload.b64`, and generated
  `ecky-integrity.json` are excluded

#### Scenario: Reserved or ambiguous entries fail

- **WHEN** payload contains `ecky-integrity.json`, duplicate normalized paths,
  traversal, symlink, or non-UTF-8 name
- **THEN** package validation fails before digest publication or extraction

#### Scenario: Integrity sidecar cannot self-hash

- **WHEN** install publishes `ecky-integrity.json`
- **THEN** it records package digest and ordered per-file inventory
- **AND** it is not part of its own package digest input

### Requirement: Immutable installed package coordinate

One package id/version coordinate SHALL identify one canonical payload digest.

#### Scenario: Same digest reinstall is idempotent

- **WHEN** identical payload bytes reinstall at an existing coordinate
- **THEN** installation succeeds without changing resolved content

#### Scenario: Different digest reinstall is rejected

- **WHEN** different payload bytes target an existing coordinate
- **THEN** installation fails before extraction
- **AND** existing content remains intact

### Requirement: Shared content-addressed package storage

Validated package payloads SHALL be stored once in a runtime-owned global store
keyed by package payload digest. Models SHALL retain dependency locks rather
than per-model package copies, link trees, or mutable package directories.

#### Scenario: Models share one payload

- **WHEN** multiple model versions lock the same package payload digest
- **THEN** they resolve the same immutable global store entry
- **AND** no per-model dependency directory is created

#### Scenario: Locked resolution ignores mutable discovery index

- **WHEN** a committed version resolves a dependency with an expected digest
- **THEN** resolution reads that digest from the content-addressed store
- **AND** a changed or absent coordinate index cannot redirect the version

### Requirement: Rooted package retention and collection

The runtime SHALL treat installed coordinate index entries, persisted
artifact-bundle dependency locks, and in-flight render/export pins as
garbage-collection roots. Library uninstall SHALL remove discovery metadata
without deleting rooted payloads.

#### Scenario: Uninstall preserves historical model

- **WHEN** a package coordinate is uninstalled while a committed model version
  locks its payload digest
- **THEN** the coordinate disappears from new unlocked resolution
- **AND** the committed version continues to render and export from the store

#### Scenario: Garbage collection deletes only unreachable payload

- **WHEN** no coordinate index, persisted model lock, or in-flight operation
  references a payload after the grace period
- **THEN** GC may delete that payload under the store mutation lock
- **AND** it rechecks roots immediately before deletion

### Requirement: Explicit dependency upgrades

Dependency locks SHALL be immutable during open, preview, render, and export.
Installing a newer package version SHALL NOT update existing model versions.

#### Scenario: New package version does not alter model

- **WHEN** a newer package version is installed
- **THEN** an existing committed model keeps its exact locked digests
- **AND** its geometry and export inputs remain unchanged

#### Scenario: Upgrade creates a model version

- **WHEN** an explicit dependency upgrade resolves and previews successfully
- **THEN** commit creates a new model version with a new dependency lock
- **AND** the previous version retains its prior lock

### Requirement: Dependency lock storage and identity

Successful live resolution SHALL produce a canonical dependency lock containing
package id/version/digest and component id/entry symbol/payload digest.
Committed version ownership SHALL be
`Message.artifactBundle.componentDependencyLock` and
`componentDependencyLockDigest`.

#### Scenario: Version persists lock without DB schema change

- **WHEN** a live-reference model version commits
- **THEN** its message artifact bundle JSON contains lock and lock digest
- **AND** no dedicated SQLite column is required

#### Scenario: Snapshot and cache include lock digest

- **WHEN** render snapshot identity or artifact content hash is built
- **THEN** it includes `componentDependencyLockDigest`
- **AND** equal source/params with different dependency locks cannot reuse one
  artifact

#### Scenario: Locked mismatch blocks mutation

- **WHEN** expected package/payload digest differs from installed inventory
- **THEN** preview, render, and commit fail naming the canonical identity
- **AND** the committed lock is not rewritten

#### Scenario: Filesystem project mirrors same lock

- **WHEN** a live-reference version exports to a filesystem project
- **THEN** identical canonical lock bytes are written to `ecky.lock.edn`
- **AND** project apply supplies those bytes as expected lock

#### Scenario: Portable project contains locked payloads

- **WHEN** portable export is explicitly requested
- **THEN** it includes each locked package payload by digest
- **AND** import verifies those payloads before publishing them to the shared
  store
- **AND** export/import does not rewrite the dependency lock

### Requirement: Imported component provenance sidecars

Package provenance SHALL remain outside Core IR. Runtime SHALL materialize
node-origin evidence after compilation and persist equivalent import-origin
records in ArtifactBundle and ModelManifest.

#### Scenario: Node origin survives compile

- **WHEN** package component expansion produces Core nodes
- **THEN** transient evidence maps their node ids to canonical identity, alias,
  payload digest, and authored/resolved spans

#### Scenario: Version artifacts persist origin

- **WHEN** render artifacts are produced
- **THEN** bundle and manifest contain matching component import-origin records
- **AND** CoreProgram public structs remain unchanged

### Requirement: Provenance-backed STEP component admission

A live STEP component SHALL reference package-local `.step`/`.stp` bytes whose
digest matches installed inventory and SHALL carry package geometry provenance.

#### Scenario: Valid STEP payload resolves

- **WHEN** locked STEP bytes and declared `analyticBrep`,
  `facetedPolyBrep`, or `hybrid` provenance match installed evidence
- **THEN** component resolution returns a StepAsset payload

#### Scenario: Missing provenance requires repackaging

- **WHEN** legacy STEP component lacks geometry provenance
- **THEN** live resolution fails with repackaging guidance
- **AND** does not infer `analyticBrep` from `.step`

#### Scenario: Payload mutation blocks resolution

- **WHEN** STEP bytes differ from locked payload digest
- **THEN** resolution fails before native execution

### Requirement: STEP dependency-lock evidence

STEP component lock entries SHALL record `payloadKind=step`, payload digest,
and declared geometry representation.

#### Scenario: STEP lock controls cache identity

- **WHEN** equal authored source resolves against different STEP bytes or
  representation evidence
- **THEN** dependency lock digests differ
- **AND** render artifacts cannot be shared

