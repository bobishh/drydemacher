# Proposal: Steel Data Config Format

## Intent

Replace internal JSON-centric configuration and internal prompt shape summaries
with strict data-only EDN represented as Steel values. This gives homoiconicity
without making user files Steel programs.

`config.edn` becomes sole runtime persistence. A one-shot startup backfill may
import `config.json` only when `config.edn` is absent. It atomically publishes,
fsyncs, reopens, and typed-verifies canonical EDN before deleting JSON. No
runtime fallback, dual-read, backup JSON, or JSON write remains.

## Scope

- Custom EDN reader, explicit typed Steel-value conversion, canonical writer,
  and fixed v1 schemas.
- Config load/save/migration concurrency and location-rich, secret-safe
  diagnostics.
- Process-local and advisory interprocess serialization with explicit cleanup
  pending state.
- Canonical internal shape-summary schema used after extractor normalization.

## Non-goals

- Steel parser syntax, evaluation, macros, symbols, lists, quotes, or executable
  config.
- Changing existing secret fields or their storage/policy.
- Replacing external JSON adapters. MCP JSON-RPC; Tauri/Specta invokes/events;
  provider REST; Direct-OCCT/Build123d/FreeCAD subprocess plans/reports; project
  mirror `ecky-project`/`ecky.lock`; package manifests,
  archives/indexes; runtime bundle/manifest; DB legacy JSON columns stay JSON.
- Claiming removal of `serde_json`; transitive dependencies retain it.

## Policy reconciliation

Current repository guidance names `app_config_dir/config.json`. This accepted
change replaces that contract with `app_config_dir/config.edn`. Implementation
must inventory/update docs, tests, hardcoded references, and AGENTS persistence
authority; after migration/cleanup, config JSON may appear only in the backfill
detector, typed importer, and stale-file cleanup.
