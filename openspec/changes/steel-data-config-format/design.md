# Design: Steel Data Config Format

## Goal

Strict internal data files, canonical output, safe config migration, typed
Steel-value integration.

## Artifact model

`config.edn` and internal prompt shape summaries use a custom EDN reader, never
the Steel parser/Engine/eval. Parser output is `SteelDataValue::{Nil,
Bool,Integer(i64),Float(f64),String,Keyword,Vector,Map}`. Conversion permits
only Steel Void/Bool/Int/Num/String/Keyword/immutable Vector/immutable HashMap
semantic variants; reverse conversion rejects callable, mutable, custom, opaque
values. Root maps require `:schema` keyword and integer `:version`.

## Variables

- Grammar: one top-level form then EOF after separators. Whitespace, commas, and
  semicolon-to-newline comments separate forms.
- Storage: sole runtime `app_config_dir/config.edn`; one-shot `config.json`
  backfill/import and cleanup only.
- Ownership: Rust parser/writer/config persistence; frontend stays camelCase
  through Tauri contract adapters.
- Shape boundary: extractor JSON may cross subprocess boundary only, then
  typed-normalizes to canonical EDN summary.
- Concurrency: one process mutex plus bounded exclusive advisory `config.lock`
  serializes every config persistence state transition.
- Safety: 1 MiB input; depth 64; 100000 nodes; decoded string 256 KiB; map/vector
  10000 entries; numeric token 128 bytes; checked allocations.
- Testing: parser/writer, hostile data, typed equivalence, persistence, fixture,
  and adapter BDD/unit evidence.

## Decision

### Grammar and values

Accept maps `{}`, vectors `[]`, keyword values, strings, finite ints/floats,
`true`, `false`, `nil`; map keys are keywords only. Keyword syntax is ASCII
lowercase kebab segments, optional single namespace `:ns/name`, case-sensitive.
Config fields are unqualified. Duplicate identity is exact decoded keyword,
including namespace: no Unicode/case normalization. Maps require even forms.
Strings are valid UTF-8 with JSON-style escapes. Int grammar is
`-?(0|[1-9][0-9]*)` into i64. Floats are finite decimal/exponent f64, reject
NaN/inf/overflow/leading zeros. Explicitly reject lists `()`, symbols/bare
tokens, quote `'`, syntax quote `` ` ``, unquote `~`/`~@`, tags `#foo`, sets
`#{}`, dotted forms, evaluation, and macros. Limits attach byte offset, line,
column, and context before checked allocation growth.

### Canonicalization

Writer orders map keys by unsigned UTF-8 bytes of full keyword text without `:`.
It emits valid UTF-8 non-controls; escapes quote/backslash/standard controls and
remaining controls as lowercase `\u00xx`; drops comments; ends with one newline.
Numbers use shortest round-trip spelling, lowercase `e`, no plus or leading
exponent zeros; integer-valued floats retain `.0`; `-0.0` becomes `0.0`. It never
serializes unsupported Steel values.

### Config precedence and migration

Only `:ecky/config` v1 and `:ecky/shape-summary` v1 exist initially; unknown
schema/version/field fails. Inventory current camelCase JSON required/default/
alias mapping into kebab keyword fields and test equivalence; secrets retain
policy. EDN present is authoritative. Valid EDN plus stale JSON deletes stale
JSON during startup cleanup; JSON is not read. Malformed/unsupported EDN fails
closed, with no JSON rescue or mutation. When EDN is absent, startup may run the
one-shot backfill importer: typed-parse legacy JSON, apply startup migrations,
write canonical EDN with the same-directory atomic protocol, reopen and
typed-parse it, and prove typed equivalence before deleting JSON. A JSON deletion
failure enters `READY_WITH_CLEANUP_PENDING`; EDN remains authoritative, the app
loads it, emits a redacted cleanup warning, and retries deletion under lock on
every startup and successful save. Invalid JSON with absent EDN returns a
location-rich, secret-safe migration error, preserves JSON, publishes no EDN,
and fails config load closed.
Neither file means in-memory defaults and no write until explicit save. Existing
EDN is never overwritten from JSON.

EDN accepts canonical keys only. Legacy aliases belong only to the JSON
translator. The canonical writer emits every resolved, nondeprecated field,
including `nil`, empty vectors, and empty nested maps/vectors.

#### Config v1 root

