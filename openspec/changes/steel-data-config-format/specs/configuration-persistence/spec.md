## ADDED Requirements

### Requirement: Config EDN is sole runtime persistence

The system SHALL recognize only `:ecky/config` v1 with known fields, map current
camelCase JSON aliases/defaults/required fields to kebab EDN fields, and preserve
secret policy. `app_config_dir/config.edn` SHALL be sole runtime persistence.
`config.json` SHALL be read only by the one-shot backfill detector/importer when
EDN is absent, then deleted only by stale-file cleanup. No fallback, dual-read,
backup JSON, or normal JSON write is permitted. A malformed EDN fails closed and
does not read or delete JSON. Absent both yields unsaved defaults.

When EDN is absent and JSON exists, startup SHALL typed-parse JSON, apply startup
migrations, atomically write canonical EDN through a same-directory unique temp,
file fsync, rename, and parent fsync, then reopen and typed-parse EDN. It SHALL
delete JSON only after typed equivalence succeeds. Errors before rename clean the
temp and preserve source files/current config. A rename followed by failed parent
fsync is a published-but-not-proven-durable recovery error and SHALL NOT swap
in-memory Config. JSON deletion failure SHALL enter
`READY_WITH_CLEANUP_PENDING`: the app loads authoritative EDN, never parses JSON,
surfaces a redacted warning, and retries deletion on every startup and after each
successful save. Invalid JSON SHALL return a location-rich, secret-safe migration
error, preserve JSON, publish no EDN, and fail config load closed.

The system SHALL use one in-process persistence mutex around the full inspect,
import, write, fsync, rename, reopen, verify, delete, and in-memory-swap state
machine, including `save_config`. Inside that mutex it SHALL acquire an exclusive
advisory OS lock on same-directory `config.lock` before file inspection. Lock
acquisition SHALL use a bounded wait and return a staged, secret-safe contention
error. OS lock release on process exit SHALL provide stale-lock recovery; lock
file age/content SHALL NOT determine ownership. All operations use mutex then
file-lock order. In-memory state swaps only after durable publish and reopen
typed-equivalence proof. Locked writers serialize; last successful durable
writer wins.

`READY` SHALL mean durable verified EDN without stale JSON.
`READY_WITH_CLEANUP_PENDING` SHALL mean durable verified EDN with stale JSON
whose deletion failed. Save success depends on EDN durability/equivalence, not
cleanup success; cleanup status is returned separately and successful deletion
clears it. No operation in either state parses JSON.

Every parser, migration, persistence, cleanup, or locking diagnostic SHALL expose
only stage, error class, location (byte offset, line, column), token class, and an
optional safe path basename. Raw source/config text, token contents, API keys,
auto-agent commands/arguments, and absolute sensitive paths SHALL NOT appear in
stdout, stderr, logs, Tauri message/details, or warnings.

Canonical EDN accepts only keys in these tables. Writer emits every resolved
nondeprecated field, including nil and empty collections. Unknown EDN keys reject
at the root and every nested level. Legacy aliases are JSON-translator-only.

| Config field | Canonical EDN type | Required/default | Legacy JSON canonical / aliases | Secret/privacy |
| --- | --- | --- | --- | --- |
| `:schema` | Keyword `:ecky/config` | required exact | none | discriminator |
| `:version` | Integer `1` | required exact | none | discriminator |
| `:engines` | Vector<Engine> | required; Engine.id unique | `engines` | contains secrets |
| `:selected-engine-id` | String | required | `selectedEngineId` / `selected_engine_id` | local id |
| `:freecad-cmd` | String | `""` | `freecadCmd` / `freecad_cmd` | machine-local |
| `:cad-text-font-path` | String | `""` | `cadTextFontPath` / `cad_text_font_path` | machine-local |
| `:freecad-library-roots` | Vector<String> | `[]` | `freecadLibraryRoots` | machine-local |
| `:assets` | Vector<Asset> | `[]`; Asset.id unique | `assets` | nonportable paths |
| `:microwave` | Nil or Microwave | `nil` | `microwave` | local |
| `:voice` | Voice | Voice defaults | `voice` | local |
| `:mcp` | Mcp | Mcp defaults | `mcp` | commands sensitive |
| `:has-seen-onboarding` | Bool | `false` | `hasSeenOnboarding` | local |
| `:connection-type` | Nil or `:api-key`/`:mcp` | `nil` | `connectionType` strings `api_key`/`mcp` | routing |
| `:default-engine-kind` | enum Keyword | existing-file default `:ecky` | `defaultEngineKind`, enum aliases below | public |
| `:default-source-language` | enum Keyword | existing-file default `:ecky` | `defaultSourceLanguage`, enum aliases below | public |
| `:default-geometry-backend` | enum Keyword | existing-file default `:build123d` | `defaultGeometryBackend`, enum aliases below | public |
| `:max-generation-attempts` | Integer 0..=u32::MAX | `3`; decode checks u32 width | `maxGenerationAttempts` | local limit |
| `:max-verify-attempts` | Integer 0..=u32::MAX | `2`; decode checks u32 width | `maxVerifyAttempts` / `max_verify_attempts`; absence normalizes to 2 before migration | local limit |
| `:projects-root` | Nil or String | `nil` | `projectsRoot` | machine-local |

