//! Human-authored annotations (`readme.yml`): the taste layer.
//!
//! Annotations contain meaning, never measured numbers. Numbers always come
//! from [`crate::scan::Facts`], so there is a single source of truth.

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{Error, Result};
use crate::lang::{Lang, Loc};
use crate::outline::Preset;
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
    pub star: Option<bool>,
    pub fork: Option<bool>,
    pub text: Loc,
}

#[derive(Debug, Clone, Default)]
pub struct LlmAnn {
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub preset: Option<Preset>,
    pub locales: Vec<Lang>,
    pub project: ProjectAnn,
    pub quickstart: QuickstartAnn,
    pub dirs: BTreeMap<String, DirAnn>,
    pub sections: BTreeMap<String, Loc>,
    pub sources: Vec<SourceAnn>,
    pub compatibility: Vec<CompatAnn>,
    pub awards: Vec<AwardAnn>,
    pub changelog: Vec<ChangelogEntry>,
    pub cta: CtaAnn,
    pub catalog_order: Vec<String>,
    pub excludes: Vec<String>,
    pub badge_style: String,
    pub credit: bool,
    pub llm: LlmAnn,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            preset: None,
            locales: Vec::new(),
            project: ProjectAnn::default(),
            quickstart: QuickstartAnn::default(),
            dirs: BTreeMap::new(),
            sections: BTreeMap::new(),
            sources: Vec::new(),
            compatibility: Vec::new(),
            awards: Vec::new(),
            changelog: Vec::new(),
            cta: CtaAnn::default(),
            catalog_order: Vec::new(),
            excludes: Vec::new(),
            badge_style: "flat".to_string(),
            credit: true,
            llm: LlmAnn::default(),
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
                star: cta.get("star").and_then(|value| value.as_str()).map(|value| value != "false"),
                fork: cta.get("fork").and_then(|value| value.as_str()).map(|value| value != "false"),
                text: cta.get("text").map(Loc::from_yaml).unwrap_or_default(),
            };
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
        }

        if let Some(credit) = value.get("credit").and_then(|value| value.as_str()) {
            config.credit = credit != "false";
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
            };
        }

        Ok(config)
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
        push_loc(&mut out, &self.cta.text);
        out
    }
}
