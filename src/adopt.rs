//! Adopt an existing hand-written README into `readme-gen` keep blocks.

use std::collections::{BTreeMap, HashSet};

use crate::lang::Lang;

#[derive(Debug)]
pub struct AdoptPlan {
    pub locales: Vec<Lang>,
    pub keeps: BTreeMap<(String, String), String>,
    pub sections: Vec<(Lang, String, String)>,
}

pub fn analyze(text: &str, default_lang: Lang) -> AdoptPlan {
    let mut locales = Vec::new();
    let mut keeps: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut sections: Vec<(Lang, String, String)> = Vec::new();
    let mut used_ids: HashSet<(String, String)> = HashSet::new();

    let mut current_lang = default_lang;
    let mut saw_bilingual_split = false;
    let mut current_heading: Option<(String, String)> = None;
    let mut current_body = String::new();
    let mut fence: Option<(char, usize)> = None;

    let flush = |lang: Lang,
                 heading: Option<(String, String)>,
                 body: &str,
                 keeps: &mut BTreeMap<(String, String), String>,
                 sections: &mut Vec<(Lang, String, String)>,
                 used: &mut HashSet<(String, String)>| {
        let Some((title, heading_line)) = heading else {
            return;
        };
        let cleaned_body = strip_existing_markers(body).trim().to_string();
        if cleaned_body.is_empty() {
            return;
        }
        let base_id = map_heading_to_id(&title);
        if base_id == "toc" {
            return;
        }
        let mut id = base_id.clone();
        let mut suffix = 2usize;
        while !used.insert((lang.as_str().to_string(), id.clone())) {
            id = format!("{base_id}-{suffix}");
            suffix += 1;
        }
        let full = format!("{heading_line}\n\n{cleaned_body}");
        keeps.insert((lang.as_str().to_string(), id.clone()), full);
        sections.push((lang, id, title));
    };

    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(first) = trimmed.chars().next().filter(|&c| c == '`' || c == '~') {
            let len = trimmed.chars().take_while(|&c| c == first).count();
            if len >= 3 {
                let rest = trimmed[len..].trim();
                match fence {
                    None => {
                        fence = Some((first, len));
                        if current_heading.is_some() {
                            current_body.push_str(line);
                            current_body.push('\n');
                        }
                        continue;
                    }
                    Some((open_c, open_len))
                        if open_c == first && len >= open_len && rest.is_empty() =>
                    {
                        fence = None;
                        if current_heading.is_some() {
                            current_body.push_str(line);
                            current_body.push('\n');
                        }
                        continue;
                    }
                    _ => {}
                }
            }
        }
        if fence.is_some() {
            if current_heading.is_some() {
                current_body.push_str(line);
                current_body.push('\n');
            }
            continue;
        }

        if let Some((level, heading_text)) = parse_heading(trimmed) {
            if level == 2 && is_english_heading(heading_text) {
                flush(
                    current_lang,
                    current_heading.take(),
                    &current_body,
                    &mut keeps,
                    &mut sections,
                    &mut used_ids,
                );
                current_body.clear();
                current_lang = Lang::En;
                saw_bilingual_split = true;
                if !locales.contains(&Lang::En) {
                    locales.push(Lang::En);
                }
                continue;
            }
            if level == 2 && is_chinese_heading(heading_text) {
                flush(
                    current_lang,
                    current_heading.take(),
                    &current_body,
                    &mut keeps,
                    &mut sections,
                    &mut used_ids,
                );
                current_body.clear();
                current_lang = Lang::Zh;
                saw_bilingual_split = true;
                if !locales.contains(&Lang::Zh) {
                    locales.push(Lang::Zh);
                }
                continue;
            }

            let split_level = if saw_bilingual_split { 3 } else { 2 };
            if level == split_level || (!saw_bilingual_split && level == 2) {
                flush(
                    current_lang,
                    current_heading.take(),
                    &current_body,
                    &mut keeps,
                    &mut sections,
                    &mut used_ids,
                );
                current_body.clear();
                current_heading = Some((heading_text.to_string(), trimmed.to_string()));
                if !locales.contains(&current_lang) {
                    locales.push(current_lang);
                }
                continue;
            }
        }

        if current_heading.is_some() {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }

    flush(
        current_lang,
        current_heading.take(),
        &current_body,
        &mut keeps,
        &mut sections,
        &mut used_ids,
    );

    if locales.is_empty() {
        locales.push(default_lang);
    }
    locales.sort();
    locales.dedup();

    AdoptPlan {
        locales,
        keeps,
        sections,
    }
}

