## Why

`define-component` and the library already support self-contained copy-inline
reuse, while installed packages already expose exact
`packageId/version/componentId` lookup. Ecky needs a second, explicit live
reference mode with deterministic resolution, immutable package bytes, and
version-owned dependency evidence; it must not silently replace the existing
copy-inline `component_import` workflow.

## What Changes

- Keep two distinct reuse modes:
  - MCP/UI `component_import` vendors self-contained source into the model and
    creates no runtime dependency.
  - authored `(import-component ...)` preserves an exact live package
    coordinate, binds a local alias, and requires a dependency lock.
- Add a host-owned component-import pre-resolution API before the existing
  `SourceCompiler::compile(&str)` seam. Keep the pure compiler API unchanged.
- Support installed Ecky source packages in this change. Resolve and
  AST-materialize their exported `define-component` definitions before normal
  compilation.
- Define exact package digest input, immutable install-coordinate behavior,
  lock ownership, snapshot storage, and source/project mirror behavior.
- Persist package/node provenance outside Core IR in bundle/manifest evidence.
- Leave native STEP package components to the dependent
  `native-step-component-import` change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `component-surface`: Add explicit live package references while preserving
  copy-inline vendoring as a separate operation.
- `component-library`: Add source-package resolution, exact integrity hashing,
  dependency locking, immutable coordinates, and provenance evidence.

## Impact

- New `component_import_runtime` host module.
- Existing `ecky_cad_host::source_compiler` orchestration; no signature change
  to `ecky-render::SourceCompiler` or `compile_to_core_program(&str)`.
- Component package contracts/install resolver.
- `ArtifactBundle`, `ModelManifest`, render snapshot identity, message-version
  JSON, and filesystem project mirror.
- Ecky language docs and focused Rust integration tests. No new UI surface.
