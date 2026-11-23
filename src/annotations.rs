//! Human-authored annotations (`readme.yml`): the taste layer.
//!
//! Annotations contain meaning, never measured numbers. Numbers always come
//! from [`crate::scan::Facts`], so there is a single source of truth.

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{Error, Result};
use crate::json::Json;
use crate::lang::{Lang, LocaleMode, Loc};
use crate::outline::Preset;
use crate::sha256;
use crate::util::read_text;
use crate::yaml::{self, Yaml};

#[derive(Debug, Clone, Default)]
pub struct ProjectAnn {
    pub name: Option<String>,
    pub tagline: Loc,
    pub description: Loc,
    pub author: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Pack {
    pub name: Loc,
    pub dirs: Vec<String>,
    pub best_for: Loc,
}

#[derive(Debug, Clone, Default)]
pub struct QuickstartAnn {
    pub clone_url: Option<String>,
    pub packs: Vec<Pack>,
}

#[derive(Debug, Clone, Default)]
pub struct DirAnn {
    pub summary: Loc,
    pub bullets: Vec<Loc>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SourceAnn {
    pub name: String,
    pub note: Loc,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CompatAnn {
    pub name: String,
    pub note: Loc,
}

#[derive(Debug, Clone, Default)]
pub struct AwardAnn {
    pub award: Loc,
    pub winner: String,
}

#[derive(Debug, Clone, Default)]
pub struct ChangelogEntry {
    pub date: String,
    pub text: Loc,
}

#[derive(Debug, Clone, Default)]
pub struct CtaAnn {
    pub top: Option<bool>,
    pub star: Option<bool>,
    pub fork: Option<bool>,
    pub text: Loc,
}

#[derive(Debug, Clone, Default)]
pub struct Announcement {
    pub kind: String,
    pub date: Option<String>,
    pub text: Loc,
}

#[derive(Debug, Clone, Default)]
pub struct SetupEntry {
    pub target: Loc,
    pub recommend: Loc,
}

#[derive(Debug, Clone, Default)]
pub struct ToolEntry {
    pub command: String,
    pub note: Loc,
}

#[derive(Debug, Clone, Default)]
pub struct CatalogGroup {
    pub name: Loc,
    pub dirs: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct NavItem {
    pub label: Loc,
    pub href: String,
}

#[derive(Debug, Clone, Default)]
pub struct ExtraNumber {
    pub label: Loc,
    pub value: Loc,
}

#[derive(Debug, Clone, Default)]
pub struct LlmAnn {
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub no_thinking: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RulesAnn {
    pub max_dir_depth: Option<usize>,
    pub forbid_exts: Vec<String>,
    pub no_empty_dirs: bool,
    pub require_dir_annotations: bool,
}

#[derive(Debug, Clone, Default)]
pub struct UsageAnn {
    pub capture: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ScanAnn {
    pub media_meta: bool,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub preset: Option<Preset>,
    pub locales: Vec<Lang>,
    pub locale_mode: LocaleMode,
    pub project: ProjectAnn,
    pub nav: Vec<NavItem>,
    pub announcements: Vec<Announcement>,
    pub quickstart: QuickstartAnn,
    pub numbers_extra: Vec<ExtraNumber>,
    pub dirs: BTreeMap<String, DirAnn>,
    pub sections: BTreeMap<String, Loc>,
    pub setup: Vec<SetupEntry>,
    pub tooling: Vec<ToolEntry>,
    pub sources: Vec<SourceAnn>,
    pub compatibility: Vec<CompatAnn>,
    pub awards: Vec<AwardAnn>,
    pub changelog: Vec<ChangelogEntry>,
    pub cta: CtaAnn,
    pub catalog_order: Vec<String>,
    pub catalog_groups: Vec<CatalogGroup>,
    pub excludes: Vec<String>,
    pub badge_style: String,
    pub badge_links: bool,
    pub credit: bool,
    pub llm: LlmAnn,
    pub rules: RulesAnn,
    pub usage: UsageAnn,
    pub scan: ScanAnn,
    pub unknown_keys: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            preset: None,
            locales: Vec::new(),
            locale_mode: LocaleMode::Single,
            project: ProjectAnn::default(),
            nav: Vec::new(),
            announcements: Vec::new(),
            quickstart: QuickstartAnn::default(),
            numbers_extra: Vec::new(),
            dirs: BTreeMap::new(),
            sections: BTreeMap::new(),
            setup: Vec::new(),
            tooling: Vec::new(),
            sources: Vec::new(),
            compatibility: Vec::new(),
            awards: Vec::new(),
            changelog: Vec::new(),
            cta: CtaAnn::default(),
            catalog_order: Vec::new(),
            catalog_groups: Vec::new(),
            excludes: Vec::new(),
            badge_style: "flat".to_string(),
            badge_links: true,
            credit: true,
            llm: LlmAnn::default(),
            rules: RulesAnn::default(),
            usage: UsageAnn::default(),
            scan: ScanAnn::default(),
            unknown_keys: Vec::new(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text = read_text(path)?;
        let value = yaml::parse(&text).map_err(|error| error.context(path.display().to_string()))?;
        Self::from_yaml(&value)
    }

    pub fn from_yaml(value: &Yaml) -> Result<Self> {
        let mut config = Config::default();

        if let Some(preset) = value.get("preset").and_then(|value| value.as_str()) {
            config.preset = Some(Preset::parse(preset).ok_or_else(|| {
                Error::new(format!(
                    "unknown preset '{preset}' (library|cli|app|dataset|showcase)"
                ))
            })?);
        }

        if let Some(locales) = value.get("locale") {
            config.locales = parse_locales(locales)?;
        }

        if let Some(project) = value.get("project") {
            config.project = ProjectAnn {
                name: project.get("name").and_then(|value| value.as_str()).map(str::to_string),
                tagline: project
                    .get("tagline")
                    .map(Loc::from_yaml)
                    .unwrap_or_default(),
                description: project
                    .get("description")
                    .map(Loc::from_yaml)
                    .unwrap_or_default(),
                author: project.get("author").and_then(|value| value.as_str()).map(str::to_string),
                license: project.get("license").and_then(|value| value.as_str()).map(str::to_string),
            };
        }

        if let Some(quickstart) = value.get("quickstart") {
            let mut ann = QuickstartAnn::default();
            ann.clone_url = quickstart
                .get("clone")
                .or_else(|| quickstart.get("clone_url"))
                .and_then(|value| value.as_str())
                .map(str::to_string);
            for item in quickstart
                .get("packs")
                .map(|value| value.as_list())
                .unwrap_or(&[])
            {
                ann.packs.push(Pack {
                    name: item.get("name").map(Loc::from_yaml).unwrap_or_default(),
                    dirs: item
                        .get("dirs")
                        .map(|value| {
                            value
                                .as_list()
                                .iter()
                                .filter_map(|dir| dir.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default(),
                    best_for: item
                        .get("best_for")
                        .map(Loc::from_yaml)
                        .unwrap_or_default(),
                });
            }
            config.quickstart = ann;
        }

        if let Some(dirs) = value.get("dirs") {
            for (name, item) in dirs.as_map() {
                let mut ann = DirAnn {
                    summary: item
                        .get("summary")
                        .map(Loc::from_yaml)
                        .unwrap_or_default(),
                    bullets: Vec::new(),
                    tags: item
                        .get("tags")
                        .map(|value| {
                            value
                                .as_list()
                                .iter()
                                .filter_map(|tag| tag.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default(),
                };
                if let Some(bullets) = item.get("bullets") {
                    for bullet in bullets.as_list() {
                        ann.bullets.push(Loc::from_yaml(bullet));
                    }
                }
                config.dirs.insert(name.clone(), ann);
            }
        }

        if let Some(sections) = value.get("sections") {
            for (name, item) in sections.as_map() {
                config.sections.insert(name.clone(), Loc::from_yaml(item));
            }
        }

        if let Some(sources) = value.get("sources") {
            for item in sources.as_list() {
                let name = item
                    .get("name")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .to_string();
                if name.is_empty() {
                    continue;
                }
                config.sources.push(SourceAnn {
                    name,
                    note: item.get("note").map(Loc::from_yaml).unwrap_or_default(),
                    url: item.get("url").and_then(|value| value.as_str()).map(str::to_string),
                });
            }
        }

        if let Some(items) = value.get("compatibility") {
            for item in items.as_list() {
                let name = item
                    .get("name")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .to_string();
                if !name.is_empty() {
                    config.compatibility.push(CompatAnn {
                        name,
                        note: item
                            .get("note")
                            .map(Loc::from_yaml)
                            .unwrap_or_default(),
                    });
                }
            }
        }

        if let Some(items) = value.get("awards") {
            for item in items.as_list() {
                let winner = item
                    .get("winner")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .to_string();
                if !winner.is_empty() {
                    config.awards.push(AwardAnn {
                        award: item
                            .get("award")
                            .map(Loc::from_yaml)
                            .unwrap_or_default(),
                        winner,
                    });
                }
            }
        }

        if let Some(items) = value.get("changelog") {
            for item in items.as_list() {
                let date = item
                    .get("date")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .to_string();
                let text = item
                    .get("text")
                    .map(Loc::from_yaml)
                    .unwrap_or_default();
                if !date.is_empty() && !text.is_empty() {
                    config.changelog.push(ChangelogEntry { date, text });
                }
            }
        }

        if let Some(cta) = value.get("cta") {
            config.cta = CtaAnn {
                top: cta.get("top").and_then(parse_bool),
                star: cta.get("star").and_then(parse_bool),
                fork: cta.get("fork").and_then(parse_bool),
                text: cta.get("text").map(Loc::from_yaml).unwrap_or_default(),
            };
        }

        if let Some(items) = value.get("nav") {
            for item in items.as_list() {
                let href = item
                    .get("href")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let label = item.get("label").map(Loc::from_yaml).unwrap_or_default();
                if !href.is_empty() && !label.is_empty() {
                    config.nav.push(NavItem { label, href });
                }
            }
        }

        if let Some(items) = value.get("announcements") {
            for item in items.as_list() {
                let kind = item
                    .get("kind")
                    .or_else(|| item.get("type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("note")
                    .trim()
                    .to_uppercase();
                let date = item
                    .get("date")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                let text = item.get("text").map(Loc::from_yaml).unwrap_or_default();
                if !text.is_empty() {
                    config.announcements.push(Announcement { kind, date, text });
                }
            }
        }

        if let Some(numbers) = value.get("numbers") {
            for item in numbers.get("extra").map(|v| v.as_list()).unwrap_or(&[]) {
                let label = item.get("label").map(Loc::from_yaml).unwrap_or_default();
                let val = item.get("value").map(Loc::from_yaml).unwrap_or_default();
                if !label.is_empty() && !val.is_empty() {
                    config.numbers_extra.push(ExtraNumber { label, value: val });
                }
            }
        }

        if let Some(items) = value.get("setup") {
            for item in items.as_list() {
                let target = item.get("target").map(Loc::from_yaml).unwrap_or_default();
                let recommend = item
                    .get("recommend")
                    .or_else(|| item.get("note"))
                    .map(Loc::from_yaml)
                    .unwrap_or_default();
                if !target.is_empty() && !recommend.is_empty() {
                    config.setup.push(SetupEntry { target, recommend });
                }
            }
        }

        if let Some(items) = value.get("tooling") {
            for item in items.as_list() {
                let command = item
                    .get("command")
                    .or_else(|| item.get("script"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let note = item.get("note").map(Loc::from_yaml).unwrap_or_default();
                if !command.is_empty() {
                    config.tooling.push(ToolEntry { command, note });
                }
            }
        }

        if let Some(catalog) = value.get("catalog") {
            config.catalog_order = catalog
                .get("order")
                .map(|value| {
                    value
                        .as_list()
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            for item in catalog
                .get("groups")
                .map(|v| v.as_list())
                .unwrap_or(&[])
            {
                let name = item.get("name").map(Loc::from_yaml).unwrap_or_default();
                let dirs: Vec<String> = item
                    .get("dirs")
                    .map(|v| {
                        v.as_list()
                            .iter()
                            .filter_map(|d| d.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                if !name.is_empty() && !dirs.is_empty() {
                    config.catalog_groups.push(CatalogGroup { name, dirs });
                }
            }
        }

        if let Some(excludes) = value.get("exclude").or_else(|| value.get("excludes")) {
            config.excludes = excludes
                .as_list()
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect();
        }

        if let Some(badges) = value.get("badges") {
            if let Some(style) = badges.get("style").and_then(|value| value.as_str()) {
                config.badge_style = style.to_string();
            }
            if let Some(links) = badges.get("links").and_then(parse_bool) {
                config.badge_links = links;
            }
        }

        if let Some(credit) = value.get("credit").and_then(parse_bool) {
            config.credit = credit;
        }

        const KNOWN_TOP_KEYS: &[&str] = &[
            "preset",
            "locale",
            "locale_mode",
            "project",
            "nav",
            "announcements",
            "quickstart",
            "numbers",
            "dirs",
            "sections",
            "setup",
            "tooling",
            "sources",
            "compatibility",
            "awards",
            "changelog",
            "cta",
            "catalog",
            "exclude",
            "excludes",
            "badges",
            "credit",
            "llm",
            "rules",
            "usage",
            "scan",
        ];
        for (key, _) in value.as_map() {
            if !KNOWN_TOP_KEYS.contains(&key.as_str()) {
                config.unknown_keys.push(key.clone());
            }
        }

        if let Some(mode) = value.get("locale_mode").and_then(|v| v.as_str()) {
            config.locale_mode = LocaleMode::parse(mode).ok_or_else(|| {
                Error::new(format!(
                    "unknown locale_mode '{mode}' (single|split)"
                ))
            })?;
        }

        if let Some(llm) = value.get("llm") {
            config.llm = LlmAnn {
                endpoint: llm
                    .get("endpoint")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                model: llm.get("model").and_then(|value| value.as_str()).map(str::to_string),
                temperature: llm
                    .get("temperature")
                    .and_then(|value| value.as_str())
                    .and_then(|value| value.parse::<f64>().ok()),
                no_thinking: llm.get("no_thinking").and_then(parse_bool).unwrap_or(false),
            };
        }

        if let Some(rules) = value.get("rules") {
            config.rules = RulesAnn {
                max_dir_depth: rules
                    .get("max_dir_depth")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.trim().parse::<usize>().ok()),
                forbid_exts: rules
                    .get("forbid_exts")
                    .map(|v| {
                        v.as_list()
                            .iter()
                            .filter_map(|e| e.as_str().map(|s| s.trim().trim_start_matches('.').to_lowercase()))
                            .filter(|s| !s.is_empty())
                            .collect()
                    })
                    .unwrap_or_default(),
                no_empty_dirs: rules.get("no_empty_dirs").and_then(parse_bool).unwrap_or(false),
                require_dir_annotations: rules
                    .get("require_dir_annotations")
                    .and_then(parse_bool)
                    .unwrap_or(false),
            };
        }

        if let Some(usage) = value.get("usage") {
            config.usage = UsageAnn {
                capture: usage
                    .get("capture")
                    .or_else(|| usage.get("capture_help"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()),
            };
        }

        if let Some(scan) = value.get("scan") {
            config.scan = ScanAnn {
                media_meta: scan.get("media_meta").and_then(parse_bool).unwrap_or(false),
            };
        }

        Ok(config)
    }
}

fn parse_bool(value: &Yaml) -> Option<bool> {
    let raw = value.as_str()?.trim();
    if raw.eq_ignore_ascii_case("false") || raw.eq_ignore_ascii_case("no") {
        Some(false)
    } else if raw.eq_ignore_ascii_case("true") || raw.eq_ignore_ascii_case("yes") {
        Some(true)
    } else {
        None
    }
}

fn parse_locales(value: &Yaml) -> Result<Vec<Lang>> {
    let mut out = Vec::new();
    match value {
        Yaml::Str(text) => {
            for part in text.split(',') {
                if let Some(lang) = Lang::parse(part) {
                    out.push(lang);
                }
            }
        }
        Yaml::List(items) => {
            for item in items {
                if let Some(text) = item.as_str() {
                    if let Some(lang) = Lang::parse(text) {
                        out.push(lang);
                    }
                }
            }
        }
        _ => {}
    }
    if out.is_empty() {
        return Err(Error::new(
            "locale must be a list such as [en, zh]",
        ));
    }
    out.sort();
    out.dedup();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
preset: showcase
locale: [en, zh]
project:
  name: The-Wallpaper-Collection
  tagline:
    en: Eight years of desktop taste
    zh: 八年策展
quickstart:
  clone: https://github.com/MilesCorporate/The-Wallpaper-Collection.git
  packs:
    - name:
        en: New machine
        zh: 新机器
      dirs: [2026 Wallpaper, 2025 Wallpaper]
      best_for:
        en: Fresh 4K setup
        zh: 全新 4K 桌面
dirs:
  "2026 Wallpaper":
    summary:
      en: Active growth lane
      zh: 当前增长的一年
    bullets:
      - en: 272 images
        zh: 272 张
    tags: [f1, 4k]
sections:
  why: |
    因为值得。
sources:
  - name: Unsplash
    note:
      en: Photographer-attributed work
      zh: 摄影师署名作品
    url: https://unsplash.com/license
"#;

    #[test]
    fn parses_annotations() {
        let value = yaml::parse(SAMPLE).unwrap();
        let config = Config::from_yaml(&value).unwrap();
        assert_eq!(config.preset, Some(Preset::Showcase));
        assert_eq!(config.locales, vec![Lang::En, Lang::Zh]);
        assert_eq!(
            config.project.tagline.get(Lang::Zh),
            Some("八年策展")
        );
        assert_eq!(config.quickstart.packs.len(), 1);
        assert_eq!(
            config.quickstart.packs[0].name.get(Lang::En),
            Some("New machine")
        );
        assert_eq!(config.dirs["2026 Wallpaper"].tags, vec!["f1", "4k"]);
        assert_eq!(
            config.sections["why"].get(Lang::En),
            Some("因为值得。")
        );
        assert_eq!(config.sources[0].name, "Unsplash");
    }

    #[test]
    fn rejects_unknown_preset() {
        let value = yaml::parse("preset: nope\n").unwrap();
        assert!(Config::from_yaml(&value).is_err());
    }
}

impl Config {
    /// All human-authored strings, used by the number-tracing checker. Any
    /// number a human wrote in `readme.yml` is considered approved.
    pub fn text_corpus(&self) -> String {
        fn push_loc(out: &mut String, loc: &Loc) {
            if let Some(value) = &loc.en {
                out.push_str(value);
                out.push('\n');
            }
            if let Some(value) = &loc.zh {
                out.push_str(value);
                out.push('\n');
            }
        }

        let mut out = String::new();
        if let Some(name) = &self.project.name {
            out.push_str(name);
            out.push('\n');
        }
        push_loc(&mut out, &self.project.tagline);
        push_loc(&mut out, &self.project.description);
        if let Some(author) = &self.project.author {
            out.push_str(author);
            out.push('\n');
        }
        if let Some(license) = &self.project.license {
            out.push_str(license);
            out.push('\n');
        }
        if let Some(clone) = &self.quickstart.clone_url {
            out.push_str(clone);
            out.push('\n');
        }
        for pack in &self.quickstart.packs {
            push_loc(&mut out, &pack.name);
            push_loc(&mut out, &pack.best_for);
            for dir in &pack.dirs {
                out.push_str(dir);
                out.push('\n');
            }
        }
        for dir in self.dirs.values() {
            push_loc(&mut out, &dir.summary);
            for bullet in &dir.bullets {
                push_loc(&mut out, bullet);
            }
            for tag in &dir.tags {
                out.push_str(tag);
                out.push('\n');
            }
        }
        for section in self.sections.values() {
            push_loc(&mut out, section);
        }
        for source in &self.sources {
            out.push_str(&source.name);
            out.push('\n');
            push_loc(&mut out, &source.note);
            if let Some(url) = &source.url {
                out.push_str(url);
                out.push('\n');
            }
        }
        for item in &self.compatibility {
            out.push_str(&item.name);
            out.push('\n');
            push_loc(&mut out, &item.note);
        }
        for item in &self.awards {
            push_loc(&mut out, &item.award);
            out.push_str(&item.winner);
            out.push('\n');
        }
        for entry in &self.changelog {
            out.push_str(&entry.date);
            out.push('\n');
            push_loc(&mut out, &entry.text);
        }
        for item in &self.nav {
            push_loc(&mut out, &item.label);
            out.push_str(&item.href);
            out.push('\n');
        }
        for item in &self.announcements {
            if let Some(date) = &item.date {
                out.push_str(date);
                out.push('\n');
            }
            push_loc(&mut out, &item.text);
        }
        for item in &self.numbers_extra {
            push_loc(&mut out, &item.label);
            push_loc(&mut out, &item.value);
        }
        for item in &self.setup {
            push_loc(&mut out, &item.target);
            push_loc(&mut out, &item.recommend);
        }
        for item in &self.tooling {
            out.push_str(&item.command);
            out.push('\n');
            push_loc(&mut out, &item.note);
        }
        for group in &self.catalog_groups {
            push_loc(&mut out, &group.name);
        }
        push_loc(&mut out, &self.cta.text);
        out
    }

    /// Deterministic hash of configuration/annotations for cache invalidation and drift check.
    pub fn content_hash(&self) -> String {
        fn loc_json(loc: &Loc) -> Json {
            let mut obj = Json::obj();
            obj.push("en", loc.en.clone());
            obj.push("zh", loc.zh.clone());
            obj
        }

        let mut root = Json::obj();
        root.push("preset", self.preset.map(|p| p.as_str().to_string()));
        let mut locales = Json::arr();
        for lang in &self.locales {
            locales.push_value(lang.as_str());
        }
        root.push("locales", locales);
        root.push("locale_mode", self.locale_mode.as_str());

        let mut project = Json::obj();
        project.push("name", self.project.name.clone());
        project.push("tagline", loc_json(&self.project.tagline));
        project.push("description", loc_json(&self.project.description));
        project.push("author", self.project.author.clone());
        project.push("license", self.project.license.clone());
        root.push("project", project);

        let mut nav = Json::arr();
        for item in &self.nav {
            let mut n = Json::obj();
            n.push("label", loc_json(&item.label));
            n.push("href", item.href.clone());
            nav.push_value(n);
        }
        root.push("nav", nav);

        let mut announcements = Json::arr();
        for item in &self.announcements {
            let mut a = Json::obj();
            a.push("kind", item.kind.clone());
            a.push("date", item.date.clone());
            a.push("text", loc_json(&item.text));
            announcements.push_value(a);
        }
        root.push("announcements", announcements);

        let mut quickstart = Json::obj();
        quickstart.push("clone", self.quickstart.clone_url.clone());
        let mut packs = Json::arr();
        for p in &self.quickstart.packs {
            let mut pj = Json::obj();
            pj.push("name", loc_json(&p.name));
            let mut dirs = Json::arr();
            for d in &p.dirs {
                dirs.push_value(d.clone());
            }
            pj.push("dirs", dirs);
            pj.push("best_for", loc_json(&p.best_for));
            packs.push_value(pj);
        }
        quickstart.push("packs", packs);
        root.push("quickstart", quickstart);

        let mut numbers_extra = Json::arr();
        for item in &self.numbers_extra {
            let mut n = Json::obj();
            n.push("label", loc_json(&item.label));
            n.push("value", loc_json(&item.value));
            numbers_extra.push_value(n);
        }
        root.push("numbers_extra", numbers_extra);

        let mut dirs = Json::arr();
        for (name, dir) in &self.dirs {
            let mut d = Json::obj();
            d.push("name", name.clone());
            d.push("summary", loc_json(&dir.summary));
            let mut bullets = Json::arr();
            for b in &dir.bullets {
                bullets.push_value(loc_json(b));
            }
            d.push("bullets", bullets);
            let mut tags = Json::arr();
            for t in &dir.tags {
                tags.push_value(t.clone());
            }
            d.push("tags", tags);
            dirs.push_value(d);
        }
        root.push("dirs", dirs);

        let mut sections = Json::arr();
        for (name, loc) in &self.sections {
            let mut s = Json::obj();
            s.push("name", name.clone());
            s.push("text", loc_json(loc));
            sections.push_value(s);
        }
        root.push("sections", sections);

        let mut setup = Json::arr();
        for item in &self.setup {
            let mut s = Json::obj();
            s.push("target", loc_json(&item.target));
            s.push("recommend", loc_json(&item.recommend));
            setup.push_value(s);
        }
        root.push("setup", setup);

        let mut tooling = Json::arr();
        for item in &self.tooling {
            let mut t = Json::obj();
            t.push("command", item.command.clone());
            t.push("note", loc_json(&item.note));
            tooling.push_value(t);
        }
        root.push("tooling", tooling);

        let mut sources = Json::arr();
        for item in &self.sources {
            let mut s = Json::obj();
            s.push("name", item.name.clone());
            s.push("url", item.url.clone());
            s.push("note", loc_json(&item.note));
            sources.push_value(s);
        }
        root.push("sources", sources);

        let mut compat = Json::arr();
        for item in &self.compatibility {
            let mut c = Json::obj();
            c.push("name", item.name.clone());
            c.push("note", loc_json(&item.note));
            compat.push_value(c);
        }
        root.push("compatibility", compat);

        let mut awards = Json::arr();
        for item in &self.awards {
            let mut a = Json::obj();
            a.push("award", loc_json(&item.award));
            a.push("winner", item.winner.clone());
            awards.push_value(a);
        }
        root.push("awards", awards);

        let mut changelog = Json::arr();
        for item in &self.changelog {
            let mut c = Json::obj();
            c.push("date", item.date.clone());
            c.push("text", loc_json(&item.text));
            changelog.push_value(c);
        }
        root.push("changelog", changelog);

        let mut cta = Json::obj();
        cta.push("top", self.cta.top);
        cta.push("star", self.cta.star);
        cta.push("fork", self.cta.fork);
        cta.push("text", loc_json(&self.cta.text));
        root.push("cta", cta);

        let mut order = Json::arr();
        for item in &self.catalog_order {
            order.push_value(item.clone());
        }
        root.push("catalog_order", order);

        let mut groups = Json::arr();
        for group in &self.catalog_groups {
            let mut g = Json::obj();
            g.push("name", loc_json(&group.name));
            let mut gd = Json::arr();
            for d in &group.dirs {
                gd.push_value(d.clone());
            }
            g.push("dirs", gd);
            groups.push_value(g);
        }
        root.push("catalog_groups", groups);

        let mut excludes = Json::arr();
        for item in &self.excludes {
            excludes.push_value(item.clone());
        }
        root.push("excludes", excludes);
        root.push("badge_style", self.badge_style.clone());
        root.push("badge_links", self.badge_links);
        root.push("credit", self.credit);
        root.push("usage_capture", self.usage.capture.clone());

        sha256::hex_digest(root.to_compact().as_bytes())
    }
}