| Engine field | Canonical EDN type | Required/default | Legacy JSON canonical / aliases | Secret/privacy |
| --- | --- | --- | --- | --- |
| `:id` | String | required | `id` | local id |
| `:name` | String | required | `name` | display |
| `:provider` | String | required | `provider` | routing |
| `:api-key` | String | required | `apiKey` / `api_key` | plaintext local secret unchanged; redact diagnostics |
| `:model` | String | required | `model` | metadata |
| `:light-model` | String | `""` | `lightModel` / `light_model` | metadata |
| `:base-url` | String | required | `baseUrl` / `base_url` | may expose local network |
| `:enabled` | Bool | `true` | `enabled` | local |
| `:vision-overrides` | Vector<VisionOverride> | `[]`, unique model-id | `visionOverrides` JSON string-key map | local |

| VisionOverride field | Canonical EDN type | Required/default | JSON translation |
| --- | --- | --- | --- |
| `:model-id` | String | required and unique | JSON map key |
| `:capability` | `:auto`/`:vision`/`:text-only` | required | `auto`/`vision`/`textOnly` |

Writer sorts vision overrides by unsigned UTF-8 model-id bytes. Translator maps
the vector to/from `HashMap<String, VisionCapability>`.

| Nested field | Canonical EDN type | Required/default | Legacy JSON canonical / aliases | Secret/privacy |
| --- | --- | --- | --- | --- |
| Asset `:id` | String | required | `id` | local id |
| Asset `:name` | String | required | `name` | display |
| Asset `:path` | String | required absolute machine-local path | `path` | explicitly nonportable |
| Asset `:format` | String | required | `format` | metadata |
| Microwave `:hum-id` | Nil or String | `nil` | `humId` / `hum_id` | asset id |
| Microwave `:ding-id` | Nil or String | `nil` | `dingId` / `ding_id` | asset id |
| Microwave `:muted` | Bool | `false` | `muted` | local |
| Voice `:stt-language-code` | String | `"en-US"` | `sttLanguageCode` / `stt_language_code` | locale |
| Mcp `:port` | Nil or Integer 0..=u16::MAX | `nil`; decode checks u16 width | `port` | local network |
| Mcp `:max-sessions` | Nil or Integer 0..=u8::MAX | `nil`; decode checks u8 width | `maxSessions` | limit |
| Mcp `:mode` | `:passive`/`:active` | absent resolves active iff auto-agents nonempty, else passive | `mode` | routing |
| Mcp `:primary-agent-id` | Nil or String | `nil`, then resolve first enabled | `primaryAgentId` | local id |
| Mcp `:prompt-timeout-secs` | Integer 0..=i64::MAX | `1800`; decode checks u64 width | `promptTimeoutSecs` | limit |
| Mcp `:ecky-ast-authoring` | Bool | `false` | `eckyAstAuthoring` | capability |
| Mcp `:auto-agents` | Vector<AutoAgent> | `[]`; AutoAgent.id and label each unique | `autoAgents` | commands sensitive |
| AutoAgent `:id` | String | required | `id` | local id |
| AutoAgent `:label` | String | required | `label` | display |
| AutoAgent `:cmd` | String | required | `cmd` | machine-local, redact secret diagnostics |
| AutoAgent `:model` | Nil or String | `nil` | `model` | metadata |
| AutoAgent `:args` | Vector<String> | required | `args` | machine-local, redact secret diagnostics |
| AutoAgent `:enabled` | Bool | required | `enabled` | local |
| AutoAgent deprecated start flag | forbidden/absent | never emitted; unnormalized typed true rejects | `startOnDemand` normalizes false with static warning | removed behavior |

