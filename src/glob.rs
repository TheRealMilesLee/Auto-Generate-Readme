//! A small gitignore-style glob matcher.
//!
//! Supported: `*`, `?`, `**`, leading `/` (anchored), trailing `/` (directory
//! only), `!` negation, and patterns without `/` (matched against basenames).
//! Not supported: character classes (`[abc]`) and escaping. That is enough for
//! `.gitignore` files and the `exclude:` list used by the scanner.

use crate::util::normalize_rel;

#[derive(Debug, Clone)]
pub struct GlobSet {
    patterns: Vec<GlobPattern>,
}

#[derive(Debug, Clone)]
struct GlobPattern {
    pattern: String,
    negate: bool,
    dir_only: bool,
    anchored: bool,
    has_slash: bool,
}

impl GlobSet {
    pub fn new<I, S>(patterns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let patterns = patterns
            .into_iter()
            .filter_map(|raw| GlobPattern::parse(raw.as_ref()))
            .collect();
        Self { patterns }
    }

    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    /// Returns true when the path should be ignored.
    pub fn is_ignored(&self, rel: &str, is_dir: bool) -> bool {
        let rel = normalize_rel(rel);
        if rel.is_empty() {
            return false;
        }
        let mut ignored = false;
        for pattern in &self.patterns {
            if pattern.matches(&rel, is_dir) {
                ignored = !pattern.negate;
            }
        }
        ignored
    }
}

impl GlobPattern {
    fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        let mut pattern = trimmed.to_string();
        let mut negate = false;
        if let Some(rest) = pattern.strip_prefix('!') {
            negate = true;
            pattern = rest.trim().to_string();
        }
        if pattern.is_empty() {
            return None;
        }
        let dir_only = pattern.ends_with('/');
        if dir_only {
            pattern.pop();
        }
        let anchored = pattern.starts_with('/');
        if anchored {
            pattern.remove(0);
        }
        let has_slash = pattern.contains('/');
        if pattern.is_empty() {
            return None;
        }
        Some(Self {
            pattern,
            negate,
            dir_only,
            anchored,
            has_slash,
        })
    }

    fn matches(&self, rel: &str, is_dir: bool) -> bool {
        if self.dir_only && !is_dir {
            // A directory pattern also covers everything below that directory.
            let mut parts: Vec<&str> = rel.split('/').collect();
            while parts.len() > 1 {
                parts.pop();
                let ancestor = parts.join("/");
                if self.match_path(&ancestor) {
                    return true;
                }
            }
            return false;
        }
        self.match_path(rel)
    }

    fn match_path(&self, rel: &str) -> bool {
        if !self.has_slash && !self.anchored {
            let name = rel.rsplit('/').next().unwrap_or(rel);
            return wildcard_match(self.pattern.as_bytes(), name.as_bytes());
        }
        let pattern_parts: Vec<&str> = self.pattern.split('/').collect();
        let text_parts: Vec<&str> = rel.split('/').collect();
        path_match(&pattern_parts, &text_parts)
    }
}

fn path_match(pattern: &[&str], text: &[&str]) -> bool {
    match pattern.split_first() {
        None => text.is_empty(),
        Some((&"**", rest)) => {
            if path_match(rest, text) {
                return true;
            }
            if text.is_empty() {
                false
            } else {
                path_match(pattern, &text[1..])
            }
        }
        Some((segment, rest)) => {
            if text.is_empty() {
                return false;
            }
            wildcard_match(segment.as_bytes(), text[0].as_bytes()) && path_match(rest, &text[1..])
        }
    }
}

fn wildcard_match(pattern: &[u8], text: &[u8]) -> bool {
    match pattern.split_first() {
        None => text.is_empty(),
        Some((b'*', rest)) => {
            wildcard_match(rest, text) || (!text.is_empty() && wildcard_match(pattern, &text[1..]))
        }
        Some((b'?', rest)) => !text.is_empty() && wildcard_match(rest, &text[1..]),
        Some((byte, rest)) => {
            !text.is_empty() && text[0] == *byte && wildcard_match(rest, &text[1..])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(patterns: &[&str]) -> GlobSet {
        GlobSet::new(patterns.iter().map(|p| p.to_string()))
    }

    #[test]
    fn matches_basenames_at_any_depth() {
        let globs = set(&["*.py", "__pycache__/"]);
        assert!(globs.is_ignored("main.py", false));
        assert!(globs.is_ignored("src/pkg/mod.py", false));
        assert!(!globs.is_ignored("src/pkg/mod.rs", false));
        assert!(globs.is_ignored("src/__pycache__", true));
        assert!(globs.is_ignored("src/__pycache__/x.pyc", false));
    }

    #[test]
    fn matches_directory_patterns() {
        let globs = set(&["target/", "node_modules/"]);
        assert!(globs.is_ignored("target", true));
        assert!(globs.is_ignored("target/debug/app", false));
        assert!(globs.is_ignored("web/node_modules", true));
    }

    #[test]
    fn supports_double_star_and_negation() {
        let globs = set(&["**/target/", "!keep.py", "docs/**/*.md"]);
        assert!(globs.is_ignored("a/b/target", true));
        assert!(!globs.is_ignored("keep.py", false));
        assert!(globs.is_ignored("docs/guide/intro.md", false));
        assert!(!globs.is_ignored("src/intro.md", false));
    }

    #[test]
    fn negation_order_wins() {
        let globs = set(&["*.log", "!important.log"]);
        assert!(globs.is_ignored("debug.log", false));
        assert!(!globs.is_ignored("important.log", false));
    }
}
