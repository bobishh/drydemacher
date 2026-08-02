//! Strict, data-only EDN subset. This module never invokes Steel parsing or evaluation.

use std::collections::HashSet;
use std::fmt;

pub const MAX_INPUT_BYTES: usize = 1024 * 1024;
pub const MAX_DEPTH: usize = 64;
pub const MAX_NODES: usize = 100_000;
pub const MAX_STRING_BYTES: usize = 256 * 1024;
pub const MAX_COLLECTION_ENTRIES: usize = 10_000;
pub const MAX_NUMBER_TOKEN_BYTES: usize = 128;

#[derive(Clone, Debug, PartialEq)]
pub enum SteelDataValue {
    Nil,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Keyword(String),
    Vector(Vec<SteelDataValue>),
    Map(Vec<(String, SteelDataValue)>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SteelDataError {
    pub offset: usize,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl fmt::Display for SteelDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at byte {}, line {}, column {}",
            self.message, self.offset, self.line, self.column
        )
    }
}

impl std::error::Error for SteelDataError {}

pub fn parse_steel_data(input: &str) -> Result<SteelDataValue, SteelDataError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(error_at(
            input,
            MAX_INPUT_BYTES,
            "input exceeds 1 MiB limit",
        ));
    }
    let mut parser = Parser {
        input,
        pos: 0,
        nodes: 0,
    };
    parser.skip_separators();
    if parser.eof() {
        return Err(parser.error("expected one top-level form"));
    }
    let value = parser.value(0)?;
    parser.skip_separators();
    if !parser.eof() {
        return Err(parser.error("expected EOF after top-level form"));
    }
    Ok(value)
}

pub fn write_steel_data(value: &SteelDataValue) -> Result<String, SteelDataError> {
    let mut writer = Writer {
        out: String::new(),
        nodes: 0,
    };
    writer.value(value, 0)?;
    writer.push_char('\n')?;
    Ok(writer.out)
}

/// Validate a programmatically constructed value against the same bounded data
/// domain accepted by the reader. This never serializes or evaluates the value.
pub fn validate_steel_data(value: &SteelDataValue) -> Result<(), SteelDataError> {
    let mut validator = ValueValidator { nodes: 0 };
    validator.value(value, 0)
}

/// Convert strict data into Steel runtime values without parsing or evaluating.
/// Steel represents keywords as colon-prefixed immutable `SymbolV` values.
pub fn to_immutable_steel(
    value: &SteelDataValue,
) -> Result<steel_core::rvals::SteelVal, SteelDataError> {
    use steel_core::rvals::{SteelHashMap, SteelVal, SteelVector};
    validate_steel_data(value)?;
    match value {
        SteelDataValue::Nil => Ok(SteelVal::Void),
        SteelDataValue::Bool(value) => Ok(SteelVal::BoolV(*value)),
        SteelDataValue::Integer(value) => Ok(SteelVal::from(*value)),
        SteelDataValue::Float(value) if value.is_finite() => Ok(SteelVal::NumV(*value)),
        SteelDataValue::String(value) => Ok(SteelVal::StringV(value.clone().into())),
        SteelDataValue::Keyword(value) => Ok(SteelVal::SymbolV(value.clone().into())),
        SteelDataValue::Vector(values) => Ok(SteelVal::VectorV(
            values
                .iter()
                .map(to_immutable_steel)
                .collect::<Result<SteelVector, _>>()?,
        )),
        SteelDataValue::Map(entries) => {
            let map = entries
                .iter()
                .map(|(key, value)| {
                    Ok((
                        SteelVal::SymbolV(key.clone().into()),
                        to_immutable_steel(value)?,
                    ))
                })
                .collect::<Result<im_rc::HashMap<SteelVal, SteelVal>, SteelDataError>>()?;
            Ok(SteelVal::HashMapV(SteelHashMap::from(
                steel_core::gc::Gc::new(map),
            )))
        }
        SteelDataValue::Float(_) => Err(ValueValidator::error(
            "non-finite float cannot convert to Steel",
        )),
    }
}

