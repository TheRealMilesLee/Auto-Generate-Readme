//! Deterministic section renderers and document assembly.
//!
//! Every number written here comes from [`Facts`]. Prose comes from
//! `readme.yml` (or an LLM result passed in by the CLI, already validated).
//! Renderers return `None` when there is not enough factual material, so a
//! section disappears instead of being filled with placeholder text.

use std::collections::{BTreeMap, HashMap};

use crate::annotations::Config;
use crate::lang::Lang;
use crate::outline::{self, Preset, SectionKind, SectionSpec};
use crate::scan::{Facts, ProjectKind};
use crate::util::{escape_html, format_count, url_encode};

pub const GENERATOR_URL: &str = "https://github.com/TheRealMilesLee/Auto-Generate-Readme";

pub struct Ctx<'a> {
    pub facts: &'a Facts,
    pub config: &'a Config,
    pub generated: &'a BTreeMap<(String, Lang), String>,
    pub keeps: &'a BTreeMap<(String, String), String>,
    pub locales: &'a [Lang],
    pub preset: Preset,
    pub version: &'a str,
}

struct Rendered {
    id: &'static str,
    title: &'static str,
    body: String,
}

pub fn build(ctx: &Ctx<'_>) -> String {
    let mut out = String::new();
    out.push_str(&meta_line(ctx));
    out.push('\n');
    out.push_str(&hero(ctx));
    out.push('\n');

    let badges = badges(ctx);
    if !badges.is_empty() {
        out.push_str(&badges);
        out.push('\n');
    }

    if ctx.locales.len() > 1 {
        out.push_str(&language_nav(ctx));
        out.push('\n');
    }

    for (index, lang) in ctx.locales.iter().enumerate() {
        if ctx.locales.len() > 1 {
            if index > 0 {
                out.push_str("---\n\n");
            }
            let (anchor, heading) = match lang {
                Lang::En => ("english", "English"),
                Lang::Zh => ("chinese", "中文"),
            };
            out.push_str(&format!("<a id=\"{anchor}\"></a>\n## {heading}\n\n"));
        }
        out.push_str(&render_language(ctx, *lang));
        out.push('\n');
    }

    out.push_str(&footer(ctx));
    crate::util::collapse_blank_lines(&out).trim().to_string() + "\n"
}

fn meta_line(ctx: &Ctx<'_>) -> String {
    let mut meta = crate::json::Json::obj();
    meta.push("schema", 1u64);
    meta.push("facts", ctx.facts.hash.clone());
    meta.push("preset", ctx.preset.as_str());
    let mut locales = crate::json::Json::arr();
    for lang in ctx.locales {
        locales.push_value(lang.as_str());
    }
    meta.push("locales", locales);
    meta.push("generator", format!("readme-gen/{}", ctx.version));
    format!("<!-- readme-gen:meta {} -->", meta.to_compact())
}

fn hero(ctx: &Ctx<'_>) -> String {
    let name = project_name(ctx);
    let primary = ctx.locales.first().copied().unwrap_or(Lang::En);
    let mut tagline = ctx
        .config
        .project
        .tagline
        .get(primary)
        .map(str::to_string)
        .or_else(|| {
            ctx.config
                .project
                .description
                .get(primary)
                .map(str::to_string)
        })
        .unwrap_or_else(|| ctx.facts.description.clone());
    if tagline.trim().is_empty() {
        tagline = match ctx.facts.kind {
            ProjectKind::Dataset => t(primary, "A curated media collection.", "一份精选媒体收藏。").to_string(),
            _ => String::new(),
        };
    }

    let mut out = format!("# {name}\n\n");
    out.push_str("<p align=\"center\">\n");
    if !tagline.is_empty() {
        out.push_str(&format!("  <strong>{}</strong><br>\n", escape_html(&tagline)));
    }
    if ctx.locales.len() > 1 {
        if let Some(secondary) = ctx.locales.get(1).and_then(|lang| {
            ctx.config
                .project
                .tagline
                .get(*lang)
                .map(str::to_string)
        }) {
            out.push_str(&format!("  <sub>{}</sub><br>\n", escape_html(&secondary)));
        }
    }
    out.push_str(&format!("  <sub>{}</sub>\n", escape_html(&stats_line(ctx, primary))));
    out.push_str("</p>\n");
    out
}

