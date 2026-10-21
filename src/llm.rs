//! Optional LLM prose generation over an OpenAI-compatible HTTP endpoint.
//!
//! Designed for `llama-server` (llama.cpp) and Ollama, both of which expose
//! `/v1/chat/completions` on localhost. The core generator never requires a
//! model: this module only fills prose sections that `readme.yml` leaves empty.
//!
//! Guard rails:
//! - facts are the only source of numbers and paths;
//! - every response is checked with the same number tracer as `check`;
//! - one retry with the error message, then a warning and a deterministic
//!   fallback (the section is omitted);
//! - responses are cached by facts hash + prompt version + model.

use std::collections::{BTreeMap, HashSet};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

use crate::annotations::Config;
use crate::error::{Error, Result};
use crate::json::{self, Json};
use crate::lang::Lang;
use crate::scan::Facts;
use crate::sha256;
use crate::util::read_text_lossy;
use crate::validate;

const PROMPT_VERSION: &str = "2026-10-21.1";
const MAX_SECTION_CHARS: usize = 6000;

#[derive(Debug, Clone)]
pub struct Options {
    pub endpoint: String,
    pub model: String,
    pub api_key: Option<String>,
    pub temperature: f64,
    pub max_tokens: u32,
    pub timeout: Duration,
    pub cache_dir: PathBuf,
    pub no_cache: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:8080".to_string(),
            model: "local".to_string(),
            api_key: None,
            temperature: 0.2,
            max_tokens: 500,
            timeout: Duration::from_secs(180),
            cache_dir: PathBuf::from(".readme/cache/llm"),
            no_cache: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct Output {
    pub generated: BTreeMap<(String, Lang), String>,
    pub warnings: Vec<String>,
}

pub fn generate(
    facts: &Facts,
    config: &Config,
    locales: &[Lang],
    section_ids: &[&str],
    options: &Options,
) -> Result<Output> {
    let mut output = Output::default();
    let allowed = validate::allowed_numbers(facts, config);

    for id in section_ids {
        for lang in locales {
            if config
                .sections
                .get(*id)
                .and_then(|loc| loc.get(*lang))
                .is_some()
            {
                continue;
            }
            match generate_one(facts, config, id, *lang, &allowed, options) {
                Ok(Some(text)) => {
                    output.generated.insert((id.to_string(), *lang), text);
                }
                Ok(None) => {}
                Err(error) => output.warnings.push(format!(
                    "llm section '{id}' ({}) skipped: {error}",
                    lang.as_str()
                )),
            }
        }
    }
    Ok(output)
}

fn generate_one(
    facts: &Facts,
    config: &Config,
    id: &str,
    lang: Lang,
    allowed: &HashSet<String>,
    options: &Options,
) -> Result<Option<String>> {
    let cache_path = cache_path(facts, id, lang, options);
    if !options.no_cache {
        if let Ok(cached) = read_text_lossy(&cache_path) {
            if validate_content(&cached, allowed).is_ok() {
                return Ok(Some(cached.trim().to_string()));
            }
        }
    }

    let (system, user) = prompt(facts, config, id, lang);
    let mut messages = vec![
        ("system".to_string(), system),
        ("user".to_string(), user),
    ];

    let mut last_error = String::new();
    for attempt in 0..2 {
        let content = chat(options, &messages)?;
        let cleaned = clean(&content);
        match validate_content(&cleaned, allowed) {
            Ok(()) => {
                if !options.no_cache {
                    let _ = write_cache(&cache_path, &cleaned);
                }
                return Ok(Some(cleaned));
            }
            Err(error) => {
                last_error = error;
                if attempt == 0 {
                    messages.push(("assistant".to_string(), cleaned));
                    messages.push((
                        "user".to_string(),
                        format!(
                            "Your answer was rejected by the validator: {last_error}\n\
                             Rewrite the section. Use only numbers that appear verbatim in FACTS, \
                             no headings, no code fences, no meta commentary."
                        ),
                    ));
                }
            }
        }
    }
    Err(Error::new(last_error))
}

fn prompt(
    facts: &Facts,
    config: &Config,
    id: &str,
    lang: Lang,
) -> (String, String) {
    let language = match lang {
        Lang::En => "English",
        Lang::Zh => "Simplified Chinese (简体中文)",
    };
    let purpose = match id {
        "why" => "Explain why this project is worth using, grounded in concrete facts.",
        "audience" => "Describe who this project is for, using its actual categories.",
        "philosophy" => "State the project's engineering or curation principles.",
        _ => "Write a short overview of the project.",
    };

    let system = format!(
        "You are a senior technical writer producing one section of a README.\n\
         Rules:\n\
         - Output only the section body. No heading, no code fence, no preamble, no meta commentary.\n\
         - Every fact you state must come from FACTS. Do not invent numbers, paths, package names,\n\
           people, dates, benchmarks, or claims.\n\
         - Reuse numbers exactly as they appear in FACTS; never compute or round them.\n\
         - Prefer 2-4 short paragraphs. Bullet lists are allowed when they add clarity.\n\
         - Write in {language}.\n\
         - Do not mention the reader, the prompt, or the generation process."
    );

    let mut context = String::new();
    context.push_str(&facts.brief());
    context.push('\n');
    if !facts.description.is_empty() {
        context.push_str(&format!("description: {}\n", facts.description));
    }
    if let Some(license) = &facts.license {
        context.push_str(&format!("license: {license}\n"));
    }
    if let Some(version) = &facts.version {
        context.push_str(&format!("version: {version}\n"));
    }
    if let Some(tagline) = config.project.tagline.get_or_fallback(lang) {
        context.push_str(&format!("tagline: {tagline}\n"));
    }
    if let Some(description) = config.project.description.get_or_fallback(lang) {
        context.push_str(&format!("summary: {description}\n"));
    }
    for (name, dir) in config.dirs.iter().take(12) {
        if let Some(summary) = dir
            .summary
            .get(lang)
            .or_else(|| dir.summary.get_or_fallback(lang))
        {
            context.push_str(&format!("folder {name}: {summary}\n"));
        }
    }
    for pack in config.quickstart.packs.iter().take(6) {
        if let Some(name) = pack.name.get_or_fallback(lang) {
            context.push_str(&format!(
                "pack {name}: {}\n",
                pack.dirs.join(", ")
            ));
        }
    }
    if context.len() > 4000 {
        context = context.chars().take(4000).collect();
    }

    let user = format!(
        "SECTION: {id}\nPURPOSE: {purpose}\n\nFACTS:\n{context}\n\nWrite the section body only."
    );
    (system, user)
}

fn clean(content: &str) -> String {
    let mut text = content.trim().to_string();
    if text.starts_with("```") {
        let mut lines: Vec<&str> = text.lines().collect();
        lines.retain(|line| !line.trim_start().starts_with("```"));
        text = lines.join("\n").trim().to_string();
    }
    // Drop a leading heading if the model wrapped the section in one.
    while text.starts_with('#') {
        let mut lines = text.lines();
        lines.next();
        text = lines.collect::<Vec<_>>().join("\n").trim().to_string();
    }
    text
}

fn validate_content(content: &str, allowed: &HashSet<String>) -> std::result::Result<(), String> {
    if content.trim().is_empty() {
        return Err("empty response".to_string());
    }
    if content.len() > MAX_SECTION_CHARS {
        return Err(format!(
            "response is {} characters (limit {MAX_SECTION_CHARS})",
            content.len()
        ));
    }
    if content.contains("{{") || content.contains("}}") {
        return Err("response contains an unresolved placeholder".to_string());
    }
    if content.contains("```") {
        return Err("response contains a code fence; prose sections must not".to_string());
    }
    if content
        .lines()
        .any(|line| line.trim_start().starts_with('#'))
    {
        return Err("response contains a heading; return the body only".to_string());
    }
    let lower = content.to_lowercase();
    for banned in [
        "as an ai",
        "i cannot",
        "here is the",
        "sure, here",
        "以下是",
        "下面是",
        "作为一个",
        "好的，",
    ] {
        if lower.starts_with(banned) {
            return Err(format!("response starts with meta commentary: {banned}"));
        }
    }
    let unknown = validate::unknown_numbers(content, allowed);
    if !unknown.is_empty() {
        return Err(format!(
            "numbers not present in FACTS or readme.yml: {}",
            unknown.join(", ")
        ));
    }
    Ok(())
}

fn cache_path(facts: &Facts, id: &str, lang: Lang, options: &Options) -> PathBuf {
    let mut key = Json::obj();
    key.push("prompt", PROMPT_VERSION);
    key.push("facts", facts.hash.clone());
    key.push("section", id);
    key.push("lang", lang.as_str());
    key.push("model", options.model.clone());
    key.push("endpoint", options.endpoint.clone());
    key.push("temperature", options.temperature.to_string());
    let digest = sha256::hex_digest(key.to_compact().as_bytes());
    options.cache_dir.join(format!("{digest}.md"))
}

fn write_cache(path: &PathBuf, content: &str) -> Result<()> {
    crate::util::write_text_atomic(path, content)
}

#[derive(Debug, Clone)]
struct Target {
    host: String,
    port: u16,
    path: String,
}

fn parse_endpoint(endpoint: &str) -> Result<Target> {
    let rest = endpoint
        .trim()
        .strip_prefix("http://")
        .ok_or_else(|| {
            Error::new(
                "only http:// endpoints are supported (run llama-server or Ollama locally)",
            )
        })?;
    let (authority, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, "/"),
    };
    if authority.is_empty() {
        return Err(Error::new(format!("invalid endpoint: {endpoint}")));
    }