| Rust field | Canonical EDN key and SteelData type | EDN read rule | Canonical writer | Legacy JSON canonical / aliases | Privacy and portability |
| --- | --- | --- | --- | --- | --- |
| schema | `:schema` Keyword `:ecky/config` | required, exact | emit | none | public discriminator |
| version | `:version` Integer `1` | required, exact | emit | none | public discriminator |
| engines | `:engines` Vector of Engine maps | required; Engine.id unique | emit, including `[]`; duplicate ids reject | `engines`, no alias | contains API keys |
| selected_engine_id | `:selected-engine-id` String | required | emit | `selectedEngineId` / `selected_engine_id` | local identifier |
| freecad_cmd | `:freecad-cmd` String | default `""` | emit | `freecadCmd` / `freecad_cmd` | machine-local command |
| cad_text_font_path | `:cad-text-font-path` String | default `""` | emit | `cadTextFontPath` / `cad_text_font_path` | machine-local path |
| freecad_library_roots | `:freecad-library-roots` Vector<String> | default `[]` | emit | `freecadLibraryRoots`, no alias | machine-local paths |
| assets | `:assets` Vector<Asset> | default `[]`; Asset.id unique | emit; duplicate ids reject | `assets`, no alias | paths are nonportable |
| microwave | `:microwave` Nil or Microwave map | default `nil` | emit `nil` or full map | `microwave`, no alias | local preference |
| voice | `:voice` Voice map | default Voice defaults | emit full map | `voice`, no alias | local preference |
| mcp | `:mcp` Mcp map | default Mcp defaults | emit full map | `mcp`, no alias | commands may be sensitive |
| has_seen_onboarding | `:has-seen-onboarding` Bool | default `false` | emit | `hasSeenOnboarding`, no alias | local preference |
| connection_type | `:connection-type` Nil or Keyword `:api-key`/`:mcp` | default `nil`; other values reject | emit | `connectionType`, no alias; JSON strings `api_key`/`mcp` | local routing |
| default_engine_kind | `:default-engine-kind` Keyword | default `:ecky` for an existing file | emit | `defaultEngineKind`; aliases below | public enum |
| default_source_language | `:default-source-language` Keyword | default `:ecky` for an existing file | emit | `defaultSourceLanguage`; aliases below | public enum |
| default_geometry_backend | `:default-geometry-backend` Keyword | default `:build123d` for an existing file | emit | `defaultGeometryBackend`; aliases below | public enum |
| max_generation_attempts | `:max-generation-attempts` Integer, 0..=u32::MAX | default `3`; typed decode checks u32 width | emit | `maxGenerationAttempts`, no alias | local limit |
| max_verify_attempts | `:max-verify-attempts` Integer, 0..=u32::MAX | default `2`; typed decode checks u32 width; absent legacy JSON is normalized to `2` before EDN migration | emit | `maxVerifyAttempts` / `max_verify_attempts` | local limit |
| projects_root | `:projects-root` Nil or String | default `nil` | emit | `projectsRoot`, no alias | machine-local path |

#### Engine and vision override

| Rust field | Canonical EDN key and SteelData type | EDN read rule | Canonical writer | Legacy JSON canonical / aliases | Privacy and portability |
| --- | --- | --- | --- | --- | --- |
| id | `:id` String | required | emit | `id` | local identifier |
| name | `:name` String | required | emit | `name` | display text |
| provider | `:provider` String | required | emit | `provider` | routing |
| api_key | `:api-key` String | required | emit plaintext | `apiKey` / `api_key` | plaintext local secret under current policy; diagnostics redact value |
| model | `:model` String | required | emit | `model` | provider metadata |
| light_model | `:light-model` String | default `""` | emit | `lightModel` / `light_model` | provider metadata |
| base_url | `:base-url` String | required | emit | `baseUrl` / `base_url` | endpoint, may reveal local network |
| enabled | `:enabled` Bool | default `true` | emit | `enabled` | local preference |
| vision_overrides | `:vision-overrides` Vector<VisionOverride> | default `[]`; unique `model-id` | emit sorted by unsigned UTF-8 `model-id` | `visionOverrides` object map, no alias | model metadata |
| VisionOverride.model_id | `:model-id` String | required, unique within engine | emit | JSON object key | model identifier |
| VisionOverride.capability | `:capability` Keyword `:auto`/`:vision`/`:text-only` | required | emit | JSON values `auto`/`vision`/`textOnly` | local preference |

The EDN vector is necessary because EDN maps permit keyword keys only. The
translator maps it to/from `HashMap<String, VisionCapability>`.

#### Asset, Microwave, and Voice