fn parse_heading(trimmed: &str) -> Option<(usize, &str)> {
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|&c| c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &trimmed[level..];
    if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
        return None;
    }
    Some((level, rest.trim()))
}

fn is_english_heading(title: &str) -> bool {
    title.eq_ignore_ascii_case("english")
}

fn is_chinese_heading(title: &str) -> bool {
    title == "中文" || title.eq_ignore_ascii_case("chinese")
}

fn strip_existing_markers(body: &str) -> String {
    body.lines()
        .filter(|line| {
            let t = line.trim();
            !(t.starts_with("<!-- readme-gen:section:")
                || t.starts_with("<!-- readme-gen:keep:")
                || t.starts_with("<a id="))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn map_heading_to_id(heading: &str) -> String {
    let lower = heading.trim().to_lowercase();
    if lower == "contents" || lower == "table of contents" || lower == "目录" {
        return "toc".to_string();
    }
    if lower.contains("quick start")
        || lower.contains("quickstart")
        || lower.contains("getting started")
        || lower.contains("快速开始")
        || lower.contains("30 秒上手")
    {
        return "quickstart".to_string();
    }
    if lower.contains("starter pack") || lower.contains("入门精选") {
        return "packs".to_string();
    }
    if lower.contains("by the numbers") || lower.contains("数据一览") {
        return "numbers".to_string();
    }
    if lower.contains("why") || lower.contains("为什么") {
        return "why".to_string();
    }
    if lower.contains("who this is for") || lower.contains("适合谁") || lower.contains("audience") {
        return "audience".to_string();
    }
    if lower.contains("catalog") || lower.contains("目录一览") {
        return "catalog".to_string();
    }
    if lower.contains("structure") || lower.contains("项目结构") {
        return "structure".to_string();
    }
    if lower.contains("pick your setup") || lower.contains("使用建议") {
        return "setup".to_string();
    }
    if lower.contains("tooling") || lower.contains("工具") {
        return "tooling".to_string();
    }
    if lower.contains("install") || lower.contains("安装") {
        return "install".to_string();
    }
    if lower == "usage" || lower.starts_with("usage ") || lower == "使用" || lower == "使用方法" {
        return "usage".to_string();
    }
    if lower.contains("dependenc") || lower == "依赖" {
        return "deps".to_string();
    }
    if lower.contains("source") || lower == "来源" {
        return "sources".to_string();
    }
    if lower.contains("compat") || lower.contains("format") || lower.contains("格式") {
        return "compatibility".to_string();
    }
    if lower.contains("highlight") || lower.contains("award") || lower.contains("亮点") || lower.contains("奖杯") {
        return "awards".to_string();
    }
    if lower.contains("contribut") || lower.contains("贡献") {
        return "contributing".to_string();
    }
    if lower.contains("license") || lower.contains("licence") || lower.contains("许可证") {
        return "license".to_string();
    }
    if lower.contains("changelog") || lower.contains("update log") || lower.contains("更新记录") {
        return "changelog".to_string();
    }
    if lower.contains("star") || lower.contains("fork") {
        return "cta".to_string();
    }

    let mut slug = String::new();
    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
        } else if (ch == ' ' || ch == '-' || ch == '_') && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "custom".to_string()
    } else {
        slug.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adopts_monolingual_sections() {
        let doc = "# Demo\n\n## Installation\n\nRun cargo install.\n\n## Custom FAQ\n\nAnswer here.\n";
        let plan = analyze(doc, Lang::En);
        assert_eq!(plan.locales, vec![Lang::En]);
        assert!(plan.keeps.contains_key(&("en".to_string(), "install".to_string())));
        assert!(plan.keeps.contains_key(&("en".to_string(), "custom-faq".to_string())));
    }

    #[test]
    fn adopts_bilingual_sections() {
        let doc = "# Demo\n\n## English\n\n### Usage\n\nRun it.\n\n## 中文\n\n### 使用\n\n运行它。\n";
        let plan = analyze(doc, Lang::En);
        assert_eq!(plan.locales, vec![Lang::En, Lang::Zh]);
        assert!(plan.keeps.contains_key(&("en".to_string(), "usage".to_string())));
        assert!(plan.keeps.contains_key(&("zh".to_string(), "usage".to_string())));
    }
}
