use serde::{de::DeserializeOwned, Serialize};

use crate::steel_data::{parse_steel_data, write_steel_data, SteelDataValue};

pub fn to_vec<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let value = serde_json::to_value(value).map_err(|error| error.to_string())?;
    let edn = write_steel_data(&json_to_steel(value)?).map_err(|error| error.to_string())?;
    Ok(edn.into_bytes())
}

pub fn from_slice<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    let value = parse_steel_data(text).map_err(|error| error.to_string())?;
    serde_json::from_value(steel_to_json(value)?).map_err(|error| error.to_string())
}

fn json_to_steel(value: serde_json::Value) -> Result<SteelDataValue, String> {
    match value {
        serde_json::Value::Null => Ok(SteelDataValue::Nil),
        serde_json::Value::Bool(value) => Ok(SteelDataValue::Bool(value)),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(SteelDataValue::Integer(value))
            } else if let Some(value) = value.as_u64() {
                i64::try_from(value)
                    .map(SteelDataValue::Integer)
                    .map_err(|_| "EDN integer exceeds signed 64-bit range".to_string())
            } else {
                value
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .map(SteelDataValue::Float)
                    .ok_or_else(|| "EDN float must be finite".to_string())
            }
        }
        serde_json::Value::String(value) => Ok(SteelDataValue::String(value)),
        serde_json::Value::Array(values) => values
            .into_iter()
            .map(json_to_steel)
            .collect::<Result<Vec<_>, _>>()
            .map(SteelDataValue::Vector),
        serde_json::Value::Object(values) => values
            .into_iter()
            .map(|(key, value)| Ok((format!(":{}", camel_to_kebab(&key)?), json_to_steel(value)?)))
            .collect::<Result<Vec<_>, String>>()
            .map(SteelDataValue::Map),
    }
}

fn steel_to_json(value: SteelDataValue) -> Result<serde_json::Value, String> {
    match value {
        SteelDataValue::Nil => Ok(serde_json::Value::Null),
        SteelDataValue::Bool(value) => Ok(serde_json::Value::Bool(value)),
        SteelDataValue::Integer(value) => Ok(serde_json::Value::Number(value.into())),
        SteelDataValue::Float(value) => serde_json::Number::from_f64(value)
            .map(serde_json::Value::Number)
            .ok_or_else(|| "EDN float must be finite".to_string()),
        SteelDataValue::String(value) => Ok(serde_json::Value::String(value)),
        SteelDataValue::Keyword(value) => Ok(serde_json::Value::String(value)),
        SteelDataValue::Vector(values) => values
            .into_iter()
            .map(steel_to_json)
            .collect::<Result<Vec<_>, _>>()
            .map(serde_json::Value::Array),
        SteelDataValue::Map(values) => values
            .into_iter()
            .map(|(key, value)| {
                let key = key
                    .strip_prefix(':')
                    .ok_or_else(|| "EDN map key must be a keyword".to_string())?;
                Ok((kebab_to_camel(key)?, steel_to_json(value)?))
            })
            .collect::<Result<serde_json::Map<_, _>, String>>()
            .map(serde_json::Value::Object),
    }
}

fn camel_to_kebab(key: &str) -> Result<String, String> {
    let mut result = String::with_capacity(key.len() + 4);
    for character in key.chars() {
        if character.is_ascii_uppercase() {
            if result.is_empty() || result.ends_with('-') {
                return Err(format!("EDN key '{key}' is not lower camelCase"));
            }
            result.push('-');
            result.push(character.to_ascii_lowercase());
        } else if character == '_' {
            result.push('-');
        } else if character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-' {
            result.push(character);
        } else {
            return Err(format!("EDN key '{key}' contains unsupported characters"));
        }
    }
    if result.is_empty() {
        return Err("EDN map key must not be empty".to_string());
    }
    Ok(result)
}

fn kebab_to_camel(key: &str) -> Result<String, String> {
    let mut segments = key.split('-');
    let first = segments
        .next()
        .filter(|segment| !segment.is_empty())
        .ok_or_else(|| "EDN map key must not be empty".to_string())?;
    let mut result = first.to_string();
    for segment in segments {
        if segment.is_empty() {
            return Err(format!("EDN key '{key}' is not kebab-case"));
        }
        let mut characters = segment.chars();
        let first = characters.next().expect("non-empty segment");
        result.push(first.to_ascii_uppercase());
        result.extend(characters);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        schema_version: u32,
        digest: String,
        values: Vec<f64>,
        optional: Option<String>,
    }

    #[test]
    fn strict_edn_round_trip_uses_keyword_map_keys() {
        let fixture = Fixture {
            schema_version: 1,
            digest: "sha256:test".to_string(),
            values: vec![1.0, 2.5],
            optional: None,
        };
        let bytes = to_vec(&fixture).expect("encode EDN");
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.starts_with("{:"));
        assert!(text.contains(":schema-version 1"));
        assert_eq!(from_slice::<Fixture>(&bytes).unwrap(), fixture);
    }
}