    let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
        let end = rest
            .find(']')
            .ok_or_else(|| Error::new(format!("invalid IPv6 endpoint: {endpoint}")))?;
        let host = rest[..end].to_string();
        let port = rest[end + 1..]
            .strip_prefix(':')
            .and_then(|port| port.parse::<u16>().ok())
            .unwrap_or(80);
        (host, port)
    } else if let Some((host, port)) = authority.rsplit_once(':') {
        match port.parse::<u16>() {
            Ok(port) => (host.to_string(), port),
            Err(_) => (authority.to_string(), 80),
        }
    } else {
        (authority.to_string(), 80)
    };

    let mut path = path.trim_end_matches('/').to_string();
    if path.is_empty() {
        path = "/v1/chat/completions".to_string();
    } else if path.ends_with("/v1") {
        path.push_str("/chat/completions");
    } else if !path.ends_with("/chat/completions") {
        path.push_str("/v1/chat/completions");
    }

    Ok(Target { host, port, path })
}

fn chat(options: &Options, messages: &[(String, String)]) -> Result<String> {
    let target = parse_endpoint(&options.endpoint)?;

    let mut body = Json::obj();
    body.push("model", options.model.clone());
    let mut payload = Json::arr();
    for (role, content) in messages {
        let mut message = Json::obj();
        message.push("role", role.clone());
        message.push("content", content.clone());
        payload.push_value(message);
    }
    body.push("messages", payload);
    body.push("temperature", Json::Float(options.temperature));
    body.push("max_tokens", u64::from(options.max_tokens));
    body.push("stream", false);
    let body = body.to_compact();

    let mut request = format!(
        "POST {} HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nAccept: application/json\r\nConnection: close\r\nContent-Length: {}\r\n",
        target.path,
        target.host,
        target.port,
        body.as_bytes().len()
    );
    if let Some(key) = &options.api_key {
        request.push_str(&format!("Authorization: Bearer {key}\r\n"));
    }
    request.push_str("\r\n");
    request.push_str(&body);

    let mut stream = TcpStream::connect((target.host.as_str(), target.port))
        .map_err(|error| {
            Error::new(format!(
                "cannot connect to {}:{}: {error}. Start llama-server or Ollama first.",
                target.host, target.port
            ))
        })?;
    stream
        .set_read_timeout(Some(options.timeout))
        .map_err(|error| Error::new(format!("cannot set read timeout: {error}")))?;
    stream
        .set_write_timeout(Some(options.timeout))
        .map_err(|error| Error::new(format!("cannot set write timeout: {error}")))?;
    stream
        .write_all(request.as_bytes())
        .map_err(|error| Error::new(format!("cannot send request: {error}")))?;
    stream.flush().ok();

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| Error::new(format!("cannot read response: {error}")))?;
    if response.is_empty() {
        return Err(Error::new("empty response from model server"));
    }

    parse_http_response(&response)
}

