## ADDED Requirements

### Requirement: External contracts remain JSON adapters

The system SHALL retain JSON at MCP JSON-RPC, Tauri/Specta invokes/events,
provider REST, Direct-OCCT/Build123d/FreeCAD subprocess plans/reports, project
`ecky-project`/`ecky.lock`, package manifests/archives/indexes, runtime
bundle/manifest, and DB legacy JSON columns. Internal EDN explicitly translates
at boundaries. Only config persistence and internal prompt shape summary migrate.
`config.json` is not an external runtime contract: it is allowed only for the
one-shot typed backfill importer and stale-file cleanup. No `serde_json` removal
claim, including transitive uses.

#### Scenario: External adapter does not leak EDN syntax

- GIVEN a canonical internal shape summary or config value
- WHEN it crosses an external JSON contract
- THEN the adapter emits the contract's JSON shape
- AND internal EDN tokens and Steel-only values do not appear on the wire.
