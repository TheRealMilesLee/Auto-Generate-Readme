//! Minimal JSON value, parser and pretty printer.
//!
//! Used for `facts.json`, README metadata comments and LLM responses. Keeping
//! this local avoids a serde dependency and keeps the binary self-contained.

use std::fmt::Write as _;

use crate::bail;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn obj() -> Self {
        Json::Obj(Vec::new())
    }

    pub fn arr() -> Self {
        Json::Arr(Vec::new())
    }

    pub fn push(&mut self, key: impl Into<String>, value: impl Into<Json>) {
        if let Json::Obj(entries) = self {
            entries.push((key.into(), value.into()));
        }
    }

    pub fn push_value(&mut self, value: impl Into<Json>) {
        if let Json::Arr(items) = self {
            items.push(value.into());
        }
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(entries) => entries
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Json::UInt(value) => Some(*value),
            Json::Int(value) if *value >= 0 => Some(*value as u64),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(items) => Some(items),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Json::Null)
    }

    pub fn to_compact(&self) -> String {
        let mut out = String::new();
        self.write_compact(&mut out);
        out
    }

    pub fn to_pretty(&self) -> String {
        let mut out = String::new();
        self.write_pretty(&mut out, 0);
        out
    }

    fn write_compact(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(true) => out.push_str("true"),
            Json::Bool(false) => out.push_str("false"),
            Json::Int(value) => {
                let _ = write!(out, "{value}");
            }
            Json::UInt(value) => {
                let _ = write!(out, "{value}");
            }
            Json::Float(value) => {
                if value.fract() == 0.0 && value.abs() < 1e15 {
                    let _ = write!(out, "{value:.1}");
                } else {
                    let _ = write!(out, "{value}");
                }
            }
            Json::Str(value) => write_json_string(out, value),
            Json::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write_compact(out);
                }
                out.push(']');
            }
            Json::Obj(entries) => {
                out.push('{');
                for (i, (key, value)) in entries.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_json_string(out, key);
                    out.push(':');
                    value.write_compact(out);
                }
                out.push('}');
            }
        }
    }

    fn write_pretty(&self, out: &mut String, depth: usize) {
        let pad = |out: &mut String, depth: usize| {
            for _ in 0..depth {
                out.push_str("  ");
            }
        };
        match self {
            Json::Arr(items) if !items.is_empty() => {
                out.push_str("[\n");
                for (i, item) in items.iter().enumerate() {
                    pad(out, depth + 1);
                    item.write_pretty(out, depth + 1);
                    if i + 1 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                pad(out, depth);
                out.push(']');
            }
            Json::Obj(entries) if !entries.is_empty() => {
                out.push_str("{\n");
                for (i, (key, value)) in entries.iter().enumerate() {
                    pad(out, depth + 1);
                    write_json_string(out, key);
                    out.push_str(": ");
                    value.write_pretty(out, depth + 1);
                    if i + 1 < entries.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                pad(out, depth);
                out.push('}');
            }
            other => other.write_compact(out),
        }
    }
}

fn write_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

impl From<&str> for Json {
    fn from(value: &str) -> Self {
        Json::Str(value.to_string())
    }
}

impl From<String> for Json {
    fn from(value: String) -> Self {
        Json::Str(value)
    }
}

impl From<u64> for Json {
    fn from(value: u64) -> Self {
        Json::UInt(value)
    }
}

impl From<usize> for Json {
    fn from(value: usize) -> Self {
        Json::UInt(value as u64)
    }
}

impl From<bool> for Json {
    fn from(value: bool) -> Self {
        Json::Bool(value)
    }
}

impl<T: Into<Json>> From<Option<T>> for Json {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => value.into(),
            None => Json::Null,
        }
    }
}

pub fn parse(input: &str) -> Result<Json> {
    let mut parser = Parser {
        input,
        bytes: input.as_bytes(),
        pos: 0,
    };
    parser.skip_ws();
    let value = parser.value()?;
    parser.skip_ws();
    if parser.pos != parser.bytes.len() {
        bail!("JSON: trailing characters at byte {}", parser.pos);
    }
    Ok(value)
}