/// Convert only immutable Steel data variants. This never invokes parser, Engine,
/// evaluator, macro expander, or mutable runtime access.
pub fn from_immutable_steel(
    value: &steel_core::rvals::SteelVal,
) -> Result<SteelDataValue, SteelDataError> {
    use steel_core::rvals::SteelVal;
    let result = match value {
        SteelVal::Void => SteelDataValue::Nil,
        SteelVal::BoolV(value) => SteelDataValue::Bool(*value),
        SteelVal::IntV(value) => SteelDataValue::Integer(*value as i64),
        SteelVal::BigNum(value) => SteelDataValue::Integer(
            value
                .to_string()
                .parse()
                .map_err(|_| ValueValidator::error("Steel integer outside i64"))?,
        ),
        SteelVal::NumV(value) if value.is_finite() => SteelDataValue::Float(*value),
        SteelVal::StringV(value) => SteelDataValue::String(value.to_string()),
        SteelVal::SymbolV(value) if value.starts_with(':') => {
            SteelDataValue::Keyword(value.to_string())
        }
        SteelVal::VectorV(values) => SteelDataValue::Vector(
            values
                .iter()
                .map(from_immutable_steel)
                .collect::<Result<_, _>>()?,
        ),
        SteelVal::HashMapV(values) => {
            let mut entries = Vec::with_capacity(values.len());
            for (key, value) in values.iter() {
                let SteelVal::SymbolV(key) = key else {
                    return Err(ValueValidator::error("Steel map key is not a keyword"));
                };
                if !key.starts_with(':') {
                    return Err(ValueValidator::error("Steel map key is not a keyword"));
                }
                entries.push((key.to_string(), from_immutable_steel(value)?));
            }
            SteelDataValue::Map(entries)
        }
        _ => return Err(ValueValidator::error("unsupported Steel value")),
    };
    validate_steel_data(&result)?;
    Ok(result)
}

struct ValueValidator {
    nodes: usize,
}

impl ValueValidator {
    fn error(message: impl Into<String>) -> SteelDataError {
        SteelDataError {
            offset: 0,
            line: 1,
            column: 1,
            message: message.into(),
        }
    }