fn stats_line(ctx: &Ctx<'_>, lang: Lang) -> String {
    let facts = ctx.facts;
    if matches!(ctx.preset, Preset::Dataset | Preset::Showcase) {
        let mut parts = Vec::new();
        if facts.counts.top_dirs > 0 {
            parts.push(format!(
                "{} {}",
                format_count(facts.counts.top_dirs),
                t(lang, "folders", "个目录")
            ));
        }
        if facts.counts.images > 0 {
            parts.push(format!(
                "{} {}",
                format_count(facts.counts.images),
                t(lang, "images", "张图片")
            ));
        }
        if facts.counts.videos > 0 {
            parts.push(format!(
                "{} {}",
                format_count(facts.counts.videos),
                t(lang, "videos", "个视频")
            ));
        }
        if facts.sizes.media_bytes > 0 {
            parts.push(facts.media_human());
        }
        if !parts.is_empty() {
            return parts.join(" · ");
        }
    }

    let mut parts = Vec::new();
    parts.push(kind_label(facts.kind, lang).to_string());
    if let Some(language) = facts.languages.first() {
        parts.push(language.name.to_string());
    }
    if facts.counts.files > 0 {
        parts.push(format!(
            "{} {}",
            format_count(facts.counts.files),
            t(lang, "files", "个文件")
        ));
    }
    if let Some(license) = &facts.license {
        parts.push(license.clone());
    }
    parts.join(" · ")
}

fn kind_label(kind: ProjectKind, lang: Lang) -> &'static str {
    match (kind, lang) {
        (ProjectKind::Library, Lang::En) => "Library",
        (ProjectKind::Library, Lang::Zh) => "库",
        (ProjectKind::Cli, Lang::En) => "CLI tool",
        (ProjectKind::Cli, Lang::Zh) => "命令行工具",
        (ProjectKind::App, Lang::En) => "Application",
        (ProjectKind::App, Lang::Zh) => "应用",
        (ProjectKind::Dataset, Lang::En) => "Dataset",
        (ProjectKind::Dataset, Lang::Zh) => "数据集",
    }
}

fn badges(ctx: &Ctx<'_>) -> String {
    let facts = ctx.facts;
    let style = if ctx.config.badge_style.is_empty() {
        "flat"
    } else {
        ctx.config.badge_style.as_str()
    };
    let mut items: Vec<(String, String)> = Vec::new();

    if facts.counts.images > 0 {
        items.push((
            "images".to_string(),
            badge(
                "images",
                &format_count(facts.counts.images),
                "blue",
                style,
            ),
        ));
    }
    if facts.sizes.media_bytes > 0 {
        items.push((
            "media".to_string(),
            badge("media", &facts.media_human(), "9b59b6", style),
        ));
    }
    if let Some(language) = facts.languages.first() {
        items.push((
            language.name.to_string(),
            badge("language", language.name, "orange", style),
        ));
    }
    if let Some(license) = &facts.license {
        items.push((
            "license".to_string(),
            badge("license", license, "green", style),
        ));
    }
    if let Some(version) = &facts.version {
        items.push((
            "version".to_string(),
            badge("version", version, "blueviolet", style),
        ));
    }
    if let Some((owner, repo)) = facts.repo_slug() {
        let base = format!("https://img.shields.io/github/stars/{owner}/{repo}");
        items.push((
            "stars".to_string(),
            format!("{base}?style={style}"),
        ));
        let base = format!("https://img.shields.io/github/forks/{owner}/{repo}");
        items.push((
            "forks".to_string(),
            format!("{base}?style={style}"),
        ));
    }

    if items.is_empty() {
        return String::new();
    }
    let mut out = String::from("<p align=\"center\">\n");
    for (alt, url) in items {
        out.push_str(&format!(
            "  <img src=\"{url}\" alt=\"{alt}\">\n"
        ));
    }
    out.push_str("</p>\n");
    out
}

fn badge(label: &str, value: &str, color: &str, style: &str) -> String {
    let label = badge_escape(label);
    let value = badge_escape(value);
    format!("https://img.shields.io/badge/{label}-{value}-{color}?style={style}")
}

fn badge_escape(value: &str) -> String {
    let escaped = value.replace('-', "--").replace('_', "__");
    url_encode(&escaped)
}

fn language_nav(ctx: &Ctx<'_>) -> String {
    let mut links = Vec::new();
    for lang in ctx.locales {
        let anchor = match lang {
            Lang::En => "english",
            Lang::Zh => "chinese",
        };
        links.push(format!("<a href=\"#{anchor}\">{}</a>", lang.label()));
    }
    format!("<p align=\"center\">{}</p>\n", links.join(" · "))
}

