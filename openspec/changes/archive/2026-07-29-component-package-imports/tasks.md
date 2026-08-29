## 1. Outer Source-Reference Contract

- [x] 1.1 Add one failing integration test: install an Ecky source package,
  compile `(import-component ...)`, render its alias, and assert traveled verify
  plus canonical origin evidence.
- [x] 1.2 Add failure scenarios to the same test for missing exact version and
  copy-inline `component_import` accidentally emitting a live reference.
- [x] 1.3 Run the focused integration test; confirm failures name missing host
  pre-resolution behavior.

## 2. Package Integrity

- [x] 2.1 Add failing payload-digest/install tests covering exact file
  inventory, reserved/duplicate/unsafe entries, idempotent reinstall, and
  same-coordinate mutation rejection, plus cross-model CAS deduplication.
- [x] 2.2 Implement domain-separated length-delimited payload hashing and
  atomic runtime-owned CAS/index/`ecky-integrity.json` publication.
- [x] 2.3 Add failing uninstall/GC tests proving persisted bundle locks and
  in-flight operations retain payloads while unreachable payloads are removed.
- [x] 2.4 Implement uninstall-as-index-removal, GC root scan, grace period,
  store mutation lock, and pre-delete root recheck.
- [x] 2.5 Run archive/install/resolve/retention contract tests green.

## 3. Source Resolver Seam

- [x] 3.1 Add failing unit tests for the concrete resolver API, exact lookup,
  entry-symbol fallback, AST namespace materialization, alias collisions,
  transitive-import rejection, and raw-compiler no-I/O behavior.
- [x] 3.2 Implement `component_import_runtime` contracts, production installed
  resolver, ephemeral source materialization, and
  `compile_authoring_source`; leave `SourceCompiler::compile(&str)` unchanged.
- [x] 3.3 Materialize compiled node-origin evidence from authored/resolved span
  mappings and run source/local parity plus stable-key tests green.

## 4. Lock And Version Evidence

- [x] 4.1 Add failing contract/snapshot tests for canonical lock ordering,
  `ArtifactBundle` storage, explicit RenderSnapshot identity inclusion, cache
  separation, and bundle/manifest origin equality.
- [x] 4.2 Add camelCase lock/origin contracts and thread them through render,
  ArtifactBundle, ModelManifest, message-version JSON, and restore without a new
  SQLite column.
- [x] 4.3 Add expected-lock mismatch enforcement and filesystem
  `ecky.lock.edn` mirror/apply tests, including explicit portable
  export/import with digest verification.
- [x] 4.4 Add explicit-upgrade tests proving preview/render/export never mutate
  a committed lock and successful upgrade commits a new version.

## 5. Integration Proof

- [x] 5.1 Make the outer source-reference test green through the normal render
  service and verify raw missing/mismatch errors.
- [x] 5.2 Update file-backed Ecky docs/MCP guidance with “vendor” versus “live
  reference” terminology and exact lock behavior.
- [x] 5.3 Run focused Rust tests, `cd src-tauri && cargo check`, relevant full
  tests, and `openspec validate component-package-imports --strict`.
