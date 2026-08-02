## ADDED Requirements

### Requirement: Strict data-only EDN subset integrates with Steel values

The system SHALL use a custom EDN reader, never Steel parser/Engine/eval, with
one top-level form and EOF after separators. It SHALL accept maps/vectors,
keyword values and keyword-only map keys, JSON-escaped UTF-8 strings, exact
true/false/nil, i64 ints, finite f64 decimals/exponents, commas/whitespace and
semicolon comments. Keywords are case-sensitive ASCII lowercase kebab segments,
optionally `:ns/name`; duplicate identity is exact decoded keyword. It SHALL use
`SteelDataValue::{Nil,Bool,Integer(i64),Float(f64),String,Keyword,Vector,Map}`
and convert only to immutable Steel semantic variants. It SHALL reject lists,
symbols, quote forms, tags, sets, dotted forms, eval/macros, NaN/inf/overflow,
leading-zero numbers, and unsupported Steel values. Limits: 1 MiB input, depth
64, 100000 nodes, 256 KiB decoded strings, 10000 collection entries, 128-byte
numeric tokens, checked allocations, and location-rich, secret-safe errors.

#### Scenario: Canonical data round-trips through Steel values

- GIVEN valid commented EDN config data
- WHEN it is parsed, converted to typed Steel values, and written
- THEN comments are absent
- AND keys are ordered by unsigned UTF-8 bytes of full keyword text without `:`
- AND numbers and strings use normalized spelling
- AND output ends with one trailing newline.

#### Scenario: Hostile input is rejected without evaluation

- GIVEN input containing a quoted list, tagged literal, duplicate exact key,
  or an over-limit string
- WHEN it is parsed
- THEN parsing fails before config mutation or evaluation
- AND the error identifies location, token class, and cause without input text.

#### Scenario: Integer token above i64 domain is rejected

- GIVEN an integer token greater than `i64::MAX` or less than `i64::MIN`
- WHEN the custom EDN reader parses it
- THEN parsing fails with a location-rich integer-overflow diagnostic
- AND no wrapped or unsigned SteelData value is produced.