struct Parser<'a> {
    input: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn next_char(&mut self) -> Result<char> {
        let ch = self.input[self.pos..]
            .chars()
            .next()
            .ok_or_else(|| crate::error::Error::new("JSON: unexpected end of input"))?;
        self.pos += ch.len_utf8();
        Ok(ch)
    }

    fn skip_ws(&mut self) {
        while let Some(byte) = self.peek() {
            if matches!(byte, b' ' | b'\n' | b'\r' | b'\t') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, byte: u8) -> Result<()> {
        if self.peek() == Some(byte) {
            self.pos += 1;
            Ok(())
        } else {
            bail!(
                "JSON: expected '{}' at byte {}",
                byte as char,
                self.pos
            )
        }
    }

    fn value(&mut self) -> Result<Json> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') => {
                self.literal("true")?;
                Ok(Json::Bool(true))
            }
            Some(b'f') => {
                self.literal("false")?;
                Ok(Json::Bool(false))
            }
            Some(b'n') => {
                self.literal("null")?;
                Ok(Json::Null)
            }
            Some(byte) if byte == b'-' || byte.is_ascii_digit() => self.number(),
            Some(byte) => bail!(
                "JSON: unexpected character '{}' at byte {}",
                byte as char,
                self.pos
            ),
            None => bail!("JSON: unexpected end of input"),
        }
    }

    fn literal(&mut self, literal: &str) -> Result<()> {
        if self.input[self.pos..].starts_with(literal) {
            self.pos += literal.len();
            Ok(())
        } else {
            bail!("JSON: expected '{}' at byte {}", literal, self.pos)
        }
    }

    fn object(&mut self) -> Result<Json> {
        self.expect(b'{')?;
        let mut entries = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Obj(entries));
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            self.expect(b':')?;
            let value = self.value()?;
            entries.push((key, value));
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => bail!("JSON: expected ',' or '}}' at byte {}", self.pos),
            }
        }
        Ok(Json::Obj(entries))
    }

    fn array(&mut self) -> Result<Json> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            items.push(self.value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => bail!("JSON: expected ',' or ']' at byte {}", self.pos),
            }
        }
        Ok(Json::Arr(items))
    }

    fn string(&mut self) -> Result<String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let ch = self.next_char()?;
            match ch {
                '"' => break,
                '\\' => {
                    let escaped = self.next_char()?;
                    match escaped {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{08}'),
                        'f' => out.push('\u{0c}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let first = self.hex4()?;
                            let code = if (0xD800..=0xDBFF).contains(&first) {
                                if self.peek() == Some(b'\\') {
                                    self.pos += 1;
                                    if self.next_char()? != 'u' {
                                        bail!("JSON: invalid surrogate escape");
                                    }
                                    let second = self.hex4()?;
                                    if !(0xDC00..=0xDFFF).contains(&second) {
                                        bail!("JSON: invalid low surrogate");
                                    }
                                    0x10000
                                        + ((first as u32 - 0xD800) << 10)
                                        + (second as u32 - 0xDC00)
                                } else {
                                    0xFFFD
                                }
                            } else {
                                first as u32
                            };
                            out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                        }
                        other => bail!("JSON: invalid escape '\\{other}'"),
                    }
                }
                c if (c as u32) < 0x20 => bail!("JSON: control character in string"),
                c => out.push(c),
            }
        }
        Ok(out)
    }

    fn hex4(&mut self) -> Result<u16> {
        let mut value: u16 = 0;
        for _ in 0..4 {
            let ch = self.next_char()?;
            let digit = ch
                .to_digit(16)
                .ok_or_else(|| crate::error::Error::new("JSON: invalid \\u escape"))?;
            value = value * 16 + digit as u16;
        }
        Ok(value)
    }

    fn number(&mut self) -> Result<Json> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while let Some(byte) = self.peek() {
            if byte.is_ascii_digit() || matches!(byte, b'.' | b'e' | b'E' | b'+' | b'-') {
                self.pos += 1;
            } else {
                break;
            }
        }
        let raw = &self.input[start..self.pos];
        if raw.is_empty() || raw == "-" {
            bail!("JSON: invalid number at byte {start}");
        }
        if !raw.contains('.') && !raw.contains('e') && !raw.contains('E') {
            if let Ok(value) = raw.parse::<i64>() {
                return Ok(Json::Int(value));
            }
            if let Ok(value) = raw.parse::<u64>() {
                return Ok(Json::UInt(value));
            }
        }
        let value = raw
            .parse::<f64>()
            .map_err(|_| crate::error::Error::new(format!("JSON: invalid number '{raw}'")))?;
        if !value.is_finite() {
            bail!("JSON: non-finite number '{raw}'");
        }
        Ok(Json::Float(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_serialises() {
        let source = r#"{"a":1,"b":[true,null,"x"],"c":{"d":-2.5}}"#;
        let value = parse(source).unwrap();
        assert_eq!(value.get("a").unwrap().as_u64(), Some(1));
        assert_eq!(
            value.get("b").unwrap().as_array().unwrap()[2].as_str(),
            Some("x")
        );
        assert_eq!(value.to_compact(), source);
    }

    #[test]
    fn handles_unicode_escapes() {
        let value = parse(r#""\u4f60\u597d \ud83d\ude00""#).unwrap();
        assert_eq!(value.as_str(), Some("你好 😀"));
    }

    #[test]
    fn pretty_output_round_trips() {
        let value = parse(r#"{"a":[1,2],"b":"x"}"#).unwrap();
        let pretty = value.to_pretty();
        assert!(pretty.contains("\n  \"a\""));
        assert_eq!(parse(&pretty).unwrap(), value);
    }

    #[test]
    fn rejects_trailing_data() {
        assert!(parse("{} x").is_err());
        assert!(parse("{\"a\":}").is_err());
    }
}
