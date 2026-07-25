# Proposal: Animal Cap Catalog

## Why

Turn one-off animal valve-cap experiments into a curated, reproducible catalog.
One manifest owns source provenance, fit recipe, saved Ecky artifact identity,
and publication surfaces. The desktop app and landing consume subsets from that
manifest instead of maintaining unrelated card lists.

## What Changes

- Add a canonical repo-owned animal-cap manifest.
- Record licensed source meshes, immutable source hashes, uniform transforms,
  blind Presta bore recipe metadata, and generated artifact provenance.
- Publish complete entries through a read-only desktop catalog.
- Generate the landing showcase module from the same manifest.
- Ship one end-to-end Pug Presta cap as the first published proof.
- Keep incomplete animals visible only as catalog candidates.

## Out of Scope

- Writing app history or SQLite from a catalog script.
- Browser-side CAD compilation.
- Python mesh mutation.
- Pretending candidate animals have generated, verified artifacts.
- Automatically committing generated Ecky versions without MCP verification.
- Supporting arbitrary licenses whose redistribution terms are unclear.

## Proof

- Strict OpenSpec validation.
- Manifest validator rejects duplicate ids, missing attribution, missing source
  hashes, anonymous fit offsets, and published entries with missing artifacts.
- Desktop Playwright flow shows the published subset and a truthful empty/error
  state.
- Landing Playwright flow loads the same published Pug STL and source links.
- Landing build and Rust `cargo check` pass.
