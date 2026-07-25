# Design: Animal Cap Catalog

## Goal

One source of truth drives animal discovery, CAD recipe provenance, desktop
catalog presentation, and landing publication.

## Artifact model

`catalogs/animal-caps/catalog.json` is authoritative. Each entry contains:

- stable id, species, display name, state, and surface flags;
- source author, page URL, download URL, license, license URL, and SHA-256;
- source mesh path and measured source bounds;
- named uniform scale and named blind-bore placement;
- shared bore-profile id;
- saved thread, message, model, source, STL, preview, and verification metadata
  for published entries.

Binary source/artifact files live beside the manifest under
`catalogs/animal-caps/assets`. Generated TypeScript is derivative and carries a
generated-file header.

## Variables

- Content format: canonical JSON plus generated TypeScript imports.
- Storage: `catalogs/animal-caps`, bundled desktop resource, landing Vite asset.
- Serving path: desktop Packages catalog and landing `/#animal-caps`.
- Backend ownership: manifest parsing, validation, resource-path resolution.
- Frontend ownership: filtering, cards, pending/error states, STL interaction.
- Editing model: manifest changes through code review; CAD history through MCP.
- Testing surface: Rust contract tests, desktop Playwright, landing Playwright.
- Export format: source STL, generated `.ecky`, verified output STL, preview PNG.
- Runtime constraints: no DB writes, no browser CAD, no hidden network fetch.
- License policy: CC0 first; other licenses require explicit redistribution data.
- Geometry policy: uniform scale only; source animal shape stays undeformed.
- Fit policy: named `presta-blind-bomb-v1` profile and named axis/mouth bindings.

## Decision

### Canonical manifest

Manifest owns product truth. `published` means every required artifact exists
and verification metadata is green. `candidate` means licensed source exists
but no generated cap is promised.

### Deterministic ingest

OBJ sources may be deterministically triangulated to STL during ingest. Ingest
preserves vertex positions and records both source URL and output SHA-256. It
does not scale, deform, decimate, or cut geometry.

### MCP CAD boundary

Mesh cutting remains Ecky work:

1. inspect licensed source and bounds;
2. render `.ecky` using `solidify(import-stl(...))`;
3. validate constraints;
4. preview;
5. structurally verify;
6. commit green version;
7. publish artifact ids into manifest.

Catalog tooling never writes app history or SQLite.

### Desktop projection

Rust reads and validates the manifest, resolves bundled/local asset paths, and
returns camelCase boundary structs. Packages UI shows only `engine: true`
entries. Candidate and published states remain visually distinct. Raw backend
errors stay visible.

### Landing projection

`scripts/sync_animal_cap_catalog.mjs` validates the manifest and generates a
typed module containing static Vite imports for `landing: true` published
entries. The landing does not hand-author duplicate cards or fetch local app
state.

## Rejected paths

- Two manifests for desktop and landing: guaranteed drift.
- Svelte-embedded catalog data: content trapped in presentation.
- Python per-animal scripts: opaque duplicate geometry pipelines.
- Runtime download from source sites: unstable, slow, license context lost.
- Thread ids as the only artifact: local DB identity is not portable content.
- Publishing candidates: false downloadable artifacts.

## Proof plan

1. Outer RED desktop test expects a Pug catalog entry and raw failure state.
2. Outer RED landing test expects one manifest-backed animal STL and source.
3. Unit RED tests reject malformed manifests and stale generated projection.
4. Add manifest, assets, backend projection, desktop shell, generated landing
   projection, and landing shell.
5. Run focused tests, full landing build, desktop browser proof, and cargo check.