fn render_language(ctx: &Ctx<'_>, lang: Lang) -> String {
    let specs = outline::sections(ctx.preset);
    let mut rendered: Vec<Rendered> = Vec::new();

    for spec in specs.iter().filter(|spec| !spec.is_header()) {
        if spec.kind == SectionKind::Toc {
            continue;
        }
        if let Some(body) = render_section(ctx, *spec, lang) {
            if !body.trim().is_empty() {
                rendered.push(Rendered {
                    id: spec.id,
                    title: outline::title(spec.kind, lang),
                    body: body.trim().to_string(),
                });
            }
        }
    }

    let by_id: HashMap<&str, &Rendered> =
        rendered.iter().map(|item| (item.id, item)).collect();

    let mut out = String::new();
    for spec in specs.iter().filter(|spec| !spec.is_header()) {
        if spec.kind == SectionKind::Toc {
            if rendered.len() >= 2 {
                let mut toc = String::from("## ");
                toc.push_str(outline::title(spec.kind, lang));
                toc.push_str("\n\n");
                for item in &rendered {
                    toc.push_str(&format!(
                        "- [{}](#{})\n",
                        item.title,
                        lang.scoped_id(item.id)
                    ));
                }
                out.push_str(&toc);
                out.push('\n');
            }
            continue;
        }

        let Some(item) = by_id.get(spec.id) else {
            continue;
        };
        out.push_str(&section_block(lang, spec, &item.body));
    }

    out
}

fn section_block(lang: Lang, spec: &SectionSpec, body: &str) -> String {
    let scoped = lang.scoped_id(spec.id);
    let title = outline::title(spec.kind, lang);
    let mut out = String::new();
    out.push_str(&format!(
        "<!-- readme-gen:section:{}:{}:start -->\n",
        lang.as_str(),
        spec.id
    ));
    out.push_str(&format!("<a id=\"{scoped}\"></a>\n"));
    out.push_str(&format!("## {title}\n\n"));
    out.push_str(body.trim());
    out.push_str(&format!(
        "\n<!-- readme-gen:section:{}:{}:end -->\n\n",
        lang.as_str(),
        spec.id
    ));
    out
}

fn keep_body(ctx: &Ctx<'_>, lang: Lang, id: &str) -> Option<String> {
    ctx.keeps
        .get(&(lang.as_str().to_string(), id.to_string()))
        .cloned()
}

fn render_section(ctx: &Ctx<'_>, spec: SectionSpec, lang: Lang) -> Option<String> {
    if let Some(keep) = keep_body(ctx, lang, spec.id) {
        return Some(format!(
            "<!-- readme-gen:keep:{}:{}:start -->\n{}\n<!-- readme-gen:keep:{}:{}:end -->",
            lang.as_str(),
            spec.id,
            keep.trim(),
            lang.as_str(),
            spec.id
        ));
    }
    match spec.kind {
        SectionKind::Hero | SectionKind::Badges | SectionKind::Languages | SectionKind::Toc => None,
        SectionKind::Quickstart => Some(quickstart(ctx, lang)),
        SectionKind::Packs => packs(ctx, lang),
        SectionKind::Numbers => numbers(ctx, lang),
        SectionKind::Catalog => catalog(ctx, lang),
        SectionKind::Structure => structure(ctx),
        SectionKind::Install => install(ctx, lang),
        SectionKind::Usage => usage(ctx, lang),
        SectionKind::Deps => deps(ctx, lang),
        SectionKind::Sources => sources(ctx, lang),
        SectionKind::Compatibility => compatibility(ctx, lang),
        SectionKind::Awards => awards(ctx, lang),
        SectionKind::License => license(ctx, lang),
        SectionKind::Contributing => contributing(ctx, lang),
        SectionKind::Changelog => changelog(ctx, lang),
        SectionKind::Cta => cta(ctx, lang),
        SectionKind::Prose(id) => prose(ctx, id, lang),
    }
}

fn prose(ctx: &Ctx<'_>, id: &str, lang: Lang) -> Option<String> {
    let authored = ctx
        .config
        .sections
        .get(id)
        .and_then(|loc| loc.get(lang))
        .map(str::to_string);
    if authored.is_some() {
        return authored;
    }
    ctx.generated
        .get(&(id.to_string(), lang))
        .cloned()
        .filter(|text| !text.trim().is_empty())
}