| Struct.field | Canonical EDN key and SteelData type | EDN read rule | Canonical writer | Legacy JSON canonical / aliases | Privacy and portability |
| --- | --- | --- | --- | --- | --- |
| Asset.id | `:id` String | required | emit | `id` | local identifier |
| Asset.name | `:name` String | required | emit | `name` | display text |
| Asset.path | `:path` String | required; machine-local absolute path | emit unchanged | `path` | explicitly nonportable local path |
| Asset.format | `:format` String | required | emit | `format` | format label |
| Microwave.hum_id | `:hum-id` Nil or String | default `nil` | emit | `humId` / `hum_id` | asset identifier |
| Microwave.ding_id | `:ding-id` Nil or String | default `nil` | emit | `dingId` / `ding_id` | asset identifier |
| Microwave.muted | `:muted` Bool | default `false` | emit | `muted` | local preference |
| Voice.stt_language_code | `:stt-language-code` String | default `"en-US"` | emit | `sttLanguageCode` / `stt_language_code` | locale preference |

#### MCP and auto agents

| Struct.field | Canonical EDN key and SteelData type | EDN read rule | Canonical writer | Legacy JSON canonical / aliases | Privacy and portability |
| --- | --- | --- | --- | --- | --- |
| Mcp.port | `:port` Nil or Integer, 0..=u16::MAX | default `nil`; typed decode checks u16 width | emit | `port` | local network setting |
| Mcp.max_sessions | `:max-sessions` Nil or Integer, 0..=u8::MAX | default `nil`; typed decode checks u8 width | emit | `maxSessions` | local limit |
| Mcp.mode | `:mode` Keyword `:passive`/`:active` | if absent, resolve `:active` when auto-agents is nonempty, else `:passive` | emit resolved value | `mode` strings `passive`/`active` | local routing |
| Mcp.primary_agent_id | `:primary-agent-id` Nil or String | default `nil`; startup resolves first enabled agent when needed | emit resolved value | `primaryAgentId` | local identifier |
| Mcp.prompt_timeout_secs | `:prompt-timeout-secs` Integer, 0..=i64::MAX | default `1800`; typed decode also checks u64 width | emit | `promptTimeoutSecs` | local limit |
| Mcp.ecky_ast_authoring | `:ecky-ast-authoring` Bool | default `false` | emit | `eckyAstAuthoring` | capability flag |
| Mcp.auto_agents | `:auto-agents` Vector<AutoAgent> | default `[]`; AutoAgent.id and AutoAgent.label each unique | emit; duplicate ids or labels reject | `autoAgents` | commands may expose machine details |
| AutoAgent.id | `:id` String | required | emit | `id` | local identifier |
| AutoAgent.label | `:label` String | required | emit | `label` | display text |
| AutoAgent.cmd | `:cmd` String | required | emit | `cmd` | machine-local executable; redact where diagnostic could expose secret arguments |
| AutoAgent.model | `:model` Nil or String | default `nil` | emit | `model` | provider metadata |
| AutoAgent.args | `:args` Vector<String> | required | emit, including `[]` | `args` | machine-local arguments; diagnostics redact secret values |
| AutoAgent.enabled | `:enabled` Bool | required | emit | `enabled` | local preference |
| AutoAgent.start_on_demand | no EDN field | deprecated; unknown in EDN | never emit; typed Config with `true` rejects unless explicitly normalized first | `startOnDemand`; normalize false with static warning | removed behavior |

Encode and decode both enforce Engine.id, Asset.id, AutoAgent.id, and
AutoAgent.label uniqueness within their collections. Duplicate rejection uses a
static safe code and field path; diagnostics never echo ids, labels, API keys,
commands, or arguments.

`startOnDemand` rollout uses an explicit compatibility normalizer, not an EDN
legacy field. One-shot JSON backfill and camelCase `save_config` call it before
persistence. Any true value becomes false and records warning code
`CONFIG_DEPRECATED_START_ON_DEMAND_DROPPED` for static field
`mcp.autoAgents[].startOnDemand`; warning includes no values, ids, labels, cmd,
or args. Direct canonical encoding of an unnormalized typed Config containing
true rejects `CONFIG_NONCANONICAL_DEPRECATED_FIELD`. Canonical EDN never includes
the field. A later cleanup task removes the Rust/frontend compatibility field
after rollout evidence proves old payloads drained.

#### Enum translation

