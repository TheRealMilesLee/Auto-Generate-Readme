//! Post-generation validation. This is what replaces the old `grep -v` filters:
//! instead of deleting model output heuristically, we reject documents that
//! contain unresolved placeholders, broken links, unknown numbers or drift.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::annotations::Config;
use crate::json::Json;
use crate::lang::Lang;
use crate::render::extract_meta;
use crate::scan::Facts;

#[derive(Debug, Default)]
pub struct Report {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl Report {
    pub fn error(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    pub fn warning(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }

    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn print(&self) {
        for warning in &self.warnings {
            eprintln!("warning: {warning}");
        }
        for error in &self.errors {
            eprintln!("error: {error}");
        }
        if self.is_ok() {
            println!(
                "check passed ({} warning(s))",
                self.warnings.len()
            );
        } else {
            println!(
                "check failed: {} error(s), {} warning(s)",
                self.errors.len(),
                self.warnings.len()
            );
        }
    }
}

pub fn validate(
    text: &str,
    facts: &Facts,
    config: &Config,
    locales: &[Lang],
    readme_path: &Path,
) -> Report {
    let mut report = Report::default();
    if text.trim().is_empty() {
        report.error("README is empty");
        return report;
    }

    check_meta(text, facts, config, locales, &mut report);
    check_placeholders(text, &mut report);
    check_fences(text, &mut report);
    check_links(text, readme_path, &mut report);
    check_anchors(text, &mut report);
    check_markers(text, locales, &mut report);
    check_keeps(text, &mut report);
    check_numbers(text, facts, config, &mut report);
    check_headings(text, &mut report);
    check_rules(facts, config, &mut report);
    check_config_hygiene(facts, config, &mut report);
    report
}

pub fn validate_split(
    files: &[(Lang, &str, &Path)],
    facts: &Facts,
    config: &Config,
) -> Report {
    let mut combined = Report::default();
    let mut section_sets: Vec<(Lang, HashSet<String>)> = Vec::new();
    let mut checked_hygiene = false;

    for (lang, text, path) in files {
        let mut r = Report::default();
        if text.trim().is_empty() {
            r.error(format!("{} is empty", path.display()));
        } else {
            check_meta(text, facts, config, &[*lang], &mut r);
            check_placeholders(text, &mut r);
            check_fences(text, &mut r);
            check_links(text, path, &mut r);
            check_anchors(text, &mut r);
            let ids = collect_section_ids(text, &mut r);
            section_sets.push((*lang, ids));
            check_keeps(text, &mut r);
            check_numbers(text, facts, config, &mut r);
            check_headings(text, &mut r);
        }
        let prefix = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        for err in r.errors {
            combined.error(format!("{prefix}: {err}"));
        }
        for warn in r.warnings {
            combined.warning(format!("{prefix}: {warn}"));
        }
        if !checked_hygiene {
            check_rules(facts, config, &mut combined);
            check_config_hygiene(facts, config, &mut combined);
            checked_hygiene = true;
        }
    }

    if section_sets.len() > 1 {
        let (base_lang, base_ids) = &section_sets[0];
        for (lang, ids) in section_sets.iter().skip(1) {
            for id in base_ids.difference(ids) {
                combined.error(format!(
                    "bilingual mismatch: section '{id}' is in {} but missing from {}",
                    base_lang.as_str(),
                    lang.as_str()
                ));
            }
            for id in ids.difference(base_ids) {
                combined.error(format!(
                    "bilingual mismatch: section '{id}' is in {} but missing from {}",
                    lang.as_str(),
                    base_lang.as_str()
                ));
            }
        }
    }

    combined
}

pub fn check_rules(facts: &Facts, config: &Config, report: &mut Report) {
    if let Some(max_depth) = config.rules.max_dir_depth {
        for dir in &facts.dirs {
            if dir.max_depth > max_depth {
                report.error(format!(
                    "rule violation: directory '{}' has depth {} (max_dir_depth = {})",
                    dir.path, dir.max_depth, max_depth
                ));
            }
        }
    }
    for ext in &config.rules.forbid_exts {
        if let Some(&count) = facts.exts.get(ext) {
            if count > 0 {
                report.error(format!(
                    "rule violation: forbidden extension '.{ext}' found ({count} file(s))"
                ));
            }
        }
    }
    if config.rules.no_empty_dirs {
        for dir in &facts.empty_dirs {
            report.error(format!(
                "rule violation: empty directory '{}' (no_empty_dirs = true)",
                dir
            ));
        }
    }
    if config.rules.require_dir_annotations {
        for dir in &facts.dirs {
            let has_summary = config
                .dirs
                .get(&dir.path)
                .is_some_and(|ann| !ann.summary.is_empty());
            if !has_summary {
                report.warning(format!(
                    "missing directory annotation in readme.yml for '{}'",
                    dir.path
                ));
            }
        }
    }
}

pub fn check_config_hygiene(facts: &Facts, config: &Config, report: &mut Report) {
    for key in &config.unknown_keys {
        report.warning(format!("readme.yml: unknown top-level key '{key}'"));
    }
    let existing: HashSet<&str> = facts.dirs.iter().map(|d| d.path.as_str()).collect();
    if existing.is_empty() && config.dirs.is_empty() {
        return;
    }
    for dir_name in config.dirs.keys() {
        if !existing.contains(dir_name.as_str()) {
            report.warning(format!(
                "readme.yml: dirs entry '{dir_name}' does not match any scanned top-level folder"
            ));
        }
    }
    for dir_name in &config.catalog_order {
        if !existing.contains(dir_name.as_str()) {
            report.warning(format!(
                "readme.yml: catalog.order entry '{dir_name}' does not match any scanned top-level folder"
            ));
        }
    }
    for pack in &config.quickstart.packs {
        for dir_name in &pack.dirs {
            if !existing.contains(dir_name.as_str()) {
                report.warning(format!(
                    "readme.yml: quickstart.packs references missing folder '{dir_name}'"
                ));
            }
        }
    }
    for group in &config.catalog_groups {
        for dir_name in &group.dirs {
            if !existing.contains(dir_name.as_str()) {
                report.warning(format!(
                    "readme.yml: catalog.groups references missing folder '{dir_name}'"
                ));
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FenceState {
    marker: char,
    len: usize,
}

fn parse_fence_line(line: &str) -> Option<(char, usize, bool)> {
    let trimmed = line.trim_start();
    let first = trimmed.chars().next()?;
    if first != '`' && first != '~' {
        return None;
    }
    let len = trimmed.chars().take_while(|&c| c == first).count();
    if len < 3 {
        return None;
    }
    let rest = trimmed[len..].trim();
    Some((first, len, rest.is_empty()))
}

fn step_fence(state: &mut Option<FenceState>, line: &str) -> bool {
    let Some((marker, len, is_plain_close)) = parse_fence_line(line) else {
        return false;
    };
    match *state {
        None => {
            *state = Some(FenceState { marker, len });
            true
        }
        Some(open) if open.marker == marker && len >= open.len && is_plain_close => {
            *state = None;
            true
        }
        Some(_) => false,
    }
}

fn check_meta(
    text: &str,
    facts: &Facts,
    config: &Config,
    locales: &[Lang],
    report: &mut Report,
) {
    let Some(meta) = extract_meta(text) else {
        report.error(
            "missing `<!-- readme-gen:meta ... -->` header; regenerate with `readme-gen build`",
        );
        return;
    };

    match meta.get("facts").and_then(|v| v.as_str()) {
        None => report.error(
            "missing `facts` hash in `<!-- readme-gen:meta ... -->`; regenerate with `readme-gen build`",
        ),
        Some(hash) if hash != facts.hash => report.error(format!(
            "README is stale: embedded facts hash {hash} does not match current {}. Facts changed since the last build.",
            facts.hash
        )),
        Some(_) => {}
    }

    if let Some(cfg_hash) = meta.get("config").and_then(|v| v.as_str()) {
        let current = config.content_hash();
        if cfg_hash != current {
            report.error(format!(
                "README is stale: embedded config hash {cfg_hash} does not match current {current}. `readme.yml` changed since the last build."
            ));
        }
    }

    if let Some(expected_preset) = config.preset {
        if let Some(actual_preset) = meta.get("preset").and_then(|v| v.as_str()) {
            if actual_preset != expected_preset.as_str() {
                report.error(format!(
                    "README preset mismatch: document uses '{actual_preset}' but readme.yml specifies '{}'",
                    expected_preset.as_str()
                ));
            }
        }
    }

    if let Some(meta_locales) = meta.get("locales").and_then(|v| v.as_array()) {
        let actual: Vec<String> = meta_locales
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        let expected: Vec<String> = locales.iter().map(|l| l.as_str().to_string()).collect();
        if !actual.is_empty() && actual != expected {
            report.error(format!(
                "README locale mismatch: document has [{}] but expected [{}]",
                actual.join(", "),
                expected.join(", ")
            ));
        }
    }

    if let Some(actual_mode) = meta.get("locale_mode").and_then(|v| v.as_str()) {
        let expected_mode = config.locale_mode.as_str();
        if actual_mode != expected_mode {
            report.error(format!(
                "README locale_mode mismatch: document uses '{actual_mode}' but readme.yml specifies '{expected_mode}'"
            ));
        }
    }
}

fn check_placeholders(text: &str, report: &mut Report) {
    let mut fence = None;
    for (index, line) in text.lines().enumerate() {
        if step_fence(&mut fence, line) || fence.is_some() {
            continue;
        }
        let cleaned = strip_html_comments(line);
        if cleaned.contains("{{") || cleaned.contains("}}") {
            report.error(format!(
                "line {}: unresolved placeholder: {}",
                index + 1,
                line.trim()
            ));
        }
        let lower = cleaned.to_lowercase();
        if lower.contains("lorem ipsum") {
            report.warning(format!("line {}: Lorem ipsum text", index + 1));
        }
    }
}

fn check_fences(text: &str, report: &mut Report) {
    let mut fence: Option<(usize, FenceState)> = None;
    for (index, line) in text.lines().enumerate() {
        let Some((marker, len, is_plain_close)) = parse_fence_line(line) else {
            continue;
        };
        match fence {
            None => fence = Some((index + 1, FenceState { marker, len })),
            Some((_, open)) if open.marker == marker && len >= open.len && is_plain_close => {
                fence = None;
            }
            Some(_) => {}
        }
    }
    if let Some((line, _)) = fence {
        report.error(format!("line {line}: unclosed code fence"));
    }
}

fn check_links(text: &str, path: &Path, report: &mut Report) {
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let mut fence = None;
    for (index, line) in text.lines().enumerate() {
        if step_fence(&mut fence, line) || fence.is_some() {
            continue;
        }
        let cleaned = strip_html_comments(line);
        for target in link_targets(&cleaned) {
            let target = target.trim().trim_matches(|c| c == '<' || c == '>');
            if target.is_empty()
                || target.starts_with('#')
                || target.starts_with("http://")
                || target.starts_with("https://")
                || target.starts_with("mailto:")
                || target.starts_with("data:")
                || target.starts_with("//")
            {
                continue;
            }
            let without_fragment = target.split('#').next().unwrap_or(target);
            let without_query = without_fragment.split('?').next().unwrap_or(without_fragment);
            let decoded = percent_decode(without_query);
            let resolved = base.join(&decoded);
            if !resolved.exists() {
                report.error(format!(
                    "line {}: broken link '{}' (resolved to {})",
                    index + 1,
                    target,
                    resolved.display()
                ));
            }
        }
    }
}

fn check_anchors(text: &str, report: &mut Report) {
    let mut anchors: HashMap<String, usize> = HashMap::new();
    let mut heading_slugs: HashSet<String> = HashSet::new();
    let mut toc: Vec<(String, usize)> = Vec::new();
    let mut fence = None;

    for (index, line) in text.lines().enumerate() {
        if step_fence(&mut fence, line) || fence.is_some() {
            continue;
        }
        for id in anchor_ids(line) {
            *anchors.entry(id).or_insert(0) += 1;
        }
        if let Some((_, heading_text)) = parse_heading_line(line) {
            let slug = github_slug(heading_text);
            if !slug.is_empty() {
                heading_slugs.insert(slug);
            }
        }
        for target in link_targets(line) {
            if let Some(fragment) = target.trim().strip_prefix('#') {
                toc.push((fragment.to_string(), index + 1));
            }
        }
    }

    for (id, count) in &anchors {
        if *count > 1 {
            report.error(format!("duplicate anchor id '{id}' ({count} times)"));
        }
    }
    for (target, line) in toc {
        if !anchors.contains_key(&target) && !heading_slugs.contains(&target) {
            report.warning(format!(
                "line {line}: anchor target '#{target}' is not defined"
            ));
        }
    }
}

fn collect_section_ids(text: &str, report: &mut Report) -> HashSet<String> {
    let mut open: HashMap<(String, String), usize> = HashMap::new();
    let mut all_ids: HashSet<String> = HashSet::new();
    let mut fence = None;

    for (index, line) in text.lines().enumerate() {
        if step_fence(&mut fence, line) || fence.is_some() {
            continue;
        }
        let Some(inner) = line
            .trim()
            .strip_prefix("<!-- readme-gen:section:")
            .and_then(|rest| rest.strip_suffix("-->"))
        else {
            continue;
        };
        let parts: Vec<&str> = inner.trim().split(':').collect();
        if parts.len() != 3 {
            report.warning(format!("line {}: malformed section marker", index + 1));
            continue;
        }
        let lang = parts[0].trim().to_string();
        let id = parts[1].trim().to_string();
        match parts[2].trim() {
            "start" => {
                if open.insert((lang.clone(), id.clone()), index + 1).is_some() {
                    report.error(format!(
                        "line {}: duplicate start marker for {lang}:{id}",
                        index + 1
                    ));
                }
            }
            "end" => {
                if open.remove(&(lang, id.clone())).is_none() {
                    report.error(format!(
                        "line {}: end marker without start for {id}",
                        index + 1
                    ));
                }
                all_ids.insert(id);
            }
            _ => report.warning(format!("line {}: malformed section marker", index + 1)),
        }
    }

    for ((lang, id), line) in open {
        report.error(format!(
            "line {line}: unclosed section marker for {lang}:{id}"
        ));
    }
    all_ids
}

fn check_markers(text: &str, locales: &[Lang], report: &mut Report) {
    let mut open: HashMap<(String, String), usize> = HashMap::new();
    let mut ids: HashMap<String, HashSet<String>> = HashMap::new();
    let mut fence = None;

    for (index, line) in text.lines().enumerate() {
        if step_fence(&mut fence, line) || fence.is_some() {
            continue;
        }
        let Some(inner) = line
            .trim()
            .strip_prefix("<!-- readme-gen:section:")
            .and_then(|rest| rest.strip_suffix("-->"))
        else {
            continue;
        };
        let parts: Vec<&str> = inner.trim().split(':').collect();
        if parts.len() != 3 {
            report.warning(format!("line {}: malformed section marker", index + 1));
            continue;
        }
        let lang = parts[0].trim().to_string();
        let id = parts[1].trim().to_string();
        match parts[2].trim() {
            "start" => {
                if open.insert((lang.clone(), id.clone()), index + 1).is_some() {
                    report.error(format!(
                        "line {}: duplicate start marker for {lang}:{id}",
                        index + 1
                    ));
                }
            }
            "end" => {
                if open.remove(&(lang.clone(), id.clone())).is_none() {
                    report.error(format!(
                        "line {}: end marker without start for {lang}:{id}",
                        index + 1
                    ));
                }
                ids.entry(lang).or_default().insert(id);
            }
            _ => report.warning(format!("line {}: malformed section marker", index + 1)),
        }
    }

    for ((lang, id), line) in open {
        report.error(format!(
            "line {line}: unclosed section marker for {lang}:{id}"
        ));
    }

    if locales.len() > 1 {
        let expected = ids
            .get(locales[0].as_str())
            .cloned()
            .unwrap_or_default();
        for lang in locales.iter().skip(1) {
            let actual = ids.get(lang.as_str()).cloned().unwrap_or_default();
            for id in expected.difference(&actual) {
                report.error(format!(
                    "bilingual mismatch: section '{id}' is missing from {}",
                    lang.as_str()
                ));
            }
            for id in actual.difference(&expected) {
                report.error(format!(
                    "bilingual mismatch: section '{id}' only exists in {}",
                    lang.as_str()
                ));
            }
        }
    }
}

fn check_keeps(text: &str, report: &mut Report) {
    let mut open: Option<(String, usize)> = None;
    let mut fence = None;
    for (index, line) in text.lines().enumerate() {
        if step_fence(&mut fence, line) || fence.is_some() {
            continue;
        }
        let trimmed = line.trim();
        if let Some(inner) = trimmed
            .strip_prefix("<!-- readme-gen:keep:")
            .and_then(|rest| rest.strip_suffix("-->"))
        {
            let parts: Vec<&str> = inner.trim().split(':').collect();
            if parts.len() != 3 {
                report.warning(format!("line {}: malformed keep marker", index + 1));
                continue;
            }
            let key = format!("{}:{}", parts[0].trim(), parts[1].trim());
            match parts[2].trim() {
                "start" => {
                    if open.is_some() {
                        report.warning(format!(
                            "line {}: nested keep block",
                            index + 1
                        ));
                    }
                    open = Some((key, index + 1));
                }
                "end" => {
                    match open.take() {
                        Some((open_key, _)) if open_key == key => {}
                        Some((open_key, _)) => report.error(format!(
                            "line {}: keep end '{key}' does not match open '{open_key}'",
                            index + 1
                        )),
                        None => report.error(format!(
                            "line {}: keep end without start: {key}",
                            index + 1
                        )),
                    }
                }
                _ => {}
            }
        }
    }
    if let Some((key, line)) = open {
        report.error(format!("line {line}: unclosed keep block {key}"));
    }
}

pub fn allowed_numbers(facts: &Facts, config: &Config) -> HashSet<String> {
    let mut allowed: HashSet<String> = HashSet::new();
    collect_json_numbers(&facts.to_json(), &mut allowed);
    for token in numeric_tokens(&config.text_corpus()) {
        allowed.insert(token);
    }
    // The project's own version is a legitimate number.
    if let Some(version) = &facts.version {
        for token in numeric_tokens(version) {
            allowed.insert(token);
        }
    }
    allowed
}

/// Numbers in `text` that are neither approved nor obvious shapes.
pub fn unknown_numbers(text: &str, allowed: &HashSet<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for token in numeric_tokens(text) {
        if allowed.contains(&token) || shape_allowed(&token) {
            continue;
        }
        if seen.insert(token.clone()) {
            out.push(token);
        }
    }
    out
}

fn check_numbers(text: &str, facts: &Facts, config: &Config, report: &mut Report) {
    let allowed = allowed_numbers(facts, config);

    let mut fence = None;
    let mut seen: HashSet<String> = HashSet::new();
    for (index, line) in text.lines().enumerate() {
        if step_fence(&mut fence, line) || fence.is_some() {
            continue;
        }
        let cleaned = strip_for_number_check(line);
        for token in numeric_tokens(&cleaned) {
            if allowed.contains(&token) || shape_allowed(&token) {
                continue;
            }
            if seen.insert(token.clone()) {
                report.error(format!(
                    "line {}: number '{}' is not traceable to facts or readme.yml",
                    index + 1,
                    token
                ));
            }
        }
    }
}

fn parse_heading_line(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|ch| *ch == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &trimmed[level..];
    if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
        return None;
    }
    Some((level, rest.trim()))
}

fn github_slug(heading: &str) -> String {
    let mut out = String::new();
    for ch in heading.to_lowercase().chars() {
        if ch.is_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else if ch == ' ' {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn check_headings(text: &str, report: &mut Report) {
    let mut previous = 0usize;
    let mut fence = None;
    for (index, line) in text.lines().enumerate() {
        if step_fence(&mut fence, line) || fence.is_some() {
            continue;
        }
        let Some((level, _)) = parse_heading_line(line) else {
            continue;
        };
        if previous == 0 && level != 1 {
            report.warning(format!("line {}: document should start with an H1", index + 1));
        } else if previous > 0 && level > previous + 1 {
            report.warning(format!(
                "line {}: heading level jumps from H{previous} to H{level}",
                index + 1
            ));
        }
        previous = level;
    }
}

fn collect_json_numbers(value: &Json, out: &mut HashSet<String>) {
    match value {
        Json::Str(text) => {
            for token in numeric_tokens(text) {
                out.insert(token);
            }
        }
        Json::Int(value) => {
            out.insert(value.to_string());
        }
        Json::UInt(value) => {
            out.insert(value.to_string());
        }
        Json::Float(value) => {
            out.insert(value.to_string());
        }
        Json::Arr(items) => {
            for item in items {
                collect_json_numbers(item, out);
            }
        }
        Json::Obj(entries) => {
            for (key, item) in entries {
                collect_json_numbers(&Json::Str(key.clone()), out);
                collect_json_numbers(item, out);
            }
        }
        _ => {}
    }
}

/// Numbers that are legitimate by shape: years, semantic versions, resolutions
/// and display-class suffixes (4K, 1080p).
fn shape_allowed(token: &str) -> bool {
    if token.len() == 4 && token.chars().all(|ch| ch.is_ascii_digit()) {
        if let Ok(year) = token.parse::<u32>() {
            if (1900..=2100).contains(&year) {
                return true;
            }
        }
    }
    if token.matches('.').count() >= 2
        && token
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
    {
        return true; // semver-ish
    }
    for separator in ['x', 'X', '×'] {
        if let Some((left, right)) = token.split_once(separator) {
            if !left.is_empty()
                && !right.is_empty()
                && left.chars().all(|ch| ch.is_ascii_digit())
                && right.chars().all(|ch| ch.is_ascii_digit())
            {
                return true; // 3440x1440, 3840×2160
            }
        }
    }
    if let Some((left, right)) = token.split_once(':') {
        if !left.is_empty()
            && !right.is_empty()
            && left.chars().all(|ch| ch.is_ascii_digit())
            && right.chars().all(|ch| ch.is_ascii_digit())
        {
            return true; // 21:9
        }
    }
    if let Some(suffix) = token.chars().next_back() {
        let prefix = &token[..token.len() - suffix.len_utf8()];
        if matches!(suffix, 'k' | 'K' | 'p' | 'P')
            && !prefix.is_empty()
            && prefix.chars().all(|ch| ch.is_ascii_digit())
        {
            return true; // 4K, 1080p
        }
    }
    false
}

/// Extract numeric tokens, preserving dates/versions/resolutions as one token.
pub fn numeric_tokens(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if !chars[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let start = index;

        // Date: YYYY-MM-DD or YYYY/MM/DD
        if index + 10 <= chars.len()
            && chars[index..index + 4].iter().all(|ch| ch.is_ascii_digit())
            && matches!(chars[index + 4], '-' | '/')
            && chars[index + 5..index + 7].iter().all(|ch| ch.is_ascii_digit())
            && matches!(chars[index + 7], '-' | '/')
            && chars[index + 8..index + 10].iter().all(|ch| ch.is_ascii_digit())
        {
            let token: String = chars[index..index + 10].iter().collect();
            out.push(token);
            index += 10;
            continue;
        }

        // Resolution: 3440x1440 / 21:9
        let mut cursor = index;
        while cursor < chars.len() && chars[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor < chars.len()
            && matches!(chars[cursor], 'x' | 'X' | '×' | ':')
            && cursor + 1 < chars.len()
            && chars[cursor + 1].is_ascii_digit()
        {
            let separator = chars[cursor];
            let mut end = cursor + 1;
            while end < chars.len() && chars[end].is_ascii_digit() {
                end += 1;
            }
            let token: String = chars[index..end].iter().collect();
            out.push(token);
            index = end;
            let _ = separator;
            continue;
        }

        // Version: 1.2.3 (one or more dots)
        let mut end = index;
        let mut dots = 0;
        loop {
            while end < chars.len() && chars[end].is_ascii_digit() {
                end += 1;
            }
            if end < chars.len() && chars[end] == '.' && end + 1 < chars.len() && chars[end + 1].is_ascii_digit() {
                dots += 1;
                end += 1;
                continue;
            }
            break;
        }
        if dots >= 1 {
            let token: String = chars[index..end].iter().collect();
            out.push(token);
            index = end;
            continue;
        }

        // Suffix: 4K / 1080p
        if end < chars.len()
            && matches!(chars[end], 'k' | 'K' | 'p' | 'P')
            && (end + 1 >= chars.len() || !chars[end + 1].is_alphabetic())
        {
            let token: String = chars[index..end + 1].iter().collect();
            out.push(token);
            index = end + 1;
            continue;
        }

        // Plain integer with optional thousands separators.
        let mut end = index;
        while end < chars.len() && (chars[end].is_ascii_digit() || chars[end] == ',') {
            end += 1;
        }
        let token: String = chars[index..end]
            .iter()
            .filter(|ch| **ch != ',')
            .collect();
        if !token.is_empty() {
            out.push(token);
        }
        index = end.max(start + 1);
    }
    out
}

fn link_targets(line: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let bytes = line.as_bytes();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index] == b']' && bytes[index + 1] == b'(' {
            let start = index + 2;
            let mut end = start;
            let mut depth = 1usize;
            while end < bytes.len() && depth > 0 {
                if bytes[end] == b'(' {
                    depth += 1;
                } else if bytes[end] == b')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                end += 1;
            }
            if end <= bytes.len() {
                let raw = &line[start..end.min(bytes.len())];
                let target = raw.split_whitespace().next().unwrap_or("");
                if !target.is_empty() {
                    targets.push(target.trim_matches(|c| c == '<' || c == '>').to_string());
                }
            }
            index = end.saturating_add(1);
            continue;
        }
        index += 1;
    }
    for attribute in ["src=\"", "href=\""] {
        let mut offset = 0;
        while let Some(found) = line[offset..].find(attribute) {
            let start = offset + found + attribute.len();
            if let Some(end) = line[start..].find('"') {
                targets.push(line[start..start + end].to_string());
                offset = start + end + 1;
            } else {
                break;
            }
        }
    }
    targets
}

fn anchor_ids(line: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for prefix in ["<a id=\"", "<a name=\""] {
        let mut offset = 0;
        while let Some(found) = line[offset..].find(prefix) {
            let start = offset + found + prefix.len();
            if let Some(end) = line[start..].find('"') {
                ids.push(line[start..start + end].to_string());
                offset = start + end + 1;
            } else {
                break;
            }
        }
    }
    ids
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn strip_html_comments(line: &str) -> String {
    let mut cleaned = String::new();
    let mut rest = line;
    loop {
        if let Some(start) = rest.find("<!--") {
            cleaned.push_str(&rest[..start]);
            if let Some(end) = rest[start..].find("-->") {
                rest = &rest[start + end + 3..];
                continue;
            }
            break;
        }
        cleaned.push_str(rest);
        break;
    }
    cleaned
}

fn strip_inline_code(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let mut ticks = 0;
            while i + ticks < bytes.len() && bytes[i + ticks] == b'`' {
                ticks += 1;
            }
            let marker = &text[i..i + ticks];
            if let Some(close) = text[i + ticks..].find(marker) {
                out.push(' ');
                i += ticks + close + ticks;
                continue;
            }
            i += ticks;
            continue;
        }
        let ch = text[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn strip_ordered_list_prefix(line: &str) -> &str {
    let trimmed = line.trim_start();
    let digits = trimmed.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digits > 0 {
        let rest = &trimmed[digits..];
        if let Some(after_dot) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(".\t")) {
            return after_dot;
        }
    }
    line
}

fn strip_for_number_check(line: &str) -> String {
    let line = strip_ordered_list_prefix(line);
    // Remove HTML comments (metadata, section markers).
    let cleaned = strip_html_comments(line);
    // Remove inline code spans (single or multiple backticks).
    let without_code = strip_inline_code(&cleaned);

    // Remove link destinations (handling balanced parentheses) and HTML attribute values.
    let mut out = String::new();
    let mut chars = without_code.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == ']' && chars.peek() == Some(&'(') {
            chars.next();
            let mut depth = 1usize;
            for next in chars.by_ref() {
                if next == '(' {
                    depth += 1;
                } else if next == ')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
            out.push(' ');
            continue;
        }
        out.push(ch);
    }

    let mut final_out = String::new();
    let mut rest = out.as_str();
    while let Some(position) = rest.find('=') {
        let after_eq = rest[position + 1..].trim_start();
        if let Some(quote) = after_eq.chars().next().filter(|&c| c == '"' || c == '\'') {
            final_out.push_str(&rest[..position]);
            let inner = &after_eq[1..];
            if let Some(end) = inner.find(quote) {
                rest = &inner[end + 1..];
            } else {
                rest = "";
                break;
            }
        } else {
            final_out.push_str(&rest[..position + 1]);
            rest = &rest[position + 1..];
        }
    }
    final_out.push_str(rest);
    final_out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_numbers() {
        assert_eq!(numeric_tokens("3,087 images"), vec!["3087"]);
        assert_eq!(numeric_tokens("10.2 GiB"), vec!["10.2"]);
        assert_eq!(numeric_tokens("2026-10-08"), vec!["2026-10-08"]);
        assert_eq!(numeric_tokens("3440x1440"), vec!["3440x1440"]);
        assert_eq!(numeric_tokens("v1.2.3"), vec!["1.2.3"]);
        assert_eq!(numeric_tokens("4K and 1080p"), vec!["4K", "1080p"]);
    }

    #[test]
    fn allowed_shapes() {
        assert!(shape_allowed("2026"));
        assert!(shape_allowed("1.2.3"));
        assert!(shape_allowed("3440x1440"));
        assert!(shape_allowed("3840×2160"));
        assert!(shape_allowed("1920X1080"));
        assert!(shape_allowed("21:9"));
        assert!(shape_allowed("4K"));
        assert!(!shape_allowed("42"));
        assert!(!shape_allowed("10.2"));
    }

    #[test]
    fn strips_code_and_urls_before_number_check() {
        let line = "See `sha256` and ``v99`` and [x](https://en.wikipedia.org/wiki/Foo_(2024)) plus <img src='a/99.png'>";
        assert_eq!(numeric_tokens(&strip_for_number_check(line)), Vec::<String>::new());
        assert_eq!(
            numeric_tokens(&strip_for_number_check("1. First step")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn fences_and_headings_handle_nesting_and_hashtags() {
        let mut report = Report::default();
        let doc = "# Title\n\n#hashtag is not a heading\n\n````markdown\n```bash\n{{PLACEHOLDER}}\n```\n````\n";
        check_fences(doc, &mut report);
        check_placeholders(doc, &mut report);
        check_headings(doc, &mut report);
        assert!(report.is_ok(), "{:?}", report.errors);
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    }

    #[test]
    fn extracts_links_and_anchors() {
        let line = r#"[a](docs/x.md) ![b](img/b.png) <a id="x"></a> <a href="docs/y.md">y</a>"#;
        let targets = link_targets(line);
        assert!(targets.contains(&"docs/x.md".to_string()));
        assert!(targets.contains(&"img/b.png".to_string()));
        assert!(targets.contains(&"docs/y.md".to_string()));
        assert_eq!(anchor_ids(line), vec!["x"]);
    }
}