fn quickstart(ctx: &Ctx<'_>, lang: Lang) -> String {
    let clone = clone_url(ctx);
    let slug = ctx.facts.repo_slug();
    let mut out = String::new();

    if let Some(slug) = &slug {
        out.push_str(t(
            lang,
            "Pick the path that matches how you want to use it.",
            "按你的使用方式选一条路径。",
        ));
        out.push_str("\n\n");
        out.push_str(&table(
            &[t(lang, "Action", "操作"), t(lang, "What to do", "说明")],
            &[
                vec![
                    format!("**{}**", t(lang, "Bookmark", "收藏")),
                    format!(
                        "[{}](https://github.com/{}/{}/stargazers)",
                        t(lang, "Star the repo", "点 Star"),
                        slug.0,
                        slug.1
                    ),
                ],
                vec![
                    format!("**{}**", t(lang, "Own a copy", "自己留一份")),
                    format!(
                        "[{}](https://github.com/{}/{}/fork)",
                        t(lang, "Fork it", "Fork 到自己的账号"),
                        slug.0,
                        slug.1
                    ),
                ],
                vec![
                    format!("**{}**", t(lang, "Use locally", "本地使用")),
                    t(lang, "Clone the repository", "克隆仓库").to_string(),
                ],
            ],
        ));
        out.push('\n');
    }

    if let Some(clone) = &clone {
        let mut code = format!("git clone {clone}");
        if let Some(name) = repo_name_from_url(clone) {
            code.push_str(&format!("\ncd {name}"));
        }
        out.push_str(&format!(
            "```bash\n{code}\n\n# {}: {}\ngit clone --depth 1 {clone}\n```\n",
            t(lang, "optional shallow clone", "可选浅克隆"),
            t(lang, "faster, less history", "更快，历史更少")
        ));
    } else {
        out.push_str(t(
            lang,
            "Clone the repository and open the folder locally.",
            "克隆仓库并在本地打开该目录。",
        ));
    }
    out
}

fn packs(ctx: &Ctx<'_>, lang: Lang) -> Option<String> {
    let packs = &ctx.config.quickstart.packs;
    if packs.is_empty() {
        return None;
    }
    let mut rows = Vec::new();
    for pack in packs {
        let name = pack
            .name
            .get(lang)
            .or_else(|| pack.name.get_or_fallback(lang))
            .unwrap_or("")
            .to_string();
        let dirs = pack
            .dirs
            .iter()
            .map(|dir| format!("`{}`", md_code(dir)))
            .collect::<Vec<_>>()
            .join(" → ");
        let best_for = pack
            .best_for
            .get(lang)
            .or_else(|| pack.best_for.get_or_fallback(lang))
            .unwrap_or("")
            .to_string();
        rows.push(vec![format!("**{name}**"), dirs, best_for]);
    }
    Some(table(
        &[
            t(lang, "Pack", "精选包"),
            t(lang, "Open these folders first", "先打开这些目录"),
            t(lang, "Best for", "适合"),
        ],
        &rows,
    ))
}

