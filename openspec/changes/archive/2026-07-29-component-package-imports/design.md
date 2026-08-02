## Context

Three existing facts constrain the design:

1. `language-convenience-stdlib` Phase 3 defines MCP/UI `component_import` as
   copy-inline vendoring. It inserts full `define-component` source and leaves
   a self-contained model.
2. The compiler seam is source-only:
   `ecky_render::SourceCompiler::compile(&str)` and
   `compile_to_core_program(&str)`. Package filesystem access does not belong
   inside that pure API.
3. A committed model version is an assistant `Message` whose `output`,
   `artifact_bundle`, and `model_manifest` are serialized into existing JSON
   columns. `ArtifactBundle` is runtime-owned and already embedded in
   `RenderSnapshot`.

This change covers live references to installed Ecky source components only.
Native STEP composition depends on this resolver/lock foundation but lands in
`native-step-component-import`.

## Goals / Non-Goals

**Goals:**

- Preserve copy-inline vendoring and live package reference as explicit,
  non-overlapping modes.
- Resolve exact installed source components before the existing pure compiler.
- Make `<packageId>@<version>:<componentId>` immutable and reproducible.
- Deduplicate package payloads across models without per-model dependency trees.
- Keep payloads required by committed versions alive after library uninstall.
- Name the exact dependency lock owner and persistence path.
- Preserve imported-package provenance without changing Core IR structs.
- Keep implementation small enough for one focused backend flow.

**Non-Goals:**

- STEP/STL/compiled/locked/private payload composition.
- Remote registry, download, semver ranges, `latest`, or transitive packages.
- Automatic dependency upgrades during open, preview, render, or export.
- New library/import UI.
- Changing `ecky-render::SourceCompiler`, CoreProgram public structs, or raw
  local-component compilation.

## Decisions

### 1. Preserve two distinct import modes

| Mode | Surface | Source mutation | Runtime dependency |
| --- | --- | --- | --- |
| Vendor | MCP/UI `component_import` | Inserts full source | None |
| Live reference | `(import-component ...)` | Keeps coordinate | Exact locked package |

Vendoring remains the stdlib/library convenience path. It MUST NOT emit
`(import-component ...)` or a lock. Live reference is authored language syntax
and MUST NOT copy package source into persisted authored source.

This change does not rename the existing MCP tool. Documentation always calls
it “vendor/copy-inline” and calls the language form “live reference”.

### 2. Use explicit exact syntax and local aliases

```scheme
(import-component
  "bike.bottle-holder-kit"
  :version "1.2.0"
  :component "bottle-cage"
  :as cage)

(model
  (part holder
    (cage :diameter 74)))
```

All coordinate fields and alias are mandatory literals. Canonical identity is:

```text
<packageId>@<packageVersion>:<componentId>
```

Package version is the only resolver version. Existing component version
remains interface metadata. Aliases cannot collide with imports, local
components, helpers, or reserved CAD forms.

### 3. Add a concrete host pre-resolution seam

New module: `src-tauri/src/component_import_runtime.rs`.

```rust
pub trait InstalledComponentResolver {
    fn resolve_source_component(
        &self,
        coordinate: &ComponentCoordinate,
    ) -> AppResult<ResolvedSourceComponent>;
}

pub struct ResolveAuthoringSourceRequest<'a> {
    pub authored_source: &'a str,
    pub expected_lock: Option<&'a ComponentDependencyLock>,
}

pub struct ResolvedAuthoringSource {
    pub compiler_source: String,
    pub dependency_lock: ComponentDependencyLock,
    pub import_spans: Vec<ComponentImportSpan>,
}

pub fn resolve_authoring_source(
    request: ResolveAuthoringSourceRequest<'_>,
    resolver: &dyn InstalledComponentResolver,
) -> AppResult<ResolvedAuthoringSource>;

pub struct ResolvedCompilation {
    pub program: CoreProgram,
    pub dependency_lock: ComponentDependencyLock,
    pub origins_by_node: BTreeMap<NodeId, ComponentImportOrigin>,
}
```