fn parse_http_response(response: &[u8]) -> Result<String> {
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| (index, index + 4))
        .or_else(|| {
            response
                .windows(2)
                .position(|window| window == b"\n\n")
                .map(|index| (index, index + 2))
        })
        .ok_or_else(|| Error::new("malformed HTTP response: no header terminator"))?;

    let headers = String::from_utf8_lossy(&response[..split.0]);
    let mut lines = headers.lines();
    let status_line = lines.next().unwrap_or("");
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| Error::new(format!("malformed HTTP status line: {status_line}")))?;

    let mut chunked = false;
    let mut content_length: Option<usize> = None;
    for line in lines {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim().to_lowercase();
        let value = value.trim();
        if key == "transfer-encoding" && value.to_lowercase().contains("chunked") {
            chunked = true;
        }
        if key == "content-length" {
            content_length = value.parse::<usize>().ok();
        }
    }

    let raw_body = &response[split.1..];
    let body = if chunked {
        decode_chunked(raw_body)?
    } else if let Some(length) = content_length {
        if raw_body.len() >= length {
            raw_body[..length].to_vec()
        } else {
            raw_body.to_vec()
        }
    } else {
        raw_body.to_vec()
    };

    let text = String::from_utf8_lossy(&body).to_string();
    if !(200..300).contains(&status) {
        return Err(Error::new(format!(
            "model server returned HTTP {status}: {}",
            crate::util::truncate_chars(text.trim(), 400)
        )));
    }

    let value = json::parse(&text)
        .map_err(|error| Error::new(format!("invalid JSON from model server: {error}")))?;
    if let Some(error) = value.get("error") {
        let message = error
            .get("message")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown error");
        return Err(Error::new(format!("model server error: {message}")));
    }
    let choice = value
        .get("choices")
        .and_then(|value| value.as_array())
        .and_then(|choices| choices.first())
        .ok_or_else(|| Error::new("model response has no choices"))?;
    let content = choice
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(|value| value.as_str())
        .or_else(|| choice.get("text").and_then(|value| value.as_str()))
        .ok_or_else(|| Error::new("model response has no content"))?;
    Ok(content.to_string())
}