Encode and decode SHALL reject duplicate Engine.id, Asset.id, AutoAgent.id, or
AutoAgent.label using static safe codes and field paths without echoing values,
labels, API keys, cmd, or args.

One-shot JSON backfill and `save_config` SHALL explicitly normalize any true
`startOnDemand` to false before EDN persistence and record warning
`CONFIG_DEPRECATED_START_ON_DEMAND_DROPPED` for static field
`mcp.autoAgents[].startOnDemand`. Warning SHALL contain no values, ids, labels,
cmd, or args. Direct canonical encoding of unnormalized typed Config whose
compatibility field is true SHALL reject
`CONFIG_NONCANONICAL_DEPRECATED_FIELD`. EDN SHALL reject that key as unknown and
canonical EDN SHALL never emit it.

Enum EDN values are: EngineKind `:freecad/:ecky/:build123d`;
SourceLanguage `:legacy-python/:ecky/:build123d`; GeometryBackend
`:freecad/:build123d/:mesh`. Translator-only JSON aliases are respectively
`eckyIrV0`/`ecky_ir_v0`; `legacy_python`, `eckyIrV0`/`ecky_ir_v0`; and
`native`/`eckyRust`/`ecky_rust`.

When both files are absent, unsaved in-memory defaults are exactly: one disabled
Gemini engine (`default-gemini`, `Google Gemini`, provider `gemini`, empty API
key, `gemini-2.5-flash`, light model `gemini-2.5-flash-lite`, empty base URL,
empty vision overrides); selected id `default-gemini`; empty FreeCAD/font
strings, roots and assets; nil microwave/connection/projects root; voice
`en-US`; default Mcp (`nil`, `nil`, passive, nil, 1800, false, empty agents);
onboarding false; engine/source/backend `freecad`/`legacy-python`/`freecad`; and
generation/verify attempts 3/2.

#### Scenario: Valid EDN cleans stale JSON without reading it

- GIVEN valid config.edn and config.json with divergent non-secret values
- WHEN configuration loads
- THEN typed config comes from config.edn
- AND config.json is deleted as stale cleanup without being parsed.

#### Scenario: Invalid EDN has no JSON rescue

- GIVEN invalid config.edn and valid config.json
- WHEN configuration loads
- THEN loading fails closed with the EDN error
- AND config.json is neither read nor deleted.

#### Scenario: One-shot JSON backfill proves equivalence before deletion

- GIVEN config.edn is absent and config.json is valid
- WHEN configuration starts
- THEN startup typed-parses JSON and applies startup migrations
- AND atomically writes, fsyncs, renames, reopens, and typed-parses config.edn
- AND deletes config.json only after typed Config equivalence succeeds.

#### Scenario: Backfill deletion failure retries cleanup

- GIVEN a successful JSON-to-EDN backfill and a config.json deletion failure
- WHEN configuration starts
- THEN it loads config.edn in READY_WITH_CLEANUP_PENDING
- AND surfaces cleanup status and a redacted warning separately from EDN success
- AND next startup and each successful save retry deletion under both locks
- AND config.json is never parsed.

#### Scenario: Invalid JSON preserves evidence and fails closed

- GIVEN config.edn is absent and config.json is invalid
- WHEN configuration starts
- THEN loading fails closed with a staged location/token-class migration error
- AND config.json remains unchanged
- AND config.edn is not created.

#### Scenario: Missing both files uses unsaved defaults

- GIVEN config.edn and config.json are absent
- WHEN configuration starts
- THEN it uses in-memory defaults
- AND it writes neither file until an explicit save.

#### Scenario: save_config keeps JSON at invoke boundary