fn numbers(ctx: &Ctx<'_>, lang: Lang) -> Option<String> {
    let facts = ctx.facts;
    let mut rows: Vec<(String, String)> = Vec::new();

    if facts.counts.top_dirs > 0 {
        rows.push((
            t(lang, "Folders", "目录").to_string(),
            format_count(facts.counts.top_dirs),
        ));
    }
    if facts.counts.files > 0 {
        rows.push((
            t(lang, "Files", "文件").to_string(),
            format_count(facts.counts.files),
        ));
    }
    if facts.counts.images > 0 {
        rows.push((
            t(lang, "Images", "图片").to_string(),
            format_count(facts.counts.images),
        ));
    }
    if facts.counts.videos > 0 {
        rows.push((
            t(lang, "Videos", "视频").to_string(),
            format_count(facts.counts.videos),
        ));
    }
    if facts.sizes.media_bytes > 0 {
        rows.push((
            t(lang, "Media size", "媒体体积").to_string(),
            facts.media_human(),
        ));
    }
    if !facts.languages.is_empty() {
        let value = facts
            .languages
            .iter()
            .take(3)
            .map(|language| {
                format!(
                    "{} ({})",
                    language.name,
                    format_count(language.files)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        rows.push((t(lang, "Languages", "语言").to_string(), value));
    }
    if !facts.deps.is_empty() {
        rows.push((
            t(lang, "Dependencies", "依赖").to_string(),
            format_count(facts.deps.len() as u64),
        ));
    }
    if let Some(tests) = &facts.tests {
        rows.push((
            t(lang, "Test files", "测试文件").to_string(),
            format_count(tests.files),
        ));
    }

    if rows.is_empty() {
        return None;
    }
    let mut out = String::from("| ");
    out.push_str(t(lang, "Metric", "指标"));
    out.push_str(" | ");
    out.push_str(t(lang, "Value", "数值"));
    out.push_str(" |\n|---|---|\n");
    for (label, value) in rows {
        out.push_str(&format!("| **{label}** | {value} |\n"));
    }
    Some(out)
}

fn catalog(ctx: &Ctx<'_>, lang: Lang) -> Option<String> {
    if ctx.facts.dirs.is_empty() {
        return None;
    }
    let mut dirs: Vec<&crate::scan::DirStats> = ctx.facts.dirs.iter().collect();
    dirs.sort_by_key(|dir| {
        ctx.config
            .catalog_order
            .iter()
            .position(|name| name == &dir.path)
            .unwrap_or(usize::MAX)
    });

    let mut out = String::new();
    let mut count = 0usize;
    for dir in dirs {
        if count >= 60 {
            break;
        }
        count += 1;
        let annotation = ctx.config.dirs.get(&dir.path);
        out.push_str(&format!("### `{}`", md_code(&dir.path)));
        out.push_str(&format!(" · {}", dir.size_human()));
        if dir.images > 0 {
            out.push_str(&format!(
                " · {} {}",
                format_count(dir.images),
                t(lang, "images", "张图片")
            ));
        }
        if dir.videos > 0 {
            out.push_str(&format!(
                " · {} {}",
                format_count(dir.videos),
                t(lang, "videos", "个视频")
            ));
        }
        if dir.images == 0 && dir.videos == 0 {
            out.push_str(&format!(
                " · {} {}",
                format_count(dir.files),
                t(lang, "files", "个文件")
            ));
        }
        out.push_str("\n\n");

        if let Some(annotation) = annotation {
            if let Some(summary) = annotation
                .summary
                .get(lang)
                .or_else(|| annotation.summary.get_or_fallback(lang))
            {
                out.push_str(summary.trim());
                out.push_str("\n\n");
            }
            if !annotation.bullets.is_empty() {
                for bullet in &annotation.bullets {
                    if let Some(text) = bullet.get(lang).or_else(|| bullet.get_or_fallback(lang)) {
                        out.push_str(&format!("- {}\n", text.trim()));
                    }
                }
                out.push('\n');
            }
        }
    }
    Some(out.trim_end().to_string())
}

fn structure(ctx: &Ctx<'_>) -> Option<String> {
    if ctx.facts.tree.trim().is_empty() {
        return None;
    }
    let fence = fence_for(&ctx.facts.tree);
    Some(format!(
        "{fence}text\n{}\n{fence}",
        ctx.facts.tree.trim_end()
    ))
}

fn install(ctx: &Ctx<'_>, lang: Lang) -> Option<String> {
    let facts = ctx.facts;
    let root = &facts.root;
    let mut commands: Vec<String> = Vec::new();

    if ctx.preset == Preset::Library {
        if let Some(clone) = clone_url(ctx) {
            commands.push(format!("git clone {clone}"));
            if let Some(name) = repo_name_from_url(&clone) {
                commands.push(format!("cd {name}"));
            }
        }
    }

    if root.join("Cargo.toml").exists() {
        commands.push("cargo build --release".to_string());
        if facts.kind == ProjectKind::Cli {
            commands.push("cargo install --path .".to_string());
        }
    } else if root.join("requirements.txt").exists() || root.join("pyproject.toml").exists() {
        commands.push("python -m venv .venv".to_string());
        commands.push("# macOS / Linux".to_string());
        commands.push("source .venv/bin/activate".to_string());
        commands.push("# Windows (Git Bash)".to_string());
        commands.push("source .venv/Scripts/activate".to_string());
        if root.join("requirements.txt").exists() {
            commands.push("pip install -r requirements.txt".to_string());
        } else {
            commands.push("pip install -e .".to_string());
        }
    } else if root.join("package.json").exists() {
        commands.push("npm install".to_string());
    } else if root.join("go.mod").exists() {
        commands.push("go build ./...".to_string());
    }

    if commands.is_empty() {
        return None;
    }

    let mut out = String::new();
    out.push_str(t(
        lang,
        "Build from source with the commands for your platform.",
        "按所在平台从源码构建。",
    ));
    out.push_str("\n\n");
    out.push_str(&format!("```bash\n{}\n```\n", commands.join("\n")));
    Some(out)
}

fn usage(ctx: &Ctx<'_>, lang: Lang) -> Option<String> {
    if let Some(body) = ctx
        .config
        .sections
        .get("usage")
        .and_then(|loc| loc.get(lang))
    {
        return Some(body.to_string());
    }
    let facts = ctx.facts;
    let entry = facts.entrypoints.first()?;
    let command = if entry.ends_with("main.rs") {
        "cargo run --release -- --help".to_string()
    } else if entry.ends_with(".py") {
        format!("python {} --help", entry)
    } else if entry.ends_with(".go") {
        "go run . --help".to_string()
    } else if entry.ends_with(".js") || entry.ends_with(".ts") {
        format!("node {} --help", entry)
    } else {
        return None;
    };

    let mut out = String::new();
    out.push_str(t(
        lang,
        "Run the command-line interface:",
        "运行命令行界面：",
    ));
    out.push_str("\n\n");
    out.push_str(&format!("```bash\n{command}\n```\n"));
    Some(out)
}

fn deps(ctx: &Ctx<'_>, lang: Lang) -> Option<String> {
    if ctx.facts.deps.is_empty() {
        return None;
    }
    let mut out = String::from("| ");
    out.push_str(t(lang, "Package", "包"));
    out.push_str(" | ");
    out.push_str(t(lang, "Version", "版本"));
    out.push_str(" | ");
    out.push_str(t(lang, "Ecosystem", "生态"));
    out.push_str(" |\n|---|---|---|\n");
    for dep in ctx.facts.deps.iter().take(25) {
        out.push_str(&format!(
            "| `{}` | {} | {} |\n",
            md_code(&dep.name),
            md_cell(if dep.spec.is_empty() { "—" } else { dep.spec.as_str() }),
            dep.ecosystem
        ));
    }
    if ctx.facts.deps.len() > 25 {
        out.push_str(&format!(
            "\n_{} more dependencies omitted._\n",
            ctx.facts.deps.len() - 25
        ));
    }
    Some(out)
}

fn sources(ctx: &Ctx<'_>, lang: Lang) -> Option<String> {
    if ctx.config.sources.is_empty() {
        return None;
    }
    let mut rows = Vec::new();
    for source in &ctx.config.sources {
        let name = match &source.url {
            Some(url) => format!("[{}]({})", md_cell(&source.name), url),
            None => md_cell(&source.name),
        };
        let note = source
            .note
            .get(lang)
            .or_else(|| source.note.get_or_fallback(lang))
            .unwrap_or("")
            .to_string();
        rows.push(vec![name, note]);
    }
    Some(table(
        &[t(lang, "Source", "来源"), t(lang, "What you get", "内容")],
        &rows,
    ))
}

fn compatibility(ctx: &Ctx<'_>, lang: Lang) -> Option<String> {
    if ctx.config.compatibility.is_empty() {
        return None;
    }
    let mut rows = Vec::new();
    for item in &ctx.config.compatibility {
        let note = item
            .note
            .get(lang)
            .or_else(|| item.note.get_or_fallback(lang))
            .unwrap_or("")
            .to_string();
        rows.push(vec![format!("**{}**", md_cell(&item.name)), note]);
    }
    Some(table(
        &[t(lang, "Format", "格式"), t(lang, "Notes", "说明")],
        &rows,
    ))
}

fn awards(ctx: &Ctx<'_>, lang: Lang) -> Option<String> {
    if ctx.config.awards.is_empty() {
        return None;
    }
    let mut rows = Vec::new();
    for item in &ctx.config.awards {
        let award = item
            .award
            .get(lang)
            .or_else(|| item.award.get_or_fallback(lang))
            .unwrap_or("")
            .to_string();
        rows.push(vec![award, format!("`{}`", md_code(&item.winner))]);
    }
    Some(table(
        &[t(lang, "Award", "奖项"), t(lang, "Winner", "得主")],
        &rows,
    ))
}

fn license(ctx: &Ctx<'_>, lang: Lang) -> Option<String> {
    let name = ctx
        .config
        .project
        .license
        .clone()
        .or_else(|| ctx.facts.license.clone())?;
    let file = ctx
        .facts
        .license_file
        .clone()
        .or_else(|| {
            let root = &ctx.facts.root;
            ["LICENSE", "LICENSE.md", "LICENSE.txt", "COPYING"]
                .iter()
                .find(|name| root.join(name).exists())
                .map(|name| name.to_string())
        });
    let mut out = match lang {
        Lang::En => format!("Released under the **{name}** license."),
        Lang::Zh => format!("本项目采用 **{name}** 许可证。"),
    };
    if let Some(file) = file {
        out.push_str(&format!(
            " {}",
            match lang {
                Lang::En => format!("See [`{file}`]({file})."),
                Lang::Zh => format!("详见 [`{file}`]({file})。"),
            }
        ));
    }
    Some(out)
}

fn contributing(ctx: &Ctx<'_>, lang: Lang) -> Option<String> {
    let mut out = String::new();
    out.push_str(t(
        lang,
        "Issues and pull requests are welcome. Please keep changes small and focused, and run the test suite before opening a PR.",
        "欢迎提交 Issue 和 Pull Request。请保持改动小而聚焦，并在提交前跑完测试。",
    ));
    let slug = ctx.facts.repo_slug();
    if let Some((owner, repo)) = slug {
        out.push_str("\n\n");
        out.push_str(&format!(
            "- [{}](https://github.com/{owner}/{repo}/issues)\n- [{}](https://github.com/{owner}/{repo}/pulls)\n",
            t(lang, "Open an issue", "提交 Issue"),
            t(lang, "Open a pull request", "提交 PR")
        ));
    }
    if ctx.facts.root.join("CONTRIBUTING.md").exists() {
        out.push_str(&format!(
            "\n{}",
            match lang {
                Lang::En => "See [CONTRIBUTING.md](CONTRIBUTING.md) for details.",
                Lang::Zh => "详见 [CONTRIBUTING.md](CONTRIBUTING.md)。",
            }
        ));
    }
    Some(out)
}

fn changelog(ctx: &Ctx<'_>, lang: Lang) -> Option<String> {
    if !ctx.config.changelog.is_empty() {
        let mut out = String::from("| ");
        out.push_str(t(lang, "When", "时间"));
        out.push_str(" | ");
        out.push_str(t(lang, "What changed", "变更"));
        out.push_str(" |\n|---|---|\n");
        for entry in &ctx.config.changelog {
            let text = entry
                .text
                .get(lang)
                .or_else(|| entry.text.get_or_fallback(lang))
                .unwrap_or("");
            out.push_str(&format!(
                "| **{}** | {} |\n",
                md_cell(&entry.date),
                md_cell(text)
            ));
        }
        return Some(out);
    }

    let git = ctx.facts.git.as_ref()?;
    if git.recent.is_empty() {
        return None;
    }
    let mut out = String::from("| ");
    out.push_str(t(lang, "Date", "日期"));
    out.push_str(" | ");
    out.push_str(t(lang, "Commit", "提交"));
    out.push_str(" |\n|---|---|\n");
    for commit in git.recent.iter().take(6) {
        out.push_str(&format!(
            "| {} | {} |\n",
            md_cell(&commit.date),
            md_cell(&commit.subject)
        ));
    }
    Some(out)
}

fn cta(ctx: &Ctx<'_>, lang: Lang) -> Option<String> {
    let (owner, repo) = ctx.facts.repo_slug()?;
    let star = ctx.config.cta.star.unwrap_or(true);
    let fork = ctx.config.cta.fork.unwrap_or(true);
    if !star && !fork {
        return None;
    }
    let mut out = String::new();
    if let Some(text) = ctx
        .config
        .cta
        .text
        .get(lang)
        .or_else(|| ctx.config.cta.text.get_or_fallback(lang))
    {
        out.push_str(text.trim());
        out.push_str("\n\n");
    }
    out.push_str("<p align=\"center\">\n");
    if star {
        out.push_str(&format!(
            "  <a href=\"https://github.com/{owner}/{repo}/stargazers\"><img src=\"https://img.shields.io/badge/⭐_Star-ff4785?style=for-the-badge\" alt=\"Star\"></a>\n"
        ));
    }
    if fork {
        out.push_str(&format!(
            "  <a href=\"https://github.com/{owner}/{repo}/fork\"><img src=\"https://img.shields.io/badge/🍴_Fork-24292f?style=for-the-badge\" alt=\"Fork\"></a>\n"
        ));
    }
    out.push_str("</p>\n");
    Some(out)
}

fn footer(ctx: &Ctx<'_>) -> String {
    if !ctx.config.credit {
        return String::new();
    }
    format!(
        "---\n\n*Generated by [readme-gen]({GENERATOR_URL}) v{} from repository facts. Facts hash: `{}`.*\n",
        ctx.version,
        &ctx.facts.hash[..12]
    )
}

fn project_name(ctx: &Ctx<'_>) -> String {
    ctx.config
        .project
        .name
        .clone()
        .unwrap_or_else(|| ctx.facts.name.clone())
}

fn clone_url(ctx: &Ctx<'_>) -> Option<String> {
    if let Some(clone) = &ctx.config.quickstart.clone_url {
        return Some(clone.clone());
    }
    let (owner, repo) = ctx.facts.repo_slug()?;
    Some(format!("https://github.com/{owner}/{repo}.git"))
}

fn repo_name_from_url(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches(".git").trim_end_matches('/');
    let name = trimmed.rsplit('/').next()?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    out.push('|');
    for header in headers {
        out.push(' ');
        out.push_str(header);
        out.push_str(" |");
    }
    out.push('\n');
    out.push('|');
    for _ in headers {
        out.push_str("---|");
    }
    out.push('\n');
    for row in rows {
        out.push('|');
        for cell in row {
            out.push(' ');
            out.push_str(&md_cell(cell));
            out.push_str(" |");
        }
        out.push('\n');
    }
    out
}

fn md_cell(value: &str) -> String {
    value
        .replace('|', "\\|")
        .replace("\r\n", "\n")
        .replace('\n', "<br>")
        .trim()
        .to_string()
}

fn md_code(value: &str) -> String {
    value.replace('`', "'").replace('\n', " ")
}

fn fence_for(body: &str) -> String {
    if body.contains("```") {
        "````".to_string()
    } else {
        "```".to_string()
    }
}

fn t(lang: Lang, en: &'static str, zh: &'static str) -> &'static str {
    match lang {
        Lang::En => en,
        Lang::Zh => zh,
    }
}

/// Extract `<!-- readme-gen:keep:{lang}:{id}:start -->` blocks from an existing
/// README so a rebuild never destroys hand-written overrides.
pub fn extract_keeps(text: &str) -> BTreeMap<(String, String), String> {
    let mut keeps = BTreeMap::new();
    let mut current: Option<(String, String, String)> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(inner) = trimmed
            .strip_prefix("<!-- readme-gen:keep:")
            .and_then(|rest| rest.strip_suffix("-->"))
        {
            let parts: Vec<&str> = inner.trim().split(':').collect();
            if parts.len() == 3 {
                let lang = parts[0].trim();
                let id = parts[1].trim();
                match parts[2].trim() {
                    "start" => {
                        current = Some((lang.to_string(), id.to_string(), String::new()));
                    }
                    "end" => {
                        if let Some((lang, id, body)) = current.take() {
                            keeps.insert((lang, id), body.trim_end().to_string());
                        }
                    }
                    _ => {}
                }
            }
        } else if let Some((_, _, body)) = current.as_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    keeps
}

/// Parse the metadata comment emitted by [`build`].
pub fn extract_meta(text: &str) -> Option<crate::json::Json> {
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with("<!-- readme-gen:meta"))?;
    let inner = line
        .trim()
        .strip_prefix("<!-- readme-gen:meta ")?
        .strip_suffix(" -->")?;
    crate::json::parse(inner).ok()
}

/// Extract the facts hash embedded by [`build`], if present.
pub fn extract_facts_hash(text: &str) -> Option<String> {
    let value = extract_meta(text)?;
    value.get("facts")?.as_str().map(str::to_string)
}

/// Locales recorded in the metadata comment (used by `check` when no config).
pub fn extract_meta_locales(text: &str) -> Option<Vec<Lang>> {
    let value = extract_meta(text)?;
    let items = value.get("locales")?.as_array()?;
    let mut locales = Vec::new();
    for item in items {
        if let Some(lang) = item.as_str().and_then(Lang::parse) {
            locales.push(lang);
        }
    }
    if locales.is_empty() {
        None
    } else {
        Some(locales)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_keep_blocks() {
        let text = r#"
<!-- readme-gen:keep:en:usage:start -->
Hand-written usage.
<!-- readme-gen:keep:en:usage:end -->
"#;
        let keeps = extract_keeps(text);
        assert_eq!(
            keeps.get(&("en".to_string(), "usage".to_string())),
            Some(&"Hand-written usage.".to_string())
        );
    }

    #[test]
    fn reads_meta_hash() {
        let text = r#"<!-- readme-gen:meta {"schema":1,"facts":"abc123"} -->
# x
"#;
        assert_eq!(extract_facts_hash(text).as_deref(), Some("abc123"));
    }

    #[test]
    fn tables_escape_pipes() {
        let table = table(&["a", "b"], &[vec!["x|y".to_string(), "z".to_string()]]);
        assert!(table.contains(r"x\|y"));
    }
}
