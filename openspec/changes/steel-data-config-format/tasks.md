# Tasks: Steel Data Config Format

## Guardrails

- Implementation evidence recorded here: strict parser, typed Config codec, EDN
  persistence, migration, cleanup retry, and single-process/advisory locking now
  exist. Remaining unchecked work still requires its stated proof.
- Future work: outer BDD RED before inner unit cycles; Rust snake_case and Tauri
  payload camelCase; run `cd src-tauri && cargo check` before success.
- Preserve current config, existing secret fields, and external JSON protocol
  adapters. Legacy config JSON exists only for the one-shot importer/cleanup; no
  `serde_json` removal claim.

## 1. Strict Steel data core

- [x] 1.1 RED: token/canonical fixtures cover keyword values/namespaces, EOF,
  comma/comment separators, JSON escapes, ints/floats, canonical ordering/spelling.
- [x] 1.2 RED: hostile limits reject exact forbidden forms plus every budget.
- [x] 1.3 Implement custom bounded reader, `SteelDataValue`, and restrictive
  immutable Steel conversion; never invoke Steel parser/Engine/eval.
- [x] 1.4 RED: hostile syntax/value/budget matrix rejects with location-rich,
  secret-safe errors: executable forms, tags/sets/dotted pairs, duplicate keys,
  NaN/Infinity, depth/node/string/collection excess, and integer tokens outside
  the i64 domain.

## 2. Configuration persistence

- [x] 2.1 RED: inventory/test current production JSON camelCase aliases, required
  fields, defaults, enum aliases, and full root/nested Config v1 field table;
  normalized EDN Config equivalence.
- [x] 2.2 RED: unknown schema/version/fields fail; invalid EDN fails closed with
  no JSON rescue; missing-both yields unsaved defaults.
- [x] 2.3 Implement startup one-shot JSON backfill only when EDN is absent:
  typed JSON parse, startup migrations, canonical same-directory atomic EDN
  write/fsync/rename, reopen typed equivalence, then JSON deletion.
- [x] 2.4 Outer BDD RED: `save_config` camelCase JSON invoke atomically writes EDN
  only; durable reopen proves result and creates no JSON artifact.
- [x] 2.5 RED: pre-rename and parent-fsync durability failures preserve in-memory
  state; no Config swap occurs before fsync/rename/parent-fsync/reopen/equivalence.
- [x] 2.6 RED: valid JSON backfill publishes equivalent EDN before deleting JSON;
  JSON deletion failure enters READY_WITH_CLEANUP_PENDING, loads EDN, surfaces a
  redacted warning/status, and retries under lock each startup/successful save.
- [x] 2.7 RED: invalid JSON with absent EDN returns a staged, location-rich,
  secret-safe migration error, preserves
  JSON, creates no EDN, and fails load closed; valid EDN plus stale JSON deletes
  JSON without reading it; invalid EDN neither reads nor deletes JSON.
- [x] 2.8 Inventory/update docs/tests/hardcoded config.json references; amend
  AGENTS persistence authority to config.edn. Add artifact scan proving no path
  reads/writes config.json except detector/importer/stale cleanup.
- [x] 2.9 RED fixtures: vision override vector/HashMap translation and UTF-8 sort;
  explicit `startOnDemand` normalizer/static warning, direct unnormalized encode
  rejection, and absent legacy maxVerifyAttempts -> 2.
- [x] 2.10 RED fixtures: API-key/agent-argument diagnostic redaction, nil/empty
  canonical emission, and machine-local absolute Asset.path round-trip.
- [x] 2.11 RED integer-domain fixtures: reject negative unsigned-target values,
  values above u8/u16/u32 target width, and tokens above i64::MAX before typed
  Config mutation.
- [x] 2.12 RED encode/decode fixtures: reject duplicate Engine.id, Asset.id,
  AutoAgent.id, and AutoAgent.label with static value-free diagnostics.
- [x] 2.13 Outer BDD RED: old camelCase save_config payload containing true
  startOnDemand normalizes false, emits only warning code
  CONFIG_DEPRECATED_START_ON_DEMAND_DROPPED plus its static field, and persists
  EDN without the deprecated key.
  Evidence: `bdd_tauri_camel_case_start_on_demand_normalizes_with_only_static_warning`.
- [ ] 2.14 After rollout evidence, remove start_on_demand from Rust contracts and
  generated frontend bindings; retain JSON backfill recognition only for the
  documented one-shot migration window.
- [x] 2.15 Implement one persistence mutex around inspect/import/write/verify/
  delete/in-memory swap and save_config; acquire bounded exclusive advisory
  `config.lock` inside it. Prove OS stale-lock release and safe timeout error.
- [x] 2.16 Outer BDD RED: paused backfill blocks newer save_config, then newer save
  wins; two processes serialize through config.lock and last successful durable
  locked writer wins.
  Evidence: `bdd_paused_backfill_blocks_newer_save_then_newer_durable_write_wins`
  and `bdd_two_process_writers_serialize_and_last_durable_writer_wins`.
- [x] 2.17 RED: save success follows durable EDN independently of cleanup status;
  cleanup success clears READY_WITH_CLEANUP_PENDING. Completion gate requires no
  config.json only in READY and records/retries the pending exception.
- [x] 2.18 RED diagnostic matrix across stdout/stderr/log/Tauri message/details/
  warnings permits stage, class, location, token class, safe basename only; rejects
  raw source/config/token text, secrets, agent cmd/args, absolute sensitive paths.
  Evidence: `bdd_persistence_diagnostic_matrix_redacts_every_public_sink`.

## 3. Shape summaries and boundaries

- [x] 3.1 RED: exact allowed-field shape fixture; empty and UTF-8 id-sorted parts;
  exact format/kind allowlists; unknown-field/leakage/invariant rejection.
- [x] 3.2 Implement typed FreeCAD boundary normalization; reject unknown fields.
- [x] 3.3 RED: current PartBinding missing optional metrics must be completed from
  authoritative geometry or summary emission rejects.
- [x] 3.4 RED: topology counts reject negative values and integer tokens above
  i64::MAX; typed decode checks the Rust u64 target width.
- [x] 3.5 RED: external JSON adapter inventory tests cover MCP, Tauri/Specta,
  provider REST, Direct-OCCT/Build123d/FreeCAD plans/reports,
  project mirror, package/archive/index, runtime manifests, DB JSON columns.

## 4. Completion proof

- [x] 4.1 Focused parser/config/shape tests, happy plus failure/pending state.
- [x] 4.2 `cd src-tauri && cargo check`.
- [x] 4.3 `openspec validate steel-data-config-format --strict --no-interactive`.