| Type | Canonical EDN keywords | Legacy JSON canonical values / translator-only aliases |
| --- | --- | --- |
| EngineKind | `:freecad`, `:ecky`, `:build123d` | `freecad`; `ecky` / `eckyIrV0`, `ecky_ir_v0`; `build123d` |
| SourceLanguage | `:legacy-python`, `:ecky`, `:build123d` | `legacyPython` / `legacy_python`; `ecky` / `eckyIrV0`, `ecky_ir_v0`; `build123d` |
| GeometryBackend | `:freecad`, `:build123d`, `:mesh` | `freecad`; `build123d`; `mesh` / `native`, `eckyRust`, `ecky_rust` |

#### Missing-both startup defaults

When neither config file exists, startup uses this resolved in-memory Config and
does not persist until explicit save:

| Field | Exact value |
| --- | --- |
| engines | one engine: id `default-gemini`, name `Google Gemini`, provider `gemini`, api-key `""`, model `gemini-2.5-flash`, light-model `gemini-2.5-flash-lite`, base-url `""`, enabled `false`, vision-overrides `[]` |
| selected-engine-id | `"default-gemini"` |
| freecad-cmd / cad-text-font-path | `""` / `""` |
| freecad-library-roots / assets | `[]` / `[]` before runtime asset reconciliation |
| microwave / connection-type / projects-root | `nil` / `nil` / `nil` |
| voice | `{:stt-language-code "en-US"}` |
| mcp | port `nil`, max-sessions `nil`, mode `:passive`, primary-agent-id `nil`, prompt-timeout-secs `1800`, ecky-ast-authoring `false`, auto-agents `[]` |
| has-seen-onboarding | `false` |
| default-engine-kind / default-source-language / default-geometry-backend | `:freecad` / `:legacy-python` / `:freecad` |
| max-generation-attempts / max-verify-attempts | `3` / `2` |

### Atomic persistence

Atomic EDN persistence uses unique same-directory temp, permissions policy
preservation, write+file fsync, rename commit point, parent fsync. Pre-rename
errors clean temp and preserve source files. Rename without successful parent
fsync is a published-but-not-proven-durable recovery error and never swaps the
in-memory Config. The backfill importer performs reopen/typed-equivalence
verification after durable publish and before deleting JSON. No backup JSON is
created.

`save_config` remains a Tauri camelCase JSON payload boundary. Rust receives
typed `Config` and writes canonical `config.edn` only.

### Serialization and cleanup state

One in-process persistence mutex covers the complete inspect/import/write/
fsync/rename/reopen/verify/delete/in-memory-swap sequence and every
`save_config`. While holding it, the process acquires an exclusive advisory OS
lock on same-directory `config.lock` before inspecting either config file. Lock
acquisition has a bounded wait and returns a staged, secret-safe contention error
on timeout. OS lock ownership, not file age/content, determines liveness, so
process exit releases stale locks automatically. The inert lock file may remain.

All startup and save transitions use the same lock order: process mutex, then
interprocess lock. `save_config` waits behind migration. Successful locked
writers serialize; the last one to durably publish becomes authoritative. The
in-memory Config swaps only after EDN file fsync, rename, parent-directory fsync,
reopen, typed parse, and typed-equivalence verification all succeed. A failed
writer never installs its candidate in memory.

State `READY` means durable verified EDN and no stale JSON. State
`READY_WITH_CLEANUP_PENDING` means durable verified EDN plus JSON whose deletion
failed. Both states load EDN and never parse JSON. Cleanup failure is separate
from EDN save success: return save success plus cleanup-pending status, surface a
redacted warning, and retry deletion under both locks on every startup and after
every successful save. Successful deletion clears the status to `READY`.

### Diagnostic safety

Parser, migration, persistence, cleanup, and lock diagnostics expose only stage,
error class, location (byte offset, line, column), token class, and a safe path
basename when useful. They never include raw source/config text, token contents,
API keys, auto-agent commands/arguments, or absolute sensitive paths. This rule
applies equally to stdout, stderr, logs, Tauri message/details, and warnings.

### Shape-summary boundary

Shape source.format only `:fcstd/:step/:stp/:openscad/:raw`; name is sanitized
UTF-8 basename <=255 bytes without separators/control/absolute path; hash is 64
lowercase hex SHA-256; units only `:mm`. Counts are nonnegative EDN Integers in
0..=i64::MAX; typed decode also checks the Rust u64 target. Bounds are
three finite coords and min<=max. Volume/area finite >=0. Parts sorted by id UTF-8
bytes: unique nonempty id <=128 bytes; UTF-8 label <=256; kind allowlisted.
Never copy raw absolute paths, source bytes, unknown extractor fields. `label` is
authored content; `source.name` is path-sanitized. FreeCAD JSON normalizes at
subprocess boundary. Only config persistence and internal prompt shape summary
migrate here.