fn decode_chunked(mut body: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let Some(line_end) = body.windows(2).position(|window| window == b"\r\n") else {
            break;
        };
        let size_line = String::from_utf8_lossy(&body[..line_end]);
        let size_hex = size_line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_hex, 16)
            .map_err(|_| Error::new(format!("invalid chunk size: {size_hex}")))?;
        body = &body[line_end + 2..];
        if size == 0 {
            break;
        }
        if body.len() < size {
            return Err(Error::new("truncated chunked response"));
        }
        out.extend_from_slice(&body[..size]);
        body = &body[size..];
        if body.starts_with(b"\r\n") {
            body = &body[2..];
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_endpoint_paths() {
        let target = parse_endpoint("http://127.0.0.1:8080").unwrap();
        assert_eq!(target.path, "/v1/chat/completions");
        assert_eq!(target.port, 8080);

        let target = parse_endpoint("http://localhost:11434/v1").unwrap();
        assert_eq!(target.path, "/v1/chat/completions");

        let target = parse_endpoint("http://[::1]:8080/v1").unwrap();
        assert_eq!(target.host, "::1");
        assert_eq!(target.port, 8080);
    }

    #[test]
    fn rejects_https() {
        assert!(parse_endpoint("https://example.com").is_err());
    }

    #[test]
    fn decodes_chunked_bodies() {
        let body = b"4\r\nWiki\r\n5\r\npedia\r\n0\r\n\r\n";
        assert_eq!(decode_chunked(body).unwrap(), b"Wikipedia");
    }

    #[test]
    fn parses_openai_chat_response() {
        let body = br#"{"choices":[{"message":{"content":"hello"}}]}"#;
        let mut response = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: ".to_vec();
        response.extend_from_slice(body.len().to_string().as_bytes());
        response.extend_from_slice(b"\r\n\r\n");
        response.extend_from_slice(body);
        assert_eq!(parse_http_response(&response).unwrap(), "hello");
    }

    #[test]
    fn cleans_models_that_add_headings() {
        assert_eq!(clean("## Why\n\nText"), "Text");
        assert_eq!(clean("```markdown\nText\n```"), "Text");
    }
}