- GIVEN a camelCase JSON `save_config` Tauri payload
- WHEN Rust accepts it as typed Config
- THEN persistence writes a canonical complete config.edn
- AND reopening config.edn produces the same resolved Config
- AND no config.json artifact is written.

#### Scenario: Paused backfill serializes a newer save

- GIVEN startup backfill is paused while holding the persistence mutex and
  config.lock
- AND a newer in-process save_config request arrives
- WHEN backfill durably publishes, verifies, cleans up, and releases both locks
- THEN save_config waits and performs no file inspection or mutation meanwhile
- AND the newer save runs next and becomes authoritative after durable proof.

#### Scenario: Two processes serialize config writers

- GIVEN two app processes target the same config directory
- WHEN both attempt startup migration or save_config concurrently
- THEN only one holds the exclusive config.lock and runs the state machine
- AND the other acquires within the bounded wait or receives a staged contention
  error without mutation
- AND if both succeed, the last durable locked writer wins.

#### Scenario: Save succeeds while cleanup remains pending

- GIVEN READY_WITH_CLEANUP_PENDING and a valid save_config request
- WHEN EDN durable publish and equivalence proof succeed but JSON deletion fails
- THEN save_config reports EDN success with separate cleanup-pending status
- AND the new in-memory Config is installed
- AND authoritative EDN is loaded without parsing JSON.

#### Scenario: Deprecated auto-agent flag is dropped visibly

- GIVEN legacy JSON containing `autoAgents[*].startOnDemand`
- WHEN it migrates
- THEN explicit normalization sets every compatibility value false
- AND config.edn contains no such field
- AND warning `CONFIG_DEPRECATED_START_ON_DEMAND_DROPPED` names only static field
  `mcp.autoAgents[].startOnDemand`.

#### Scenario: Old save_config payload survives rollout

- GIVEN a camelCase save_config payload containing true startOnDemand
- WHEN the boundary normalizer runs
- THEN it records only the static migration warning code and field
- AND persists normalized EDN without a start-on-demand key.

#### Scenario: Unnormalized deprecated state cannot encode

- GIVEN typed Config with AutoAgent.start_on_demand true
- WHEN canonical encoding runs without boundary normalization
- THEN it rejects `CONFIG_NONCANONICAL_DEPRECATED_FIELD`
- AND the diagnostic contains no agent values.

#### Scenario: Duplicate collection identities reject safely

- GIVEN typed Config or config.edn with duplicate Engine ids, Asset ids,
  AutoAgent ids, or AutoAgent labels
- WHEN canonical encode or decode runs
- THEN it rejects at the collection field with a static safe diagnostic
- AND the diagnostic contains none of the duplicate values or secret fields.

#### Scenario: Unknown nested EDN field fails closed

- GIVEN config.edn with a valid schema/version and an unknown Engine key
- WHEN configuration loads
- THEN loading fails with the key location
- AND legacy JSON is not read and current config is not mutated.

#### Scenario: Config integer outside target width is rejected

- GIVEN config.edn with a negative port or max-sessions greater than u8::MAX
- WHEN typed Config decoding runs
- THEN decoding fails at that field with its accepted range
- AND current config and both persistence files remain unchanged.

#### Scenario: Failed backfill publish preserves source state

- GIVEN config.edn is absent, config.json is valid, and atomic EDN write fails
- WHEN startup backfill runs
- THEN config.edn is not partially published
- AND config.json and current config remain unchanged
- AND the user receives a staged, location-rich, secret-safe failure.

#### Scenario: No JSON persistence artifact survives clean completion

- GIVEN migration cleanup or save_config reaches READY
- WHEN repository artifact scanning runs
- THEN config.json is absent
- AND no runtime path reads or writes config.json except the backfill detector,
  typed importer, and stale-file cleanup
- AND READY_WITH_CLEANUP_PENDING is accepted only when deletion failed, with
  explicit status and future locked retries.

#### Scenario: Diagnostics redact every output surface

- GIVEN invalid config contains an API key, auto-agent command/arguments, raw
  source text, and an absolute sensitive path
- WHEN parsing, migration, persistence, cleanup, or lock acquisition fails
- THEN stdout, stderr, logs, Tauri message/details, and warnings contain only
  allowed stage, class, location, token class, and optional safe basename
- AND none contains raw input/token text or sensitive values.
