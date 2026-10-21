//! Small utilities: text, sizes, paths, URLs and atomic writes.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Error, Result};

/// Read a UTF-8 text file and normalise CRLF/CR to LF.
pub fn read_text(path: &Path) -> Result<String> {
    let bytes = fs::read(path)
        .map_err(|e| Error::new(format!("cannot read {}: {e}", path.display())))?;
    let text = String::from_utf8(bytes)
        .map_err(|e| Error::new(format!("{} is not valid UTF-8: {e}", path.display())))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    Ok(normalize_newlines(text))
}

/// Read a text file, replacing invalid UTF-8 instead of failing.
pub fn read_text_lossy(path: &Path) -> Result<String> {
    let bytes = fs::read(path)
        .map_err(|e| Error::new(format!("cannot read {}: {e}", path.display())))?;
    let text = String::from_utf8_lossy(&bytes).to_string();
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    Ok(normalize_newlines(text))
}

pub fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Write UTF-8 text with LF line endings, creating parent directories.
///
/// The write is atomic on POSIX and best-effort atomic on Windows (where
/// `rename` over an existing file is retried after removing the destination).
pub fn write_text_atomic(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                Error::new(format!("cannot create {}: {e}", parent.display()))
            })?;
        }
    }

    let temp = temp_sibling(path);
    {
        let mut file = fs::File::create(&temp)
            .map_err(|e| Error::new(format!("cannot create {}: {e}", temp.display())))?;
        let body = text.trim_end_matches('\n');
        file.write_all(body.as_bytes())
            .map_err(|e| Error::new(format!("cannot write {}: {e}", temp.display())))?;
        file.write_all(b"\n")?;
        file.sync_all().ok();
    }

    match fs::rename(&temp, path) {
        Ok(()) => Ok(()),
        Err(first) => {
            // Windows refuses to replace an existing file with rename().
            if path.exists() {
                fs::remove_file(path).map_err(|e| {
                    Error::new(format!("cannot replace {}: {e}", path.display()))
                })?;
                fs::rename(&temp, path).map_err(|second| {
                    let _ = fs::remove_file(&temp);
                    Error::new(format!(
                        "cannot move {} to {}: {first}; retry: {second}",
                        temp.display(),
                        path.display()
                    ))
                })
            } else {
                let _ = fs::remove_file(&temp);
                Err(Error::new(format!(
                    "cannot move {} to {}: {first}",
                    temp.display(),
                    path.display()
                )))
            }
        }
    }
}

fn temp_sibling(path: &Path) -> PathBuf {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "readme".to_string());
    let temp_name = format!(".{name}.tmp.{pid}.{nanos}");
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(temp_name),
        _ => PathBuf::from(temp_name),
    }
}

/// Human readable binary size, e.g. `10.2 GiB`, `963 MiB`, `12 KiB`.
pub fn human_size(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    let b = bytes as f64;
    if b >= GIB {
        format!("{:.1} GiB", b / GIB)
    } else if b >= MIB {
        format!("{:.0} MiB", b / MIB)
    } else if b >= KIB {
        format!("{:.0} KiB", b / KIB)
    } else {
        format!("{bytes} B")
    }
}

/// Group digits with commas: 3087 -> "3,087".
pub fn format_count(value: u64) -> String {
    let raw = value.to_string();
    let mut out = String::with_capacity(raw.len() + raw.len() / 3);
    let bytes = raw.as_bytes();
    for (i, ch) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*ch as char);
    }
    out
}

/// Percent-encode a value for use in a shields.io badge path.
pub fn url_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        let ch = *byte as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '.' | '_' | '~') {
            out.push(ch);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Escape a small set of HTML characters for `<sub>`/`<strong>` blocks.
pub fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Convert a relative path to forward slashes for Markdown output.
pub fn to_slash(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.replace('\\', "/")
}

/// Normalise a relative path string: strip `./`, convert backslashes.
pub fn normalize_rel(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("./")
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string()
}

/// Truncate on a character boundary and append an ellipsis.
pub fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Trim trailing whitespace on every line (keeps the document tidy).
pub fn trim_trailing_spaces(text: &str) -> String {
    text.lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Collapse 3+ blank lines into exactly one blank line.
pub fn collapse_blank_lines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank_run = 0usize;
    for line in text.lines() {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run <= 1 {
                out.push('\n');
            }
            continue;
        }
        blank_run = 0;
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// A timestamp string for cache directories. Never written into the README.
pub fn unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// Best-effort check whether stdout is a terminal (used for progress output).
pub fn stdout_is_tty() -> bool {
    use std::io::IsTerminal;
    io::stdout().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_human_readable() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(1024), "1 KiB");
        assert_eq!(human_size(1024 * 1024), "1 MiB");
        assert_eq!(human_size(10_973_000_000), "10.2 GiB");
    }

    #[test]
    fn counts_are_grouped() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(3087), "3,087");
        assert_eq!(format_count(1_234_567), "1,234,567");
    }

    #[test]
    fn urls_are_encoded() {
        assert_eq!(url_encode("3,087"), "3%2C087");
        assert_eq!(url_encode("10.2 GiB"), "10.2%20GiB");
    }

    #[test]
    fn relative_paths_use_forward_slashes() {
        assert_eq!(normalize_rel("./a\\b/"), "a/b");
        assert_eq!(to_slash(Path::new("a/b")), "a/b");
    }
}