`resolve_authoring_source` parses top-level import forms, resolves exact
package source, and performs an AST-safe materialization into ephemeral
`compiler_source`. It namespaces package-private helpers deterministically and
binds the exported component to the requested alias. It never performs regex
replacement.

`compile_authoring_source` in the same module calls:

```text
resolve_authoring_source
  -> NativeSourceCompiler.compile(compiler_source)
  -> map compiled node spans to import origins
  -> ResolvedCompilation
```

Host render/check/lowering entrypoints that support live references call this
function, then pass its CoreProgram to existing precompiled-program backend
entrypoints. `NativeSourceCompiler`, `SourceCompiler::compile(&str)`, and
`compile_to_core_program(&str)` remain unchanged. Raw compiler use on source
containing unresolved `import-component` returns an explicit
host-resolution-required diagnostic.

The production resolver is
`InstalledLibraryComponentResolver<'a> { app: &'a dyn PathResolver }`. Tests
use an in-memory resolver.

### 4. Source package export contract

`ComponentDefinition` and public `ComponentHeader` gain optional
`entrySymbol`.

- `entrySymbol` selects the exported top-level `define-component`.
- When omitted, a valid Ecky-symbol `componentId` is the fallback.
- One package source file may contain private helpers and nested local
  components; only the selected export becomes visible under the model alias.
- Arbitrary `(model ...)` source without the export is independently renderable
  but not live-reference importable.
- Imported source cannot contain another `import-component` in this slice.

### 5. Define package digest bytes exactly

Digest name: `sha256:<hex>`. Domain prefix:

```text
ecky-package-payload-v1\0
```

Digest file set is the decoded inner payload archive:

- include every non-directory regular-file entry after safe-path validation;
- include raw `ecky-package.json` bytes and every source/asset byte;
- exclude outer-envelope `ecky-header.json` and `ecky-payload.b64`;
- reserve `ecky-integrity.json`; payloads containing that path are rejected;
- symlinks, duplicate normalized paths, traversal paths, and non-UTF-8 archive
  names are rejected.

Normalize separators to `/`. Sort entries by normalized UTF-8 path bytes.
For each entry, append to SHA-256:

```text
u64be(path_byte_length)
path_bytes
u64be(content_byte_length)
content_bytes
```

The installer writes runtime-owned `ecky-integrity.json` after validated
extraction. It stores package digest plus ordered `{path, sha256}` inventory.
That sidecar is not digest input, preventing self-reference. Resolve verifies
the referenced source against inventory; an explicit integrity operation may
rehash the full inventory.

Validated payloads publish into the global content-addressed store defined in
Decision 7. Same coordinate + same digest is idempotent. Same coordinate +
different digest is rejected before publication. Existing content remains
intact.

### 6. Name lock ownership and snapshot storage

Contracts live in `contracts/component.rs`:

```text
ComponentDependencyLock
  schemaVersion
  dependencies[]
    packageId
    version
    packageDigest
    components[]
      componentId
      entrySymbol
      payloadDigest
```

Canonical ordering: dependencies by `(packageId, version)`, components by
`componentId`. Compact serde JSON bytes produce
`componentDependencyLockDigest`.

Runtime-owned storage:

```text
Message.artifactBundle.componentDependencyLock
Message.artifactBundle.componentDependencyLockDigest
```

No SQLite column is added: `messages.artifact_bundle` already stores the bundle
JSON. `LastDesignSnapshot` naturally carries the same bundle.

`RenderSnapshot` reads the lock from `ArtifactBundle`; its `SnapshotIdentity`
adds `componentDependencyLockDigest` explicitly. `ArtifactBundle.contentHash`
and render cache keys also include that digest. A lock mismatch blocks
preview/render/commit and never rewrites an existing committed lock.

Filesystem project export mirrors the lock as `ecky.lock.json`; project apply
passes it as `expected_lock`. Unlocked first resolution returns a candidate
lock; successful version commit owns persistence.

### 7. Use one global content-addressed store, not per-model dependency trees

Runtime-owned layout under the existing component-library root:

```text
component-library/
  index/<escaped-package-id>/<escaped-version>.json
  store/sha256/<hex-digest>/
    ecky-package.json
    <payload files>
    ecky-integrity.json
```

The store directory is keyed only by the package payload digest. One payload is
stored once per application data directory even when many models and package
coordinates reference it. Models contain no `node_modules`-like tree, hard
links, or copied dependency source. Their artifact bundle contains only the
canonical dependency lock.

The coordinate index is mutable discovery metadata mapping exact
`packageId@version` to one payload digest. Unlocked authoring resolves through
the index. A committed version resolves its expected digest directly from the
store; the mutable index cannot silently redirect it.

Library uninstall removes the coordinate index entry, preventing new unlocked
resolution, but does not remove a payload referenced by a committed version.
Garbage collection treats these as roots:

- every installed coordinate index entry;
- every dependency lock in a persisted `Message.artifactBundle`;
- every explicit in-flight render/export pin.

GC follows lock digests to payload directories and deletes only unreachable
payloads after a grace period. It acquires the component-store mutation lock
and rechecks roots immediately before deletion. `LastDesignSnapshot` needs no
separate root because it points to an already persisted message bundle.

Normal filesystem project export writes source plus `ecky.lock.json`; it may
require the shared store or later package retrieval on another machine.
Explicit portable export additionally writes immutable package payloads under
`dependencies/sha256/<hex>.eckypkg`. Import verifies each payload against the
lock before publishing it into the global store. Portable export never changes
the model lock.

Dependency changes are explicit. Installing a newer package does not alter any
model. An upgrade operation resolves a new digest, previews it, and commits a
new model version with a new lock. Open, preview, render, STEP export, and STL
export never rewrite dependency locks.

### 8. Store provenance outside Core IR

Transient provenance:

```text
ResolvedCompilation.originsByNode: NodeId -> ComponentImportOrigin
```

`ComponentImportOrigin` contains canonical identity, local alias, package and
payload digests, authored call-site span, and resolved source span. The
pre-resolution span table is materialized against compiler-produced node spans
after compile.

Persisted evidence:

```text
ArtifactBundle.componentImportOrigins[]
ModelManifest.componentImportOrigins[]
```

Each record contains canonical identity, alias, payload digest, part ids, and
node ids. This gives diagnostics, viewer/manifest consumers, and later topology
work package truth without adding package fields to CoreNode/CoreProgram.

## Risks / Trade-offs

- [Ephemeral source materialization changes spans] → Preserve an explicit
  authored-span/resolved-span map and test node-origin attribution.
- [Not every compile call uses the host wrapper] → Raw compiler rejects
  unresolved imports clearly; migrate package-aware top-level entrypoints to
  `compile_authoring_source`, not every unit helper.
- [Legacy installed packages lack integrity sidecars] → Require one explicit
  baseline/integrity action before live-reference use; never baseline during a
  committed render.
- [Uninstall breaks historical versions] → Remove only the coordinate index;
  committed locks remain GC roots for immutable store payloads.
- [GC races render/export] → Hold an in-flight digest pin and recheck roots
  under the store mutation lock before deletion.
- [Lock-only filesystem project moves to another machine] → Offer explicit
  portable export containing verified payload archives.
- [Package private-name collisions] → AST namespace transform keyed by payload
  digest; never textual replacement.
- [Bundle and manifest provenance drift] → Build both from one
  `ResolvedCompilation` evidence vector and validate equality.

## Migration Plan

1. Land copy-inline/live-reference terminology in the active stdlib change.
2. Land package digest inventory, global CAS, and immutable coordinate index.
3. Land contracts plus resolver/pre-resolution tests.
4. Land source live-reference render path and bundle/manifest evidence.
5. Land lock snapshot/project mirror persistence and GC roots.
6. Land explicit portable project export/import.

Rollback disables the host pre-resolution entrypoint. Copy-inline models,
existing compiler calls, installed package rendering, and stored bundles with
optional new fields remain readable.

## Open Questions

None for source live references. STEP enters through the dependent
`native-step-component-import` change.
