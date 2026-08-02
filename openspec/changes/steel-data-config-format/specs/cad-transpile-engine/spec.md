## ADDED Requirements

### Requirement: CAD shape summaries normalize to canonical internal EDN

The system SHALL represent internal prompt shape summaries only as
`:ecky/shape-summary` v1 canonical EDN. Source format is `:fcstd/:step/:stp/
:openscad/:raw`; name is sanitized basename UTF-8 <=255 bytes; hash is 64
lowercase SHA-256 hex; units `:mm`; counts are Integers 0..=i64::MAX with an
additional Rust u64 target-width check; bounds three finite min<=max
coords. Parts sort by UTF-8 id and have unique nonempty id <=128 bytes, UTF-8
label <=256, allowlisted kind, finite nonnegative volume/area. It SHALL reject
unknown fields and never copy paths/source bytes. FreeCAD JSON MAY exist only at
subprocess boundary and SHALL typed-normalize before persistence.

All fields are required with no defaults; unknown fields reject at every level.

| Map | Exact fields and types |
| --- | --- |
| root | `:schema` exact `:ecky/shape-summary`; `:version` exact Integer 1; `:source` Source; `:units` exact `:mm`; `:topology` Topology; `:bounds` Bounds; `:parts` Vector<Part>, may be empty |
| source | `:format` one of `:fcstd/:step/:stp/:openscad/:raw`; `:name` sanitized UTF-8 basename <=255 bytes with no separators/control/absolute path; `:hash` SHA-256 exactly 64 lowercase hex |
| topology | required `:solids/:shells/:faces/:edges/:vertices`, each Integer 0..=i64::MAX; typed decode checks u64 width |
| bounds | `:min` and `:max`, each exactly three finite coordinates; component-wise min <= max |
| part | `:id` nonempty <=128 UTF-8 bytes and unique; `:label` authored UTF-8 <=256 bytes; `:kind` one of `:solid/:shell/:compound/:mesh/:unknown`; finite nonnegative `:volume` and `:area` |

Canonical writer sorts parts by unsigned UTF-8 id bytes. Raw paths, source bytes,
and unknown extractor fields are never copied. Current `PartBinding` optional
bounds/volume/area metrics must be completed by the adapter from authoritative
geometry or summary emission rejects.

#### Scenario: FreeCAD boundary JSON becomes safe canonical summary

- GIVEN valid FreeCAD extractor JSON with a source path and ordered parts
- WHEN the extractor result crosses into the application
- THEN its typed canonical EDN summary retains source format/name/hash and parts
- AND it contains neither the absolute path nor source bytes.

#### Scenario: Incomplete PartBinding cannot become a summary part

- GIVEN a current PartBinding missing volume or area
- WHEN the adapter cannot complete metrics from authoritative geometry
- THEN shape-summary emission fails with a location-rich diagnostic
- AND no zero or fabricated metric is written.

#### Scenario: Negative topology count is rejected

- GIVEN a shape-summary with a negative face count
- WHEN schema decoding runs
- THEN decoding fails at `:topology/:faces`
- AND no prompt summary is emitted.