All maps below reject unknown fields. No field is optional or defaulted.

| Shape root field | SteelData type and invariant |
| --- | --- |
| `:schema` | required Keyword, exact `:ecky/shape-summary` |
| `:version` | required Integer, exact `1` |
| `:source` | required Source map |
| `:units` | required Keyword, exact `:mm` |
| `:topology` | required Topology map |
| `:bounds` | required Bounds map |
| `:parts` | required Vector<Part>; may be empty; canonical sort by id UTF-8 bytes |

| Source field | SteelData type and invariant |
| --- | --- |
| `:format` | required Keyword, one of `:fcstd`, `:step`, `:stp`, `:openscad`, `:raw` |
| `:name` | required String, sanitized basename, valid UTF-8 <=255 bytes, no separators/control/absolute path |
| `:hash` | required String, SHA-256 exactly 64 lowercase hex characters |

| Topology field | SteelData type and invariant |
| --- | --- |
| `:solids` | required Integer 0..=i64::MAX; typed decode checks u64 width |
| `:shells` | required Integer 0..=i64::MAX; typed decode checks u64 width |
| `:faces` | required Integer 0..=i64::MAX; typed decode checks u64 width |
| `:edges` | required Integer 0..=i64::MAX; typed decode checks u64 width |
| `:vertices` | required Integer 0..=i64::MAX; typed decode checks u64 width |

| Bounds field | SteelData type and invariant |
| --- | --- |
| `:min` | required Vector of exactly three finite numeric coordinates |
| `:max` | required Vector of exactly three finite numeric coordinates, component-wise >= min |

| Part field | SteelData type and invariant |
| --- | --- |
| `:id` | required nonempty String <=128 UTF-8 bytes, unique; canonical sort key |
| `:label` | required authored String, valid UTF-8 <=256 bytes |
| `:kind` | required Keyword, one of `:solid`, `:shell`, `:compound`, `:mesh`, `:unknown` |
| `:volume` | required finite numeric value >=0 |
| `:area` | required finite numeric value >=0 |

Current `PartBinding.bounds`, `volume`, and `area` are optional runtime metrics.
The adapter must complete required summary metrics from authoritative geometry or
reject emission; it must not invent zero/default metrics.

## Examples

```clojure
{:assets []
 :cad-text-font-path ""
 :connection-type nil
 :default-engine-kind :freecad
 :default-geometry-backend :freecad
 :default-source-language :legacy-python
 :engines [{:api-key ""
            :base-url ""
            :enabled false
            :id "default-gemini"
            :light-model "gemini-2.5-flash-lite"
            :model "gemini-2.5-flash"
            :name "Google Gemini"
            :provider "gemini"
            :vision-overrides []}]
 :freecad-cmd ""
 :freecad-library-roots []
 :has-seen-onboarding false
 :max-generation-attempts 3
 :max-verify-attempts 2
 :mcp {:auto-agents []
       :ecky-ast-authoring false
       :max-sessions nil
       :mode :passive
       :port nil
       :primary-agent-id nil
       :prompt-timeout-secs 1800}
 :microwave nil
 :projects-root nil
 :schema :ecky/config
 :selected-engine-id "default-gemini"
 :version 1
 :voice {:stt-language-code "en-US"}}
```

```clojure
{:bounds {:max [80 40 12] :min [0 0 0]}
 :parts [{:area 987.25 :id "body" :kind :solid :label "Bracket" :volume 1234.5}]
 :schema :ecky/shape-summary
 :source {:format :fcstd
          :hash "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
          :name "bracket.FCStd"}
 :topology {:edges 84 :faces 42 :shells 1 :solids 1 :vertices 56}
 :units :mm
 :version 1}
```

## Rejected paths

- Reusing Steel parser/Engine/eval: expands accepted syntax into executable language.
- Generic EDN permissiveness: admits incompatible data and weakens limits.
- JSON-only canonical storage: loses intended Steel-value data model.
- Replacing all JSON: breaks protocol/public adapter contracts.

## Proof plan

BDD RED first: canonical parser/writer, hostile input rejection, typed Config
equivalence, precedence, atomic rollback, shape fixture, and adapters. Prove
one-shot backfill from a real legacy fixture, equivalence before deletion,
cleanup-pending retry, paused-backfill/save ordering, two-process exclusion,
durable-before-swap behavior, invalid-file fail-closed behavior, and diagnostic
redaction across every output surface.