    fn value(&mut self, value: &SteelDataValue, depth: usize) -> Result<(), SteelDataError> {
        if depth > MAX_DEPTH {
            return Err(Self::error("nesting depth exceeds 64"));
        }
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or_else(|| Self::error("node count overflow"))?;
        if self.nodes > MAX_NODES {
            return Err(Self::error("node limit exceeds 100000"));
        }
        match value {
            SteelDataValue::Float(value) if !value.is_finite() => {
                Err(Self::error("non-finite float cannot be serialized"))
            }
            SteelDataValue::String(value) if value.len() > MAX_STRING_BYTES => {
                Err(Self::error("decoded string exceeds 256 KiB"))
            }
            SteelDataValue::Keyword(value) if !valid_keyword(value) => {
                Err(Self::error("invalid keyword cannot be serialized"))
            }
            SteelDataValue::Vector(values) => {
                if values.len() > MAX_COLLECTION_ENTRIES {
                    return Err(Self::error("vector entry limit exceeds 10000"));
                }
                for value in values {
                    self.value(value, depth + 1)?;
                }
                Ok(())
            }
            SteelDataValue::Map(entries) => {
                if entries.len() > MAX_COLLECTION_ENTRIES {
                    return Err(Self::error("map entry limit exceeds 10000"));
                }
                let mut seen = HashSet::new();
                for (key, value) in entries {
                    if !valid_keyword(key) {
                        return Err(Self::error("invalid map keyword cannot be serialized"));
                    }
                    if !seen.insert(key.as_str()) {
                        return Err(Self::error("duplicate map key"));
                    }
                    self.value(value, depth + 1)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
    nodes: usize,
}

impl<'a> Parser<'a> {
    fn eof(&self) -> bool {
        self.pos == self.input.len()
    }
    fn error(&self, message: impl Into<String>) -> SteelDataError {
        error_at(self.input, self.pos, message)
    }
    fn bump_node(&mut self) -> Result<(), SteelDataError> {
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or_else(|| self.error("node count overflow"))?;
        if self.nodes > MAX_NODES {
            return Err(self.error("node limit exceeds 100000"));
        }
        Ok(())
    }
    fn byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.pos).copied()
    }
    fn advance_char(&mut self) {
        self.pos += self.input[self.pos..].chars().next().unwrap().len_utf8();
    }
    fn skip_separators(&mut self) {
        loop {
            match self.byte() {
                Some(b',') | Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n') => self.pos += 1,
                Some(b';') => {
                    while let Some(ch) = self.byte() {
                        self.pos += 1;
                        if ch == b'\n' {
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
    }
    fn value(&mut self, depth: usize) -> Result<SteelDataValue, SteelDataError> {
        if depth > MAX_DEPTH {
            return Err(self.error("nesting depth exceeds 64"));
        }
        self.bump_node()?;
        match self.byte() {
            Some(b'{') => self.map(depth + 1),
            Some(b'[') => self.vector(depth + 1),
            Some(b'\"') => self.string().map(SteelDataValue::String),
            Some(b':') => self.keyword().map(SteelDataValue::Keyword),
            Some(b'(') => Err(self.error("lists are forbidden")),
            Some(b'\'') | Some(b'`') | Some(b'~') => Err(self.error("quote forms are forbidden")),
            Some(b'#') => Err(self.error("tags and sets are forbidden")),
            Some(_) => self.atom(),
            None => Err(self.error("unexpected EOF")),
        }
    }
    fn vector(&mut self, depth: usize) -> Result<SteelDataValue, SteelDataError> {
        self.pos += 1;
        self.skip_separators();
        let mut values = Vec::new();
        while self.byte() != Some(b']') {
            if self.eof() {
                return Err(self.error("unterminated vector"));
            }
            if values.len() >= MAX_COLLECTION_ENTRIES {
                return Err(self.error("vector entry limit exceeds 10000"));
            }
            values
                .try_reserve(1)
                .map_err(|_| self.error("vector allocation limit exceeded"))?;
            values.push(self.value(depth)?);
            self.skip_separators();
        }
        self.pos += 1;
        Ok(SteelDataValue::Vector(values))
    }
    fn map(&mut self, depth: usize) -> Result<SteelDataValue, SteelDataError> {
        self.pos += 1;
        self.skip_separators();
        let mut pairs = Vec::new();
        let mut seen = HashSet::new();
        while self.byte() != Some(b'}') {
            if self.eof() {
                return Err(self.error("unterminated map"));
            }
            if pairs.len() >= MAX_COLLECTION_ENTRIES {
                return Err(self.error("map entry limit exceeds 10000"));
            }
            if self.byte() != Some(b':') {
                return Err(self.error("map keys must be keywords"));
            }
            let key_start = self.pos;
            self.bump_node()?;
            let key = self.keyword()?;
            let key_token = &self.input[key_start..self.pos];
            seen.try_reserve(1)
                .map_err(|_| self.error("map key allocation limit exceeded"))?;
            if !seen.insert(key_token) {
                return Err(error_at(self.input, key_start, "duplicate map key"));
            }
            self.skip_separators();
            if self.byte() == Some(b'}') || self.eof() {
                return Err(self.error("map requires value after key"));
            }
            pairs
                .try_reserve(1)
                .map_err(|_| self.error("map allocation limit exceeded"))?;
            pairs.push((key, self.value(depth)?));
            self.skip_separators();
        }
        self.pos += 1;
        Ok(SteelDataValue::Map(pairs))
    }
    fn keyword(&mut self) -> Result<String, SteelDataError> {
        let start = self.pos;
        self.pos += 1;
        while self.byte().is_some_and(|b| !is_delimiter(b)) {
            self.advance_char();
        }
        let token = &self.input[start..self.pos];
        if !valid_keyword(token) {
            return Err(error_at(self.input, start, "invalid keyword"));
        }
        let mut keyword = String::new();
        keyword
            .try_reserve(token.len())
            .map_err(|_| error_at(self.input, start, "keyword allocation limit exceeded"))?;
        keyword.push_str(token);
        Ok(keyword)
    }
    fn atom(&mut self) -> Result<SteelDataValue, SteelDataError> {
        let start = self.pos;
        while self.byte().is_some_and(|b| !is_delimiter(b)) {
            self.advance_char();
        }
        let token = &self.input[start..self.pos];
        if token.is_empty() {
            return Err(self.error("unexpected token"));
        }
        if token.len() > MAX_NUMBER_TOKEN_BYTES {
            return Err(error_at(
                self.input,
                start,
                "numeric token exceeds 128 bytes",
            ));
        }
        match token {
            "nil" => Ok(SteelDataValue::Nil),
            "true" => Ok(SteelDataValue::Bool(true)),
            "false" => Ok(SteelDataValue::Bool(false)),
            _ => self.number(token, start),
        }
    }
    fn number(&self, token: &str, start: usize) -> Result<SteelDataValue, SteelDataError> {
        if valid_integer(token) {
            return token
                .parse::<i64>()
                .map(SteelDataValue::Integer)
                .map_err(|_| error_at(self.input, start, "integer overflow"));
        }
        if valid_float(token) {
            return token
                .parse::<f64>()
                .ok()
                .filter(|n| n.is_finite())
                .map(SteelDataValue::Float)
                .ok_or_else(|| error_at(self.input, start, "non-finite or overflowing float"));
        }
        Err(error_at(
            self.input,
            start,
            "symbols, dotted forms, and unsupported values are forbidden",
        ))
    }
    fn string(&mut self) -> Result<String, SteelDataError> {
        self.pos += 1;
        let mut out = String::new();
        loop {
            let ch = self.input[self.pos..]
                .chars()
                .next()
                .ok_or_else(|| self.error("unterminated string"))?;
            match ch {
                '\"' => {
                    self.advance_char();
                    return Ok(out);
                }
                '\\' => {
                    self.pos += 1;
                    let escaped = self.escape()?;
                    out.try_reserve(escaped.len_utf8())
                        .map_err(|_| self.error("string allocation limit exceeded"))?;
                    out.push(escaped);
                }
                c if c <= '\u{1f}' => {
                    return Err(self.error("unescaped control character in string"))
                }
                c => {
                    self.advance_char();
                    out.try_reserve(c.len_utf8())
                        .map_err(|_| self.error("string allocation limit exceeded"))?;
                    out.push(c);
                }
            }
            if out.len() > MAX_STRING_BYTES {
                return Err(self.error("decoded string exceeds 256 KiB"));
            }
        }
    }
    fn escape(&mut self) -> Result<char, SteelDataError> {
        let b = self
            .byte()
            .ok_or_else(|| self.error("unterminated escape"))?;
        if !b.is_ascii() {
            self.advance_char();
            return Err(self.error("invalid string escape"));
        }
        self.pos += 1;
        match b {
            b'\"' => Ok('\"'),
            b'\\' => Ok('\\'),
            b'/' => Ok('/'),
            b'b' => Ok('\u{8}'),
            b'f' => Ok('\u{c}'),
            b'n' => Ok('\n'),
            b'r' => Ok('\r'),
            b't' => Ok('\t'),
            b'u' => self.unicode_escape(),
            _ => Err(self.error("invalid string escape")),
        }
    }
    fn unicode_escape(&mut self) -> Result<char, SteelDataError> {
        let unit = self.hex_unit()?;
        if (0xd800..=0xdbff).contains(&unit) {
            if self.byte() != Some(b'\\') || self.input.as_bytes().get(self.pos + 1) != Some(&b'u')
            {
                return Err(self.error("high surrogate requires low surrogate"));
            }
            self.pos += 2;
            let low = self.hex_unit()?;
            if !(0xdc00..=0xdfff).contains(&low) {
                return Err(self.error("invalid low surrogate"));
            }
            return char::from_u32(
                0x10000 + (((unit - 0xd800) as u32) << 10) + (low - 0xdc00) as u32,
            )
            .ok_or_else(|| self.error("invalid unicode escape"));
        }
        if (0xdc00..=0xdfff).contains(&unit) {
            return Err(self.error("unexpected low surrogate"));
        }
        char::from_u32(unit as u32).ok_or_else(|| self.error("invalid unicode escape"))
    }
    fn hex_unit(&mut self) -> Result<u16, SteelDataError> {
        let mut value = 0u16;
        for _ in 0..4 {
            let byte = self
                .byte()
                .ok_or_else(|| self.error("truncated unicode escape"))?;
            let digit = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => return Err(self.error("invalid unicode escape")),
            };
            value = value * 16 + u16::from(digit);
            self.pos += 1;
        }
        Ok(value)
    }
}

fn is_delimiter(b: u8) -> bool {
    matches!(
        b,
        b',' | b' '
            | b'\t'
            | b'\r'
            | b'\n'
            | b';'
            | b'{'
            | b'}'
            | b'['
            | b']'
            | b'('
            | b')'
            | b'\"'
    )
}
fn valid_keyword(token: &str) -> bool {
    let body = match token.strip_prefix(':') {
        Some(body) if !body.is_empty() => body,
        _ => return false,
    };
    let mut parts = body.split('/');
    let first = parts.next().unwrap();
    let second = parts.next();
    parts.next().is_none() && valid_kebab(first) && second.is_none_or(valid_kebab)
}
fn valid_kebab(part: &str) -> bool {
    !part.is_empty() && part.split('-').all(valid_keyword_segment)
}
fn valid_keyword_segment(segment: &str) -> bool {
    let mut bytes = segment.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}
fn valid_integer(token: &str) -> bool {
    let rest = token.strip_prefix('-').unwrap_or(token);
    rest == "0"
        || (!rest.starts_with('0') && !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
}
fn valid_float(token: &str) -> bool {
    let rest = token.strip_prefix('-').unwrap_or(token);
    if rest.is_empty()
        || rest.starts_with('0')
            && rest.len() > 1
            && !matches!(rest.as_bytes()[1], b'.' | b'e' | b'E')
    {
        return false;
    }
    let (mantissa, exponent) = match rest.find(['e', 'E']) {
        Some(i) => (&rest[..i], Some(&rest[i + 1..])),
        None => (rest, None),
    };
    let mantissa_ok = match mantissa.split_once('.') {
        Some((whole, frac)) => {
            !whole.is_empty()
                && whole.bytes().all(|b| b.is_ascii_digit())
                && !frac.is_empty()
                && frac.bytes().all(|b| b.is_ascii_digit())
        }
        None => mantissa.bytes().all(|b| b.is_ascii_digit()),
    };
    let exponent_ok = exponent.is_none_or(|e| {
        let e = e.strip_prefix(['+', '-']).unwrap_or(e);
        !e.is_empty() && e.bytes().all(|b| b.is_ascii_digit())
    });
    mantissa_ok && exponent_ok && (mantissa.contains('.') || exponent.is_some())
}
fn error_at(input: &str, offset: usize, message: impl Into<String>) -> SteelDataError {
    let requested = offset.min(input.len());
    let safe = if requested == input.len() {
        requested
    } else {
        input
            .char_indices()
            .take_while(|(index, _)| *index <= requested)
            .map(|(index, _)| index)
            .last()
            .unwrap_or(0)
    };
    let mut line = 1;
    let mut column = 1;
    for ch in input[..safe].chars() {
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    SteelDataError {
        offset: safe,
        line,
        column,
        message: message.into(),
    }
}

struct Writer {
    out: String,
    nodes: usize,
}

impl Writer {
    fn error(message: impl Into<String>) -> SteelDataError {
        SteelDataError {
            offset: 0,
            line: 1,
            column: 1,
            message: message.into(),
        }
    }

    fn bump_node(&mut self) -> Result<(), SteelDataError> {
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or_else(|| Self::error("node count overflow"))?;
        if self.nodes > MAX_NODES {
            return Err(Self::error("node limit exceeds 100000"));
        }
        Ok(())
    }

    fn reserve(&mut self, additional: usize) -> Result<(), SteelDataError> {
        let output_len = self
            .out
            .len()
            .checked_add(additional)
            .ok_or_else(|| Self::error("canonical output size overflow"))?;
        if output_len > MAX_INPUT_BYTES {
            return Err(Self::error("canonical output exceeds 1 MiB limit"));
        }
        self.out
            .try_reserve(additional)
            .map_err(|_| Self::error("canonical output allocation limit exceeded"))
    }

    fn push_str(&mut self, text: &str) -> Result<(), SteelDataError> {
        self.reserve(text.len())?;
        self.out.push_str(text);
        Ok(())
    }

    fn push_char(&mut self, ch: char) -> Result<(), SteelDataError> {
        self.reserve(ch.len_utf8())?;
        self.out.push(ch);
        Ok(())
    }

    fn value(&mut self, value: &SteelDataValue, depth: usize) -> Result<(), SteelDataError> {
        if depth > MAX_DEPTH {
            return Err(Self::error("nesting depth exceeds 64"));
        }
        self.bump_node()?;
        match value {
            SteelDataValue::Nil => self.push_str("nil"),
            SteelDataValue::Bool(value) => self.push_str(if *value { "true" } else { "false" }),
            SteelDataValue::Integer(value) => self.push_str(&value.to_string()),
            SteelDataValue::Float(value) => {
                if !value.is_finite() {
                    return Err(Self::error("non-finite float cannot be serialized"));
                }
                self.push_str(&canonical_float(*value))
            }
            SteelDataValue::String(value) => self.string(value),
            SteelDataValue::Keyword(value) => {
                if !valid_keyword(value) {
                    return Err(Self::error("invalid keyword cannot be serialized"));
                }
                self.push_str(value)
            }
            SteelDataValue::Vector(values) => self.vector(values, depth + 1),
            SteelDataValue::Map(values) => self.map(values, depth + 1),
        }
    }

    fn vector(&mut self, values: &[SteelDataValue], depth: usize) -> Result<(), SteelDataError> {
        if values.len() > MAX_COLLECTION_ENTRIES {
            return Err(Self::error("vector entry limit exceeds 10000"));
        }
        self.push_char('[')?;
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                self.push_char(' ')?;
            }
            self.value(value, depth)?;
        }
        self.push_char(']')
    }

    fn map(
        &mut self,
        values: &[(String, SteelDataValue)],
        depth: usize,
    ) -> Result<(), SteelDataError> {
        if values.len() > MAX_COLLECTION_ENTRIES {
            return Err(Self::error("map entry limit exceeds 10000"));
        }
        let mut seen = HashSet::new();
        seen.try_reserve(values.len())
            .map_err(|_| Self::error("map key allocation limit exceeded"))?;
        let mut sorted = Vec::new();
        sorted
            .try_reserve(values.len())
            .map_err(|_| Self::error("map allocation limit exceeded"))?;
        for (key, value) in values {
            self.bump_node()?;
            if !valid_keyword(key) {
                return Err(Self::error("invalid map keyword cannot be serialized"));
            }
            if !seen.insert(key.as_str()) {
                return Err(Self::error("duplicate map key"));
            }
            sorted.push((key, value));
        }
        sorted.sort_by(|(left, _), (right, _)| {
            let left = left.strip_prefix(':').unwrap_or("");
            let right = right.strip_prefix(':').unwrap_or("");
            left.as_bytes().cmp(right.as_bytes())
        });
        self.push_char('{')?;
        for (index, (key, value)) in sorted.into_iter().enumerate() {
            if index > 0 {
                self.push_char(' ')?;
            }
            self.push_str(key)?;
            self.push_char(' ')?;
            self.value(value, depth)?;
        }
        self.push_char('}')
    }

    fn string(&mut self, value: &str) -> Result<(), SteelDataError> {
        if value.len() > MAX_STRING_BYTES {
            return Err(Self::error("decoded string exceeds 256 KiB"));
        }
        self.push_char('"')?;
        for ch in value.chars() {
            match ch {
                '"' => self.push_str("\\\"")?,
                '\\' => self.push_str("\\\\")?,
                '\n' => self.push_str("\\n")?,
                '\r' => self.push_str("\\r")?,
                '\t' => self.push_str("\\t")?,
                '\u{8}' => self.push_str("\\b")?,
                '\u{c}' => self.push_str("\\f")?,
                c if c <= '\u{1f}' => self.push_str(&format!("\\u{:04x}", c as u32))?,
                c => self.push_char(c)?,
            }
        }
        self.push_char('"')
    }
}

fn canonical_float(value: f64) -> String {
    if value == 0.0 {
        return "0.0".into();
    }
    let plain = normalize_float_spelling(value.to_string());
    let exponential = normalize_float_spelling(format!("{value:e}"));
    if plain.len() <= exponential.len() {
        plain
    } else {
        exponential
    }
}

fn normalize_float_spelling(mut text: String) -> String {
    text.make_ascii_lowercase();
    if let Some(e) = text.find('e') {
        let (mantissa, exp) = text.split_at(e);
        let exp = exp[1..].strip_prefix('+').unwrap_or(&exp[1..]);
        let (sign, digits) = exp.strip_prefix('-').map_or(("", exp), |d| ("-", d));
        let digits = digits.trim_start_matches('0');
        let mut mantissa = mantissa.to_owned();
        if !mantissa.contains('.') {
            mantissa.push_str(".0");
        }
        return format!(
            "{mantissa}e{sign}{}",
            if digits.is_empty() { "0" } else { digits }
        );
    }
    if !text.contains('.') {
        text.push_str(".0");
    }
    text
}
