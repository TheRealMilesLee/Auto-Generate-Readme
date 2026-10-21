//! A deliberately small YAML subset parser for `readme.yml`.
//!
//! Supported:
//! - nested mappings and sequences (`- item`, `- key: value`)
//! - quoted keys and values (single/double quotes, common escapes)
//! - literal (`|`) and folded (`>`) block scalars with chomping indicators
//! - flow sequences (`[en, zh]`), inline comments, blank lines
//!
//! Not supported: anchors, aliases, tags, multi-document streams, flow maps.
//! Errors carry the 1-based line number.

use crate::bail;
use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Yaml {
    Null,
    Str(String),
    List(Vec<Yaml>),
    Map(Vec<(String, Yaml)>),
}

impl Yaml {
    pub fn get(&self, key: &str) -> Option<&Yaml> {
        match self {
            Yaml::Map(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Yaml::Str(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_list(&self) -> &[Yaml] {
        match self {
            Yaml::List(items) => items,
            _ => &[],
        }
    }

    pub fn as_map(&self) -> &[(String, Yaml)] {
        match self {
            Yaml::Map(entries) => entries,
            _ => &[],
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Yaml::Null)
    }
}

#[derive(Debug, Clone)]
struct Line {
    indent: usize,
    text: String,
    raw: String,
    number: usize,
    blank: bool,
}

pub fn parse(source: &str) -> Result<Yaml> {
    let lines = preprocess(source)?;
    let mut parser = Parser { lines, pos: 0 };
    parser.skip_blank();
    if parser.pos >= parser.lines.len() {
        return Ok(Yaml::Null);
    }
    let indent = parser.current().map(|l| l.indent).unwrap_or(0);
    let value = parser.parse_block(indent)?;
    parser.skip_blank();
    if let Some(line) = parser.current() {
        bail!("readme.yml:{}: unexpected content", line.number);
    }
    Ok(value)
}

fn preprocess(source: &str) -> Result<Vec<Line>> {
    let mut lines = Vec::new();
    for (index, raw_line) in source.lines().enumerate() {
        let number = index + 1;
        let raw = raw_line.trim_end().to_string();
        if raw.trim().is_empty() {
            lines.push(Line {
                indent: 0,
                text: String::new(),
                raw: String::new(),
                number,
                blank: true,
            });
            continue;
        }
        let indent = raw.len() - raw.trim_start_matches(' ').len();
        if raw[indent..].starts_with('\t') {
            bail!("readme.yml:{number}: tabs are not allowed for indentation");
        }
        lines.push(Line {
            indent,
            text: raw[indent..].to_string(),
            raw,
            number,
            blank: false,
        });
    }
    Ok(lines)
}

struct Parser {
    lines: Vec<Line>,
    pos: usize,
}

impl Parser {
    fn current(&self) -> Option<&Line> {
        self.lines.get(self.pos)
    }

    fn skip_blank(&mut self) {
        while let Some(line) = self.current() {
            if line.blank || line.text.trim_start().starts_with('#') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn parse_block(&mut self, indent: usize) -> Result<Yaml> {
        self.skip_blank();
        let Some(line) = self.current() else {
            return Ok(Yaml::Null);
        };
        if line.indent < indent {
            return Ok(Yaml::Null);
        }
        let actual = line.indent;
        if is_sequence_item(&line.text) {
            self.parse_sequence(actual)
        } else {
            self.parse_mapping(actual)
        }
    }

    fn parse_mapping(&mut self, indent: usize) -> Result<Yaml> {
        let mut entries = Vec::new();
        loop {
            self.skip_blank();
            let Some(line) = self.current() else {
                break;
            };
            if line.indent != indent || is_sequence_item(&line.text) {
                break;
            }
            let number = line.number;
            let text = line.text.clone();
            let Some((key_raw, rest)) = split_key(&text) else {
                bail!("readme.yml:{number}: expected 'key: value'");
            };
            let key = parse_key(&key_raw, number)?;
            self.pos += 1;
            let value = self.parse_value_after_key(indent, &rest, number)?;
            entries.push((key, value));
        }
        Ok(Yaml::Map(entries))
    }

    fn parse_value_after_key(
        &mut self,
        parent_indent: usize,
        rest: &str,
        number: usize,
    ) -> Result<Yaml> {
        let trimmed = rest.trim();
        if trimmed.is_empty() {
            self.skip_blank();
            if let Some(next) = self.current() {
                let next_indent = next.indent;
                if next_indent > parent_indent {
                    return self.parse_block(next_indent);
                }
            }
            return Ok(Yaml::Null);
        }
        if trimmed.starts_with('|') || trimmed.starts_with('>') {
            return self.parse_block_scalar(parent_indent, trimmed, number);
        }
        parse_scalar(trimmed, number)
    }

    fn parse_sequence(&mut self, indent: usize) -> Result<Yaml> {
        let mut items = Vec::new();
        loop {
            self.skip_blank();
            let Some(line) = self.current() else {
                break;
            };
            if line.indent != indent || !is_sequence_item(&line.text) {
                break;
            }
            let number = line.number;
            let rest = line.text[1..].trim_start().to_string();
            self.pos += 1;

            if rest.is_empty() {
                self.skip_blank();
                if let Some(next) = self.current() {
                    let next_indent = next.indent;
                    if next_indent > indent {
                        items.push(self.parse_block(next_indent)?);
                        continue;
                    }
                }
                items.push(Yaml::Null);
                continue;
            }

            if rest.starts_with('|') || rest.starts_with('>') {
                items.push(self.parse_block_scalar(indent, &rest, number)?);
                continue;
            }

            if split_key(&rest).is_some() {
                items.push(self.parse_sequence_map_item(indent, &rest, number)?);
            } else {
                items.push(parse_scalar(&rest, number)?);
            }
        }
        Ok(Yaml::List(items))
    }

    fn parse_sequence_map_item(
        &mut self,
        sequence_indent: usize,
        first: &str,
        number: usize,
    ) -> Result<Yaml> {
        let item_indent = sequence_indent + 2;
        let mut entries = Vec::new();

        let Some((key_raw, rest)) = split_key(first) else {
            bail!("readme.yml:{number}: invalid list item");
        };
        let key = parse_key(&key_raw, number)?;
        let value = self.parse_value_after_key(item_indent, &rest, number)?;
        entries.push((key, value));

        loop {
            self.skip_blank();
            let Some(line) = self.current() else {
                break;
            };
            if line.indent != item_indent || is_sequence_item(&line.text) {
                break;
            }
            let number = line.number;
            let text = line.text.clone();
            let Some((key_raw, rest)) = split_key(&text) else {
                bail!("readme.yml:{number}: expected 'key: value'");
            };
            let key = parse_key(&key_raw, number)?;
            self.pos += 1;
            let value = self.parse_value_after_key(item_indent, &rest, number)?;
            entries.push((key, value));
        }

        Ok(Yaml::Map(entries))
    }

    fn parse_block_scalar(
        &mut self,
        parent_indent: usize,
        header: &str,
        number: usize,
    ) -> Result<Yaml> {
        let folded = header.starts_with('>');
        let chomp = header.chars().last().unwrap_or(' ');
        let strip_trailing = chomp == '-';

        let start = self.pos;
        let mut end = start;
        let mut block_indent: Option<usize> = None;

        while end < self.lines.len() {
            let line = &self.lines[end];
            if line.blank {
                end += 1;
                continue;
            }
            if line.indent <= parent_indent {
                break;
            }
            let indent = *block_indent.get_or_insert(line.indent);
            if line.indent < indent {
                break;
            }
            end += 1;
        }

        if block_indent.is_none() {
            self.pos = end;
            return Ok(Yaml::Str(String::new()));
        }
        let indent = block_indent.unwrap_or(parent_indent + 2);

        let mut raw_lines: Vec<String> = Vec::new();
        for line in &self.lines[start..end] {
            if line.blank {
                raw_lines.push(String::new());
            } else if line.raw.len() >= indent {
                raw_lines.push(line.raw[indent..].to_string());
            } else {
                raw_lines.push(String::new());
            }
        }
        self.pos = end;

        let mut text = if folded {
            fold_lines(&raw_lines)
        } else {
            raw_lines.join("\n")
        };
        if strip_trailing {
            text = text.trim_end_matches('\n').to_string();
        }
        let text = text.trim_end().to_string();
        let _ = number;
        Ok(Yaml::Str(text))
    }
}

fn fold_lines(lines: &[String]) -> String {
    let mut out = String::new();
    let mut pending_space = false;
    let mut previous_blank = false;
    for line in lines {
        if line.trim().is_empty() {
            if !out.is_empty() {
                out.push('\n');
            }
            previous_blank = true;
            pending_space = false;
            continue;
        }
        if pending_space {
            out.push(' ');
        }
        out.push_str(line.trim_end());
        pending_space = true;
        previous_blank = false;
    }
    let _ = previous_blank;
    out
}

fn is_sequence_item(text: &str) -> bool {
    text == "-" || text.starts_with("- ")
}

/// Split `key: value` at the first unquoted colon followed by whitespace or EOL.
fn split_key(text: &str) -> Option<(String, String)> {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut chars = text.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' && q == '"' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            ':' => {
                let next = chars.peek().map(|(_, c)| *c);
                if next.is_none() || next == Some(' ') || next == Some('\t') {
                    let key = text[..index].trim().to_string();
                    let value = text[index + 1..].trim_start().to_string();
                    return Some((key, value));
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_key(raw: &str, number: usize) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("readme.yml:{number}: empty key");
    }
    match parse_scalar(trimmed, number)? {
        Yaml::Str(value) => Ok(value),
        Yaml::Null => bail!("readme.yml:{number}: empty key"),
        _ => bail!("readme.yml:{number}: keys must be scalars"),
    }
}

fn parse_scalar(text: &str, number: usize) -> Result<Yaml> {
    let text = strip_comment(text);
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed == "~" || trimmed.eq_ignore_ascii_case("null") {
        return Ok(Yaml::Null);
    }
    if let Some(rest) = trimmed.strip_prefix('"') {
        return Ok(Yaml::Str(parse_double_quoted(rest, number)?));
    }
    if let Some(rest) = trimmed.strip_prefix('\'') {
        return Ok(Yaml::Str(parse_single_quoted(rest, number)?));
    }
    if let Some(inner) = trimmed.strip_prefix('[') {
        let inner = inner
            .strip_suffix(']')
            .ok_or_else(|| Error::new(format!("readme.yml:{number}: unterminated flow sequence")))?;
        let mut items = Vec::new();
        for part in split_flow(inner, ',') {
            if part.trim().is_empty() {
                continue;
            }
            items.push(parse_scalar(part.trim(), number)?);
        }
        return Ok(Yaml::List(items));
    }
    Ok(Yaml::Str(trimmed.to_string()))
}

fn strip_comment(text: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut previous_whitespace = true;
    for (index, ch) in text.char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' && q == '"' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
            previous_whitespace = false;
            continue;
        }
        match ch {
            '"' | '\'' => {
                quote = Some(ch);
                previous_whitespace = false;
            }
            '#' if previous_whitespace => return &text[..index],
            _ => previous_whitespace = ch == ' ' || ch == '\t',
        }
    }
    text
}

fn parse_double_quoted(rest: &str, number: usize) -> Result<String> {
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => return Ok(out),
            '\\' => {
                let escaped = chars
                    .next()
                    .ok_or_else(|| Error::new(format!("readme.yml:{number}: bad escape")))?;
                match escaped {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    'u' => {
                        let mut hex = String::new();
                        for _ in 0..4 {
                            hex.push(chars.next().ok_or_else(|| {
                                Error::new(format!("readme.yml:{number}: bad \\u escape"))
                            })?);
                        }
                        let code = u32::from_str_radix(&hex, 16).map_err(|_| {
                            Error::new(format!("readme.yml:{number}: bad \\u escape"))
                        })?;
                        out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                    }
                    other => out.push(other),
                }
            }
            other => out.push(other),
        }
    }
    bail!("readme.yml:{number}: unterminated double-quoted string")
}

fn parse_single_quoted(rest: &str, number: usize) -> Result<String> {
    let mut out = String::new();
    let mut chars = rest.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\'' {
            if chars.peek() == Some(&'\'') {
                out.push('\'');
                chars.next();
            } else {
                return Ok(out);
            }
        } else {
            out.push(ch);
        }
    }
    bail!("readme.yml:{number}: unterminated single-quoted string")
}

fn split_flow(text: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' && q == '"' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            c if c == separator && depth == 0 => {
                parts.push(&text[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_mappings_and_lists() {
        let source = r#"
preset: showcase
locale: [en, zh]
project:
  name: Demo
  tagline:
    en: "Eight years, one repo"
    zh: 八年，一个仓库
dirs:
  "2026 Wallpaper":
    summary:
      en: Current growth lane
      zh: 当前增长的一年
    tags: [f1, 4k]
"#;
        let value = parse(source).unwrap();
        assert_eq!(value.get("preset").unwrap().as_str(), Some("showcase"));
        assert_eq!(value.get("locale").unwrap().as_list().len(), 2);
        let dirs = value.get("dirs").unwrap();
        let dir = dirs.get("2026 Wallpaper").unwrap();
        assert_eq!(
            dir.get("summary").unwrap().get("zh").unwrap().as_str(),
            Some("当前增长的一年")
        );
    }

    #[test]
    fn parses_block_scalars() {
        let source = "sections:\n  why:\n    en: |\n      Line one.\n      Line two.\n    zh: >-\n      你好，\n      世界。\n";
        let value = parse(source).unwrap();
        let why = value.get("sections").unwrap().get("why").unwrap();
        assert_eq!(why.get("en").unwrap().as_str(), Some("Line one.\nLine two."));
        assert_eq!(why.get("zh").unwrap().as_str(), Some("你好， 世界。"));
    }

    #[test]
    fn parses_list_of_maps() {
        let source = r#"
packs:
  - name:
      en: New machine
      zh: 新机器
    dirs: [2026 Wallpaper, "Apple Park"]
    best_for:
      en: Fresh setup
"#;
        let value = parse(source).unwrap();
        let packs = value.get("packs").unwrap().as_list();
        assert_eq!(packs.len(), 1);
        assert_eq!(
            packs[0].get("name").unwrap().get("zh").unwrap().as_str(),
            Some("新机器")
        );
        assert_eq!(packs[0].get("dirs").unwrap().as_list().len(), 2);
        assert_eq!(
            packs[0].get("best_for").unwrap().get("en").unwrap().as_str(),
            Some("Fresh setup")
        );
    }

    #[test]
    fn comments_and_urls_are_preserved() {
        let source = "clone: https://github.com/a/b.git # comment\nname: demo\n";
        let value = parse(source).unwrap();
        assert_eq!(
            value.get("clone").unwrap().as_str(),
            Some("https://github.com/a/b.git")
        );
    }

    #[test]
    fn rejects_tabs() {
        assert!(parse("a:\n\tb: 1\n").is_err());
    }
}
