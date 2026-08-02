//! Strict internal shape-summary boundary. FreeCAD JSON is normalized here,
//! before prompt data becomes canonical Steel data.

use crate::contracts::PartBinding;
use crate::contracts::{AppError, AppResult};
use crate::steel_data::{validate_steel_data, SteelDataValue};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

const SCHEMA: &str = ":ecky/shape-summary";

#[derive(Debug, Clone, PartialEq)]
pub struct ShapeSummary {
    pub source: ShapeSource,
    pub topology: ShapeTopology,
    pub bounds: ShapeBounds,
    pub parts: Vec<ShapePart>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShapeSource {
    pub format: String,
    pub name: String,
    pub hash: String,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeTopology {
    pub solids: u64,
    pub shells: u64,
    pub faces: u64,
    pub edges: u64,
    pub vertices: u64,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeBounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}
#[derive(Debug, Clone, PartialEq)]
pub struct ShapePart {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub volume: f64,
    pub area: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FreecadShapeSummaryJson {
    pub source: FreecadSourceJson,
    pub topology: FreecadTopologyJson,
    pub bounds: FreecadBoundsJson,
    pub parts: Vec<FreecadPartJson>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FreecadSourceJson {
    pub format: String,
    pub name: String,
    pub hash: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FreecadTopologyJson {
    pub solids: u64,
    pub shells: u64,
    pub faces: u64,
    pub edges: u64,
    pub vertices: u64,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FreecadBoundsJson {
    pub min: Vec<f64>,
    pub max: Vec<f64>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FreecadPartJson {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub volume: f64,
    pub area: f64,
}

fn err(field: &str) -> AppError {
    AppError::parse(format!("invalid shape summary field {field}"))
}
fn keyword(value: &str) -> SteelDataValue {
    SteelDataValue::Keyword(format!(":{value}"))
}
fn map(entries: Vec<(&str, SteelDataValue)>) -> SteelDataValue {
    SteelDataValue::Map(
        entries
            .into_iter()
            .map(|(key, value)| (format!(":{key}"), value))
            .collect(),
    )
}
fn finite_nonnegative(value: f64, field: &str) -> AppResult<f64> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(err(field))
    }
}
fn bounded_text(value: &str, maximum: usize, field: &str) -> AppResult<()> {
    if !value.is_empty() && value.len() <= maximum && !value.chars().any(|c| c.is_control()) {
        Ok(())
    } else {
        Err(err(field))
    }
}

pub fn normalize_freecad_shape_summary(input: FreecadShapeSummaryJson) -> AppResult<ShapeSummary> {
    let format = input.source.format.to_ascii_lowercase();
    if !["fcstd", "step", "stp", "openscad", "raw"].contains(&format.as_str()) {
        return Err(err("source.format"));
    }
    if input.source.hash.len() != 64
        || !input
            .source
            .hash
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
    {
        return Err(err("source.hash"));
    }
    let name = Path::new(&input.source.name)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| err("source.name"))?
        .to_string();
    bounded_text(&name, 255, "source.name")?;
    if name.contains(['/', '\\']) {
        return Err(err("source.name"));
    }
    let topology = ShapeTopology {
        solids: input.topology.solids,
        shells: input.topology.shells,
        faces: input.topology.faces,
        edges: input.topology.edges,
        vertices: input.topology.vertices,
    };
    for value in [
        topology.solids,
        topology.shells,
        topology.faces,
        topology.edges,
        topology.vertices,
    ] {
        if value > i64::MAX as u64 {
            return Err(err("topology"));
        }
    }
    let min: [f64; 3] = input.bounds.min.try_into().map_err(|_| err("bounds.min"))?;
    let max: [f64; 3] = input.bounds.max.try_into().map_err(|_| err("bounds.max"))?;
    if min.into_iter().chain(max).any(|value| !value.is_finite())
        || min.into_iter().zip(max).any(|(min, max)| min > max)
    {
        return Err(err("bounds"));
    }
    let mut ids = HashSet::new();
    let mut parts = Vec::with_capacity(input.parts.len());
    for part in input.parts {
        bounded_text(&part.id, 128, "parts.id")?;
        bounded_text(&part.label, 256, "parts.label")?;
        if !ids.insert(part.id.clone())
            || !["solid", "shell", "compound", "mesh", "unknown"].contains(&part.kind.as_str())
        {
            return Err(err("parts"));
        }
        parts.push(ShapePart {
            id: part.id,
            label: part.label,
            kind: part.kind,
            volume: finite_nonnegative(part.volume, "parts.volume")?,
            area: finite_nonnegative(part.area, "parts.area")?,
        });
    }
    parts.sort_by(|left, right| left.id.as_bytes().cmp(right.id.as_bytes()));
    Ok(ShapeSummary {
        source: ShapeSource {
            format,
            name,
            hash: input.source.hash,
        },
        topology,
        bounds: ShapeBounds { min, max },
        parts,
    })
}

/// PartBinding metrics are optional at the runtime boundary. Summary emission
/// accepts them only once authoritative geometry has completed both values.
pub fn shape_parts_from_part_bindings(bindings: &[PartBinding]) -> AppResult<Vec<ShapePart>> {
    let parts = bindings
        .iter()
        .map(|binding| {
            Ok(FreecadPartJson {
                id: binding.part_id.clone(),
                label: binding.label.clone(),
                kind: match binding.kind.as_str() {
                    "Part::Feature" | "solid" => "solid",
                    "shell" => "shell",
                    "compound" => "compound",
                    "mesh" => "mesh",
                    "unknown" => "unknown",
                    _ => return Err(err("parts.kind")),
                }
                .into(),
                volume: binding.volume.ok_or_else(|| err("parts.volume"))?,
                area: binding.area.ok_or_else(|| err("parts.area"))?,
            })
        })
        .collect::<AppResult<Vec<_>>>()?;
    normalize_parts(parts)
}

fn normalize_parts(parts: Vec<FreecadPartJson>) -> AppResult<Vec<ShapePart>> {
    let mut ids = HashSet::new();
    let mut normalized = Vec::with_capacity(parts.len());
    for part in parts {
        bounded_text(&part.id, 128, "parts.id")?;
        bounded_text(&part.label, 256, "parts.label")?;
        if !ids.insert(part.id.clone())
            || !["solid", "shell", "compound", "mesh", "unknown"].contains(&part.kind.as_str())
        {
            return Err(err("parts"));
        }
        normalized.push(ShapePart {
            id: part.id,
            label: part.label,
            kind: part.kind,
            volume: finite_nonnegative(part.volume, "parts.volume")?,
            area: finite_nonnegative(part.area, "parts.area")?,
        });
    }
    normalized.sort_by(|left, right| left.id.as_bytes().cmp(right.id.as_bytes()));
    Ok(normalized)
}

pub fn encode_shape_summary(summary: &ShapeSummary) -> AppResult<SteelDataValue> {
    let normalized = normalize_freecad_shape_summary(FreecadShapeSummaryJson {
        source: FreecadSourceJson {
            format: summary.source.format.clone(),
            name: summary.source.name.clone(),
            hash: summary.source.hash.clone(),
        },
        topology: FreecadTopologyJson {
            solids: summary.topology.solids,
            shells: summary.topology.shells,
            faces: summary.topology.faces,
            edges: summary.topology.edges,
            vertices: summary.topology.vertices,
        },
        bounds: FreecadBoundsJson {
            min: summary.bounds.min.to_vec(),
            max: summary.bounds.max.to_vec(),
        },
        parts: summary
            .parts
            .iter()
            .map(|part| FreecadPartJson {
                id: part.id.clone(),
                label: part.label.clone(),
                kind: part.kind.clone(),
                volume: part.volume,
                area: part.area,
            })
            .collect(),
    })?;
    Ok(map(vec![
        ("schema", SteelDataValue::Keyword(SCHEMA.into())),
        ("version", SteelDataValue::Integer(1)),
        (
            "source",
            map(vec![
                ("format", keyword(&normalized.source.format)),
                ("name", SteelDataValue::String(normalized.source.name)),
                ("hash", SteelDataValue::String(normalized.source.hash)),
            ]),
        ),
        ("units", keyword("mm")),
        (
            "topology",
            map(vec![
                ("solids", int(normalized.topology.solids)?),
                ("shells", int(normalized.topology.shells)?),
                ("faces", int(normalized.topology.faces)?),
                ("edges", int(normalized.topology.edges)?),
                ("vertices", int(normalized.topology.vertices)?),
            ]),
        ),
        (
            "bounds",
            map(vec![
                ("min", coords(normalized.bounds.min)),
                ("max", coords(normalized.bounds.max)),
            ]),
        ),
        (
            "parts",
            SteelDataValue::Vector(
                normalized
                    .parts
                    .into_iter()
                    .map(|part| {
                        map(vec![
                            ("id", SteelDataValue::String(part.id)),
                            ("label", SteelDataValue::String(part.label)),
                            ("kind", keyword(&part.kind)),
                            ("volume", SteelDataValue::Float(part.volume)),
                            ("area", SteelDataValue::Float(part.area)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ]))
}

fn int(value: u64) -> AppResult<SteelDataValue> {
    i64::try_from(value)
        .map(SteelDataValue::Integer)
        .map_err(|_| err("topology"))
}
fn coords(values: [f64; 3]) -> SteelDataValue {
    SteelDataValue::Vector(values.into_iter().map(SteelDataValue::Float).collect())
}

pub fn decode_shape_summary(value: &SteelDataValue) -> AppResult<ShapeSummary> {
    validate_steel_data(value).map_err(|_| err("root"))?;
    let root = required_map(
        value,
        "root",
        &[
            "schema", "version", "source", "units", "topology", "bounds", "parts",
        ],
    )?;
    if required_keyword(&root, "schema")? != SCHEMA
        || required_int(&root, "version")? != 1
        || required_keyword(&root, "units")? != ":mm"
    {
        return Err(err("root"));
    }
    let source = required_map(
        required(&root, "source")?,
        "source",
        &["format", "name", "hash"],
    )?;
    let topology = required_map(
        required(&root, "topology")?,
        "topology",
        &["solids", "shells", "faces", "edges", "vertices"],
    )?;
    let bounds = required_map(required(&root, "bounds")?, "bounds", &["min", "max"])?;
    let parts = match required(&root, "parts")? {
        SteelDataValue::Vector(parts) => parts,
        _ => return Err(err("parts")),
    };
    let json = FreecadShapeSummaryJson {
        source: FreecadSourceJson {
            format: required_keyword(&source, "format")?
                .trim_start_matches(':')
                .into(),
            name: required_string(&source, "name")?.into(),
            hash: required_string(&source, "hash")?.into(),
        },
        topology: FreecadTopologyJson {
            solids: uint(required_int(&topology, "solids")?, "topology")?,
            shells: uint(required_int(&topology, "shells")?, "topology")?,
            faces: uint(required_int(&topology, "faces")?, "topology")?,
            edges: uint(required_int(&topology, "edges")?, "topology")?,
            vertices: uint(required_int(&topology, "vertices")?, "topology")?,
        },
        bounds: FreecadBoundsJson {
            min: decode_coords(required(&bounds, "min")?)?,
            max: decode_coords(required(&bounds, "max")?)?,
        },
        parts: parts.iter().map(decode_part).collect::<AppResult<_>>()?,
    };
    normalize_freecad_shape_summary(json)
}

fn required_map<'a>(
    value: &'a SteelDataValue,
    field: &str,
    allowed: &[&str],
) -> AppResult<Vec<(&'a str, &'a SteelDataValue)>> {
    let SteelDataValue::Map(entries) = value else {
        return Err(err(field));
    };
    if entries.len() != allowed.len() {
        return Err(err(field));
    }
    let mut result = Vec::new();
    for (key, value) in entries {
        let key = key.strip_prefix(':').ok_or_else(|| err(field))?;
        if !allowed.contains(&key) || result.iter().any(|(seen, _)| *seen == key) {
            return Err(err(field));
        }
        result.push((key, value));
    }
    Ok(result)
}
fn required<'a>(
    entries: &'a Vec<(&str, &'a SteelDataValue)>,
    key: &str,
) -> AppResult<&'a SteelDataValue> {
    entries
        .iter()
        .find_map(|(name, value)| (*name == key).then_some(*value))
        .ok_or_else(|| err(key))
}
fn required_string<'a>(
    entries: &'a Vec<(&str, &'a SteelDataValue)>,
    key: &str,
) -> AppResult<&'a str> {
    match required(entries, key)? {
        SteelDataValue::String(value) => Ok(value),
        _ => Err(err(key)),
    }
}
fn required_keyword<'a>(
    entries: &'a Vec<(&str, &'a SteelDataValue)>,
    key: &str,
) -> AppResult<&'a str> {
    match required(entries, key)? {
        SteelDataValue::Keyword(value) => Ok(value),
        _ => Err(err(key)),
    }
}
fn required_int(entries: &Vec<(&str, &SteelDataValue)>, key: &str) -> AppResult<i64> {
    match required(entries, key)? {
        SteelDataValue::Integer(value) => Ok(*value),
        _ => Err(err(key)),
    }
}
fn uint(value: i64, field: &str) -> AppResult<u64> {
    u64::try_from(value).map_err(|_| err(field))
}
fn decode_coords(value: &SteelDataValue) -> AppResult<Vec<f64>> {
    let SteelDataValue::Vector(values) = value else {
        return Err(err("bounds"));
    };
    if values.len() != 3 {
        return Err(err("bounds"));
    }
    values
        .iter()
        .map(|value| match value {
            SteelDataValue::Integer(value) => Ok(*value as f64),
            SteelDataValue::Float(value) if value.is_finite() => Ok(*value),
            _ => Err(err("bounds")),
        })
        .collect()
}
fn decode_part(value: &SteelDataValue) -> AppResult<FreecadPartJson> {
    let part = required_map(value, "parts", &["id", "label", "kind", "volume", "area"])?;
    Ok(FreecadPartJson {
        id: required_string(&part, "id")?.into(),
        label: required_string(&part, "label")?.into(),
        kind: required_keyword(&part, "kind")?
            .trim_start_matches(':')
            .into(),
        volume: number(required(&part, "volume")?, "parts.volume")?,
        area: number(required(&part, "area")?, "parts.area")?,
    })
}
fn number(value: &SteelDataValue, field: &str) -> AppResult<f64> {
    match value {
        SteelDataValue::Integer(value) => Ok(*value as f64),
        SteelDataValue::Float(value) if value.is_finite() => Ok(*value),
        _ => Err(err(field)),
    }
}
