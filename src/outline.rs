//! Section graph and presets.
//!
//! A preset decides *which* sections exist and in what order. Renderers decide
//! whether a section has enough facts to appear at all.

use crate::lang::Lang;
use crate::scan::ProjectKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Library,
    Cli,
    App,
    Dataset,
    Showcase,
}

impl Preset {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "library" | "lib" => Some(Preset::Library),
            "cli" | "command" => Some(Preset::Cli),
            "app" | "application" => Some(Preset::App),
            "dataset" | "data" => Some(Preset::Dataset),
            "showcase" | "media" => Some(Preset::Showcase),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Preset::Library => "library",
            Preset::Cli => "cli",
            Preset::App => "app",
            Preset::Dataset => "dataset",
            Preset::Showcase => "showcase",
        }
    }

    pub fn from_kind(kind: ProjectKind) -> Self {
        match kind {
            ProjectKind::Library => Preset::Library,
            ProjectKind::Cli => Preset::Cli,
            ProjectKind::App => Preset::App,
            ProjectKind::Dataset => Preset::Dataset,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind {
    Hero,
    Badges,
    Languages,
    Toc,
    Quickstart,
    Packs,
    Numbers,
    Catalog,
    Structure,
    Install,
    Usage,
    Deps,
    Sources,
    Compatibility,
    Awards,
    License,
    Contributing,
    Changelog,
    Cta,
    Prose(&'static str),
}

#[derive(Debug, Clone, Copy)]
pub struct SectionSpec {
    pub kind: SectionKind,
    pub id: &'static str,
}

impl SectionSpec {
    pub fn is_header(&self) -> bool {
        matches!(
            self.kind,
            SectionKind::Hero | SectionKind::Badges | SectionKind::Languages
        )
    }
}

pub fn sections(preset: Preset) -> Vec<SectionSpec> {
    use SectionKind::*;
    let specs: Vec<(&'static str, SectionKind)> = match preset {
        Preset::Library => vec![
            ("toc", Toc),
            ("numbers", Numbers),
            ("why", Prose("why")),
            ("install", Install),
            ("usage", Usage),
            ("structure", Structure),
            ("deps", Deps),
            ("contributing", Contributing),
            ("license", License),
        ],
        Preset::Cli => vec![
            ("toc", Toc),
            ("quickstart", Quickstart),
            ("numbers", Numbers),
            ("why", Prose("why")),
            ("install", Install),
            ("usage", Usage),
            ("structure", Structure),
            ("deps", Deps),
            ("contributing", Contributing),
            ("license", License),
        ],
        Preset::App => vec![
            ("toc", Toc),
            ("quickstart", Quickstart),
            ("numbers", Numbers),
            ("why", Prose("why")),
            ("install", Install),
            ("usage", Usage),
            ("structure", Structure),
            ("license", License),
        ],
        Preset::Dataset => vec![
            ("toc", Toc),
            ("quickstart", Quickstart),
            ("numbers", Numbers),
            ("catalog", Catalog),
            ("structure", Structure),
            ("sources", Sources),
            ("license", License),
            ("changelog", Changelog),
        ],
        Preset::Showcase => vec![
            ("toc", Toc),
            ("quickstart", Quickstart),
            ("packs", Packs),
            ("numbers", Numbers),
            ("why", Prose("why")),
            ("audience", Prose("audience")),
            ("catalog", Catalog),
            ("sources", Sources),
            ("compatibility", Compatibility),
            ("awards", Awards),
            ("license", License),
            ("changelog", Changelog),
            ("cta", Cta),
        ],
    };

    let mut out = vec![
        SectionSpec {
            kind: Hero,
            id: "hero",
        },
        SectionSpec {
            kind: Badges,
            id: "badges",
        },
        SectionSpec {
            kind: Languages,
            id: "languages",
        },
    ];
    out.extend(specs.into_iter().map(|(id, kind)| SectionSpec { kind, id }));
    out
}

pub fn title(kind: SectionKind, lang: Lang) -> &'static str {
    match (kind, lang) {
        (SectionKind::Toc, Lang::En) => "Contents",
        (SectionKind::Toc, Lang::Zh) => "目录",
        (SectionKind::Quickstart, Lang::En) => "Quick start",
        (SectionKind::Quickstart, Lang::Zh) => "快速开始",
        (SectionKind::Packs, Lang::En) => "Starter packs",
        (SectionKind::Packs, Lang::Zh) => "入门精选",
        (SectionKind::Numbers, Lang::En) => "By the numbers",
        (SectionKind::Numbers, Lang::Zh) => "数据一览",
        (SectionKind::Catalog, Lang::En) => "Catalog",
        (SectionKind::Catalog, Lang::Zh) => "目录一览",
        (SectionKind::Structure, Lang::En) => "Project structure",
        (SectionKind::Structure, Lang::Zh) => "项目结构",
        (SectionKind::Install, Lang::En) => "Installation",
        (SectionKind::Install, Lang::Zh) => "安装",
        (SectionKind::Usage, Lang::En) => "Usage",
        (SectionKind::Usage, Lang::Zh) => "使用",
        (SectionKind::Deps, Lang::En) => "Dependencies",
        (SectionKind::Deps, Lang::Zh) => "依赖",
        (SectionKind::Sources, Lang::En) => "Sources",
        (SectionKind::Sources, Lang::Zh) => "来源",
        (SectionKind::Compatibility, Lang::En) => "Formats & compatibility",
        (SectionKind::Compatibility, Lang::Zh) => "格式与兼容性",
        (SectionKind::Awards, Lang::En) => "Highlights",
        (SectionKind::Awards, Lang::Zh) => "亮点",
        (SectionKind::License, Lang::En) => "License",
        (SectionKind::License, Lang::Zh) => "许可证",
        (SectionKind::Contributing, Lang::En) => "Contributing",
        (SectionKind::Contributing, Lang::Zh) => "贡献",
        (SectionKind::Changelog, Lang::En) => "Update log",
        (SectionKind::Changelog, Lang::Zh) => "更新记录",
        (SectionKind::Cta, Lang::En) => "Star & fork",
        (SectionKind::Cta, Lang::Zh) => "Star 与 Fork",
        (SectionKind::Prose("why"), Lang::En) => "Why this project",
        (SectionKind::Prose("why"), Lang::Zh) => "为什么值得",
        (SectionKind::Prose("audience"), Lang::En) => "Who this is for",
        (SectionKind::Prose("audience"), Lang::Zh) => "适合谁",
        (SectionKind::Prose(_), Lang::En) => "Overview",
        (SectionKind::Prose(_), Lang::Zh) => "概述",
        _ => "",
    }
}

pub fn plan_json(preset: Preset) -> crate::json::Json {
    let mut root = crate::json::Json::obj();
    root.push("preset", preset.as_str());
    let mut items = crate::json::Json::arr();
    for spec in sections(preset) {
        let mut item = crate::json::Json::obj();
        item.push("id", spec.id);
        item.push(
            "kind",
            match spec.kind {
                SectionKind::Prose(name) => format!("prose:{name}"),
                other => format!("{other:?}").to_lowercase(),
            },
        );
        item.push(
            "title_en",
            title(spec.kind, Lang::En),
        );
        item.push(
            "title_zh",
            title(spec.kind, Lang::Zh),
        );
        items.push_value(item);
    }
    root.push("sections", items);
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_has_a_toc_and_license() {
        for preset in [
            Preset::Library,
            Preset::Cli,
            Preset::App,
            Preset::Dataset,
            Preset::Showcase,
        ] {
            let specs = sections(preset);
            assert!(specs.iter().any(|spec| spec.id == "toc"));
            assert!(specs.iter().any(|spec| spec.id == "license"));
        }
    }

    #[test]
    fn showcase_has_catalog_and_packs() {
        let specs = sections(Preset::Showcase);
        assert!(specs.iter().any(|spec| spec.id == "catalog"));
        assert!(specs.iter().any(|spec| spec.id == "packs"));
        assert!(specs.iter().any(|spec| spec.id == "cta"));
    }
}
