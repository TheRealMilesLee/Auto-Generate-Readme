//! Repository scanning: the factual layer.
//!
//! Everything in [`Facts`] is mechanically observable. The scanner never reads
//! file contents except for a handful of small metadata files (manifests,
//! licence, README hashing), and never shells out to `find`, `tree` or `du`, so
//! it behaves the same on macOS, Linux and Windows (Git Bash / PowerShell).

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Error, Result};
use crate::glob::GlobSet;
use crate::json::Json;
use crate::sha256;
use crate::util::{human_size, read_text_lossy};

pub const IMAGE_EXTS: &[&str] = &[
    "jpg", "jpeg", "png", "heic", "heif", "webp", "bmp", "tif", "tiff", "avif", "gif",
];
pub const VIDEO_EXTS: &[&str] = &["mp4", "mov", "mkv", "webm", "avi", "m4v"];

const DEFAULT_EXCLUDES: &[&str] = &[
    ".git/",
    "target/",
    "node_modules/",
    ".venv/",
    "venv/",
    "__pycache__/",
    ".pytest_cache/",
    "dist/",
    "build/",
    ".idea/",
    ".vscode/",
    ".readme/",
    ".DS_Store",
    "Thumbs.db",
];

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub root: PathBuf,
    pub excludes: Vec<String>,
    /// Directory depth included in the `dirs` catalogue (1 = top level).
    pub catalog_depth: usize,
    /// Safety limit for the recursive walk.
    pub max_depth: usize,
    pub max_files: usize,
    /// Depth of the rendered tree (1 = project root + first level).
    pub tree_depth: usize,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            excludes: Vec::new(),
            catalog_depth: 1,
            max_depth: 16,
            max_files: 200_000,
            tree_depth: 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectKind {
    Library,
    Cli,
    App,
    Dataset,
}

impl ProjectKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectKind::Library => "library",
            ProjectKind::Cli => "cli",
            ProjectKind::App => "app",
            ProjectKind::Dataset => "dataset",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Counts {
    pub files: u64,
    pub dirs: u64,
    pub images: u64,
    pub videos: u64,
    pub top_dirs: u64,
}

#[derive(Debug, Clone, Default)]
pub struct Sizes {
    pub total_bytes: u64,
    pub media_bytes: u64,
    pub image_bytes: u64,
    pub video_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct LanguageStat {
    pub name: &'static str,
    pub files: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone)]
pub struct DirStats {
    pub path: String,
    pub depth: usize,
    pub files: u64,
    pub bytes: u64,
    pub images: u64,
    pub videos: u64,
    pub exts: BTreeMap<String, u64>,
}

impl DirStats {
    pub fn size_human(&self) -> String {
        human_size(self.bytes)
    }
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub spec: String,
    pub ecosystem: &'static str,
}

#[derive(Debug, Clone)]
pub struct TestInfo {
    pub dirs: Vec<String>,
    pub files: u64,
}

#[derive(Debug, Clone)]
pub struct ReadmeInfo {
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone)]
pub struct Commit {
    pub date: String,
    pub subject: String,
}

#[derive(Debug, Clone, Default)]
pub struct GitInfo {
    pub remote: Option<String>,
    pub branch: Option<String>,
    pub commits: u64,
    pub first_date: Option<String>,
    pub last_date: Option<String>,
    pub recent: Vec<Commit>,
}

#[derive(Debug, Clone)]
pub struct Facts {
    pub root: PathBuf,
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub kind: ProjectKind,
    pub license: Option<String>,
    pub license_file: Option<String>,
    pub counts: Counts,
    pub sizes: Sizes,
    pub languages: Vec<LanguageStat>,
    pub dirs: Vec<DirStats>,
    pub tree: String,
    pub deps: Vec<Dependency>,
    pub entrypoints: Vec<String>,
    pub tests: Option<TestInfo>,
    pub readme: Option<ReadmeInfo>,
    pub git: Option<GitInfo>,
    pub warnings: Vec<String>,
    pub hash: String,
}

#[derive(Debug, Default)]
struct TreeNode {
    name: String,
    is_dir: bool,
    children: Vec<TreeNode>,
    more: usize,
}

pub fn scan(options: &ScanOptions) -> Result<Facts> {
    let root = absolute(options.root.clone())?;
    if !root.is_dir() {
        return Err(Error::new(format!(
            "not a directory: {}",
            root.display()
        )));
    }

    let mut excludes: Vec<String> = DEFAULT_EXCLUDES.iter().map(|s| s.to_string()).collect();
    excludes.extend(options.excludes.iter().cloned());
    if let Ok(gitignore) = fs::read_to_string(root.join(".gitignore")) {
        excludes.extend(
            gitignore
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .collect::<Vec<_>>(),
        );
    }
    let globs = GlobSet::new(excludes);

    let mut walker = Walker::new(options, globs.clone());
    walker.walk(&root, "", 0)?;

    let mut dirs: Vec<DirStats> = walker
        .agg
        .into_iter()
        .filter(|(_, stats)| stats.depth == options.catalog_depth && stats.files > 0)
        .map(|(_, stats)| stats)
        .collect();
    dirs.sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));

    let mut languages: Vec<LanguageStat> = walker
        .languages
        .into_iter()
        .map(|(name, (files, bytes))| LanguageStat { name, files, bytes })
        .collect();
    languages.sort_by(|a, b| b.bytes.cmp(&a.bytes).then(a.name.cmp(b.name)));

    let name = detect_name(&root);
    let description = detect_description(&root);
    let version = detect_version(&root);
    let (license, license_file) = detect_license(&root);
    let deps = detect_dependencies(&root);
    let readme = detect_readme(&root);
    let git = detect_git(&root);

    let mut counts = walker.counts;
    counts.top_dirs = dirs.len() as u64;

    let tree = build_tree(&root, "", 0, options, &globs)?
        .render();

    let tests = if walker.test_files > 0 {
        Some(TestInfo {
            dirs: walker.test_dirs,
            files: walker.test_files,
        })
    } else {
        None
    };

    let kind = detect_kind(&root, &counts, &walker.entrypoints);

    let mut facts = Facts {
        root,
        name,
        description,
        version,
        kind,
        license,
        license_file,
        counts,
        sizes: walker.sizes,
        languages,
        dirs,
        tree,
        deps,
        entrypoints: walker.entrypoints,
        tests,
        readme,
        git,
        warnings: walker.warnings,
        hash: String::new(),
    };
    facts.hash = facts.content_hash();
    Ok(facts)
}

fn absolute(path: PathBuf) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

struct Walker<'a> {
    options: &'a ScanOptions,
    globs: GlobSet,
    counts: Counts,
    sizes: Sizes,
    agg: HashMap<String, DirStats>,
    languages: HashMap<&'static str, (u64, u64)>,
    entrypoints: Vec<String>,
    test_files: u64,
    test_dirs: Vec<String>,
    warnings: Vec<String>,
}

impl<'a> Walker<'a> {
    fn new(options: &'a ScanOptions, globs: GlobSet) -> Self {
        Self {
            options,
            globs,
            counts: Counts::default(),
            sizes: Sizes::default(),
            agg: HashMap::new(),
            languages: HashMap::new(),
            entrypoints: Vec::new(),
            test_files: 0,
            test_dirs: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn walk(&mut self, dir: &Path, rel: &str, depth: usize) -> Result<()> {
        if depth > self.options.max_depth {
            return Ok(());
        }
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(error) => {
                self.warnings
                    .push(format!("cannot read {}: {error}", dir.display()));
                return Ok(());
            }
        };
        let mut entries: Vec<_> = entries.filter_map(|entry| entry.ok()).collect();
        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            let name = entry.file_name().to_string_lossy().to_string();
            let child_rel = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            let metadata = match fs::symlink_metadata(entry.path()) {
                Ok(metadata) => metadata,
                Err(error) => {
                    self.warnings
                        .push(format!("cannot stat {}: {error}", child_rel));
                    continue;
                }
            };

            if metadata.file_type().is_symlink() {
                continue;
            }
            if depth == 0 && metadata.is_file() && is_readme_file(&name) {
                continue;
            }

            if metadata.is_dir() {
                if self.globs.is_ignored(&child_rel, true) {
                    continue;
                }
                self.counts.dirs += 1;
                if depth + 1 <= self.options.catalog_depth {
                    self.agg.entry(child_rel.clone()).or_insert_with(|| DirStats {
                        path: child_rel.clone(),
                        depth: depth + 1,
                        files: 0,
                        bytes: 0,
                        images: 0,
                        videos: 0,
                        exts: BTreeMap::new(),
                    });
                }
                if depth + 1 == 1 {
                    let lower = name.to_lowercase();
                    if matches!(lower.as_str(), "tests" | "test" | "spec") {
                        self.test_dirs.push(child_rel.clone());
                    }
                }
                self.walk(&entry.path(), &child_rel, depth + 1)?;
            } else if metadata.is_file() {
                if self.globs.is_ignored(&child_rel, false) {
                    continue;
                }
                self.counts.files += 1;
                if self.counts.files > self.options.max_files as u64 {
                    return Err(Error::new(format!(
                        "more than {} files; pass --max-files to raise the limit",
                        self.options.max_files
                    )));
                }
                let bytes = metadata.len();
                self.sizes.total_bytes += bytes;

                let ext = entry
                    .path()
                    .extension()
                    .map(|ext| ext.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                let is_image = IMAGE_EXTS.contains(&ext.as_str());
                let is_video = VIDEO_EXTS.contains(&ext.as_str());
                if is_image {
                    self.counts.images += 1;
                    self.sizes.image_bytes += bytes;
                    self.sizes.media_bytes += bytes;
                } else if is_video {
                    self.counts.videos += 1;
                    self.sizes.video_bytes += bytes;
                    self.sizes.media_bytes += bytes;
                }

                if let Some(language) = language_for(&ext) {
                    let entry = self.languages.entry(language).or_insert((0, 0));
                    entry.0 += 1;
                    entry.1 += bytes;
                }

                self.add_to_dirs(rel, bytes, is_image, is_video, &ext);
                self.collect_entrypoint(&child_rel, depth + 1);

                if child_rel.starts_with("tests/")
                    || child_rel.starts_with("test/")
                    || child_rel.starts_with("spec/")
                {
                    self.test_files += 1;
                }
            }
        }
        Ok(())
    }

    fn add_to_dirs(&mut self, parent_rel: &str, bytes: u64, image: bool, video: bool, ext: &str) {
        if parent_rel.is_empty() {
            return;
        }
        let mut prefix = String::new();
        for part in parent_rel.split('/').filter(|part| !part.is_empty()) {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            let depth = prefix.matches('/').count() + 1;
            if depth > self.options.catalog_depth {
                break;
            }
            if let Some(stats) = self.agg.get_mut(&prefix) {
                stats.files += 1;
                stats.bytes += bytes;
                if image {
                    stats.images += 1;
                }
                if video {
                    stats.videos += 1;
                }
                if !ext.is_empty() {
                    *stats.exts.entry(ext.to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    fn collect_entrypoint(&mut self, rel: &str, depth: usize) {
        if depth > 2 || self.entrypoints.len() >= 12 {
            return;
        }
        let name = rel.rsplit('/').next().unwrap_or(rel);
        let is_entry = matches!(
            name,
            "main.py"
                | "app.py"
                | "cli.py"
                | "__main__.py"
                | "main.rs"
                | "main.go"
                | "main.cpp"
                | "main.c"
                | "index.js"
                | "index.ts"
                | "cli.js"
                | "cli.ts"
        );
        if is_entry && !self.entrypoints.iter().any(|existing| existing == rel) {
            self.entrypoints.push(rel.to_string());
        }
    }
}

fn language_for(ext: &str) -> Option<&'static str> {
    let language = match ext {
        "rs" => "Rust",
        "py" | "pyi" => "Python",
        "js" | "mjs" | "cjs" => "JavaScript",
        "jsx" => "JavaScript",
        "ts" | "mts" | "cts" => "TypeScript",
        "tsx" => "TypeScript",
        "go" => "Go",
        "java" => "Java",
        "kt" | "kts" => "Kotlin",
        "c" => "C",
        "h" => "C/C++ header",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => "C++",
        "cs" => "C#",
        "rb" => "Ruby",
        "php" => "PHP",
        "swift" => "Swift",
        "m" | "mm" => "Objective-C",
        "scala" => "Scala",
        "sh" | "bash" | "zsh" => "Shell",
        "ps1" => "PowerShell",
        "lua" => "Lua",
        "dart" => "Dart",
        "ex" | "exs" => "Elixir",
        "hs" => "Haskell",
        "ml" | "mli" => "OCaml",
        "clj" | "cljs" => "Clojure",
        "vue" => "Vue",
        "svelte" => "Svelte",
        "html" | "htm" => "HTML",
        "css" => "CSS",
        "scss" | "sass" => "Sass",
        "sql" => "SQL",
        "r" => "R",
        "jl" => "Julia",
        "zig" => "Zig",
        "nim" => "Nim",
        "sol" => "Solidity",
        "asm" | "s" => "Assembly",
        _ => return None,
    };
    Some(language)
}

fn build_tree(
    dir: &Path,
    rel: &str,
    depth: usize,
    options: &ScanOptions,
    globs: &GlobSet,
) -> Result<TreeNode> {
    let mut node = TreeNode {
        name: if depth == 0 {
            dir.file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "project".to_string())
        } else {
            dir.file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default()
        },
        is_dir: true,
        children: Vec::new(),
        more: 0,
    };
    if depth >= options.tree_depth {
        return Ok(node);
    }

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(node),
    };
    let mut entries: Vec<_> = entries.filter_map(|entry| entry.ok()).collect();
    entries.sort_by_key(|entry| {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        (if is_dir { 0 } else { 1 }, name.to_lowercase())
    });

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let child_rel = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let metadata = match fs::symlink_metadata(entry.path()) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        let is_dir = metadata.is_dir();
        if depth == 0 && !is_dir && is_readme_file(&name) {
            continue;
        }
        if globs.is_ignored(&child_rel, is_dir) {
            continue;
        }
        if node.children.len() >= 40 {
            node.more += 1;
            continue;
        }
        if is_dir {
            node.children.push(build_tree(
                &entry.path(),
                &child_rel,
                depth + 1,
                options,
                globs,
            )?);
        } else {
            node.children.push(TreeNode {
                name,
                is_dir: false,
                children: Vec::new(),
                more: 0,
            });
        }
    }
    Ok(node)
}

impl TreeNode {
    fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&self.name);
        out.push('/');
        out.push('\n');
        self.render_children("", &mut out);
        out
    }

    fn render_children(&self, prefix: &str, out: &mut String) {
        let total = self.children.len() + if self.more > 0 { 1 } else { 0 };
        for (index, child) in self.children.iter().enumerate() {
            let last = index + 1 == total;
            out.push_str(prefix);
            out.push_str(if last { "└── " } else { "├── " });
            out.push_str(&child.name);
            if child.is_dir {
                out.push('/');
            }
            out.push('\n');
            if child.is_dir && !child.children.is_empty() {
                let next_prefix = format!("{prefix}{}", if last { "    " } else { "│   " });
                child.render_children(&next_prefix, out);
            }
        }
        if self.more > 0 {
            out.push_str(prefix);
            out.push_str("└── … (+");
            out.push_str(&self.more.to_string());
            out.push_str(" more)\n");
        }
    }
}

fn is_readme_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    if !lower.starts_with("readme") {
        return false;
    }
    lower.ends_with(".md")
        || lower.ends_with(".rst")
        || lower.ends_with(".txt")
        || !lower.contains('.')
}

fn detect_name(root: &Path) -> String {
    if let Some(name) = read_cargo_field(root, "name") {
        return name;
    }
    if let Some(name) = read_package_json_field(root, "name") {
        return name;
    }
    if let Some(name) = read_pyproject_field(root, "name") {
        return name;
    }
    root.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string())
}

fn detect_description(root: &Path) -> String {
    read_cargo_field(root, "description")
        .or_else(|| read_package_json_field(root, "description"))
        .or_else(|| read_pyproject_field(root, "description"))
        .unwrap_or_default()
}

fn detect_version(root: &Path) -> Option<String> {
    read_cargo_field(root, "version")
        .or_else(|| read_package_json_field(root, "version"))
        .or_else(|| read_pyproject_field(root, "version"))
}

fn read_cargo_field(root: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let mut in_package = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if in_package {
            if let Some(value) = inline_value(line, key) {
                return Some(value);
            }
        }
    }
    None
}

fn read_package_json_field(root: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(root.join("package.json")).ok()?;
    let value = crate::json::parse(&text).ok()?;
    value.get(key)?.as_str().map(|value| value.to_string())
}

fn read_pyproject_field(root: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(root.join("pyproject.toml")).ok()?;
    let mut section = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            section = line.trim_matches(|c| c == '[' || c == ']').to_string();
            continue;
        }
        if matches!(
            section.as_str(),
            "project" | "tool.poetry" | "tool.poetry.project"
        ) {
            if let Some(value) = inline_value(line, key) {
                return Some(value);
            }
        }
    }
    None
}

fn inline_value(line: &str, key: &str) -> Option<String> {
    let line = line.split('#').next().unwrap_or(line).trim();
    let rest = line.strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix('=')?.trim();
    let value = rest.trim().trim_matches(|c| c == '"' || c == '\'');
    if value.is_empty() || value.starts_with('{') {
        None
    } else {
        Some(value.to_string())
    }
}

fn detect_license(root: &Path) -> (Option<String>, Option<String>) {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return (None, None),
    };
    for entry in entries.filter_map(|entry| entry.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        let lower = name.to_lowercase();
        if !(lower.starts_with("license") || lower.starts_with("copying")) {
            continue;
        }
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let content = read_text_lossy(&path).unwrap_or_default();
        let head: String = content.chars().take(8000).collect::<String>().to_lowercase();
        let detected = if head.contains("mit license") {
            "MIT"
        } else if head.contains("apache license") {
            "Apache-2.0"
        } else if head.contains("gnu general public license") {
            if head.contains("version 3") {
                "GPL-3.0"
            } else {
                "GPL-2.0"
            }
        } else if head.contains("mozilla public license") {
            "MPL-2.0"
        } else if head.contains("bsd 3-clause") || head.contains("redistribution and use") && head.contains("neither the name") {
            "BSD-3-Clause"
        } else if head.contains("bsd") {
            "BSD"
        } else if head.contains("unlicense") {
            "Unlicense"
        } else {
            return (None, Some(name));
        };
        return (Some(detected.to_string()), Some(name));
    }

    // Fall back to manifest metadata.
    if let Some(text) = fs::read_to_string(root.join("Cargo.toml")).ok() {
        let mut in_package = false;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_package = line == "[package]";
                continue;
            }
            if in_package {
                if let Some(value) = inline_value(line, "license") {
                    return (Some(value), None);
                }
            }
        }
    }
    if let Some(value) = read_package_json_field(root, "license") {
        return (Some(value), None);
    }
    (None, None)
}

fn detect_readme(root: &Path) -> Option<ReadmeInfo> {
    let entries = fs::read_dir(root).ok()?;
    let mut candidates: Vec<(String, PathBuf)> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            let lower = name.to_lowercase();
            if lower.starts_with("readme") {
                Some((name, entry.path()))
            } else {
                None
            }
        })
        .collect();
    candidates.sort_by(|a, b| {
        let rank = |name: &str| {
            if name.eq_ignore_ascii_case("README.md") {
                0
            } else if name.to_lowercase().ends_with(".md") {
                1
            } else {
                2
            }
        };
        rank(&a.0).cmp(&rank(&b.0)).then(a.0.cmp(&b.0))
    });
    let (name, path) = candidates.into_iter().next()?;
    let bytes = fs::metadata(&path).ok()?.len();
    Some(ReadmeInfo {
        path: name,
        bytes,
    })
}

fn detect_git(root: &Path) -> Option<GitInfo> {
    let inside = git(root, &["rev-parse", "--is-inside-work-tree"])?;
    if inside.trim() != "true" {
        return None;
    }
    let remote = git(root, &["config", "--get", "remote.origin.url"])
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let branch = git(root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let commits = git(root, &["rev-list", "--count", "HEAD"])
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(0);
    let first_date = git(root, &["log", "--max-parents=0", "-1", "--format=%cs"])
        .map(|value| value.trim().to_string());
    let last_date = git(root, &["log", "-1", "--format=%cs"]).map(|value| value.trim().to_string());
    let recent = git(root, &["log", "-6", "--format=%cs%x09%s"])
        .map(|value| {
            value
                .lines()
                .filter_map(|line| {
                    let (date, subject) = line.split_once('\t')?;
                    Some(Commit {
                        date: date.trim().to_string(),
                        subject: subject.trim().to_string(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Some(GitInfo {
        remote,
        branch,
        commits,
        first_date,
        last_date,
        recent,
    })
}

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn detect_kind(root: &Path, counts: &Counts, entrypoints: &[String]) -> ProjectKind {
    if counts.images > 100 && counts.images * 2 > counts.files {
        return ProjectKind::Dataset;
    }
    if root.join("src/main.rs").exists() || root.join("src/lib.rs").exists() {
        return if root.join("src/main.rs").exists() {
            ProjectKind::Cli
        } else {
            ProjectKind::Library
        };
    }
    if root.join("Cargo.toml").exists() {
        let text = fs::read_to_string(root.join("Cargo.toml")).unwrap_or_default();
        if text.contains("[lib]") && !text.contains("[[bin]]") {
            return ProjectKind::Library;
        }
        if text.contains("[[bin]]") {
            return ProjectKind::Cli;
        }
    }
    if root.join("package.json").exists() {
        if let Ok(text) = fs::read_to_string(root.join("package.json")) {
            if let Ok(value) = crate::json::parse(&text) {
                if value.get("bin").is_some() {
                    return ProjectKind::Cli;
                }
            }
        }
    }
    if !entrypoints.is_empty() {
        return ProjectKind::Cli;
    }
    ProjectKind::App
}

fn detect_dependencies(root: &Path) -> Vec<Dependency> {
    let mut deps: Vec<Dependency> = Vec::new();
    let mut seen: HashMap<String, ()> = HashMap::new();
    let mut push = |name: String, spec: String, ecosystem: &'static str| {
        let key = format!("{ecosystem}:{name}");
        if seen.insert(key, ()).is_none() && !name.is_empty() {
            deps.push(Dependency {
                name,
                spec,
                ecosystem,
            });
        }
    };

    if let Ok(text) = fs::read_to_string(root.join("requirements.txt")) {
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() || line.starts_with('-') {
                continue;
            }
            let (name, spec) = split_dependency(line);
            push(name, spec, "python");
        }
    }

    if let Ok(text) = fs::read_to_string(root.join("Cargo.toml")) {
        let mut in_deps = false;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_deps = matches!(
                    line,
                    "[dependencies]" | "[dev-dependencies]" | "[build-dependencies]"
                );
                continue;
            }
            if !in_deps || line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                push(
                    key.trim().to_string(),
                    value.trim().trim_matches(|c| c == '"' || c == '\'').to_string(),
                    "rust",
                );
            }
        }
    }

    if let Ok(text) = fs::read_to_string(root.join("package.json")) {
        if let Ok(value) = crate::json::parse(&text) {
            for key in ["dependencies", "devDependencies"] {
                if let Some(Json::Obj(entries)) = value.get(key) {
                    for (name, spec) in entries {
                        push(
                            name.clone(),
                            spec.as_str().unwrap_or("").to_string(),
                            "node",
                        );
                    }
                }
            }
        }
    }

    if let Ok(text) = fs::read_to_string(root.join("pyproject.toml")) {
        let mut in_dependencies = false;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with("dependencies") && line.contains('[') {
                in_dependencies = true;
            }
            if in_dependencies {
                for quote in ['"', '\''] {
                    let mut rest = line;
                    while let Some(start) = rest.find(quote) {
                        let after = &rest[start + 1..];
                        if let Some(end) = after.find(quote) {
                            let value = &after[..end];
                            if !value.is_empty() && !value.contains(' ') {
                                let (name, spec) = split_dependency(value);
                                push(name, spec, "python");
                            }
                            rest = &after[end + 1..];
                        } else {
                            break;
                        }
                    }
                }
                if line.contains(']') {
                    in_dependencies = false;
                }
            }
        }
    }

    if let Ok(text) = fs::read_to_string(root.join("go.mod")) {
        let mut in_block = false;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with("require (") {
                in_block = true;
                continue;
            }
            if in_block && line == ")" {
                in_block = false;
                continue;
            }
            if in_block && !line.is_empty() {
                let mut parts = line.split_whitespace();
                if let (Some(name), Some(spec)) = (parts.next(), parts.next()) {
                    push(name.to_string(), spec.to_string(), "go");
                }
            }
        }
    }

    deps.truncate(40);
    deps
}

fn split_dependency(line: &str) -> (String, String) {
    let index = line
        .find(|ch: char| matches!(ch, '>' | '<' | '=' | '~' | '^' | '!' | '[' | ' '))
        .unwrap_or(line.len());
    let name = line[..index].trim().to_string();
    let spec = line[index..].trim().to_string();
    (name, spec)
}

fn parse_github_remote(remote: &str) -> Option<(String, String)> {
    let cleaned = remote.trim().trim_end_matches(".git");
    if let Some(rest) = cleaned.strip_prefix("git@github.com:") {
        let (owner, repo) = rest.split_once('/')?;
        return Some((owner.to_string(), repo.to_string()));
    }
    if let Some(index) = cleaned.find("github.com/") {
        let rest = &cleaned[index + "github.com/".len()..];
        let (owner, repo) = rest.split_once('/')?;
        return Some((owner.to_string(), repo.to_string()));
    }
    None
}

impl Facts {
    pub fn repo_slug(&self) -> Option<(String, String)> {
        self.git
            .as_ref()
            .and_then(|git| git.remote.as_deref())
            .and_then(parse_github_remote)
    }

    pub fn media_human(&self) -> String {
        human_size(self.sizes.media_bytes)
    }

    pub fn to_json(&self) -> Json {
        let mut root = Json::obj();
        root.push("schema", "readme-facts/v1");
        root.push("root", self.root.to_string_lossy().to_string());
        root.push("hash", self.hash.clone());

        let mut project = Json::obj();
        project.push("name", self.name.clone());
        project.push("description", self.description.clone());
        project.push("kind", self.kind.as_str());
        project.push("version", self.version.clone());
        project.push("license", self.license.clone());
        project.push("license_file", self.license_file.clone());
        root.push("project", project);

        let mut counts = Json::obj();
        counts.push("files", self.counts.files);
        counts.push("dirs", self.counts.dirs);
        counts.push("images", self.counts.images);
        counts.push("videos", self.counts.videos);
        counts.push("top_dirs", self.counts.top_dirs);
        root.push("counts", counts);

        let mut sizes = Json::obj();
        sizes.push("total_bytes", self.sizes.total_bytes);
        sizes.push("total", human_size(self.sizes.total_bytes));
        sizes.push("media_bytes", self.sizes.media_bytes);
        sizes.push("media", human_size(self.sizes.media_bytes));
        sizes.push("images", human_size(self.sizes.image_bytes));
        sizes.push("videos", human_size(self.sizes.video_bytes));
        root.push("sizes", sizes);

        let mut languages = Json::arr();
        for language in &self.languages {
            let mut item = Json::obj();
            item.push("name", language.name);
            item.push("files", language.files);
            item.push("bytes", language.bytes);
            item.push("size", human_size(language.bytes));
            languages.push_value(item);
        }
        root.push("languages", languages);

        let mut dirs = Json::arr();
        for dir in &self.dirs {
            let mut item = Json::obj();
            item.push("path", dir.path.clone());
            item.push("depth", dir.depth);
            item.push("files", dir.files);
            item.push("images", dir.images);
            item.push("videos", dir.videos);
            item.push("bytes", dir.bytes);
            item.push("size", dir.size_human());
            let mut exts = Json::obj();
            for (ext, count) in &dir.exts {
                exts.push(ext.clone(), *count);
            }
            item.push("exts", exts);
            dirs.push_value(item);
        }
        root.push("dirs", dirs);

        let mut deps = Json::arr();
        for dep in &self.deps {
            let mut item = Json::obj();
            item.push("name", dep.name.clone());
            item.push("spec", dep.spec.clone());
            item.push("ecosystem", dep.ecosystem);
            deps.push_value(item);
        }
        root.push("deps", deps);

        let mut entrypoints = Json::arr();
        for entry in &self.entrypoints {
            entrypoints.push_value(entry.clone());
        }
        root.push("entrypoints", entrypoints);

        let tests = match &self.tests {
            Some(tests) => {
                let mut item = Json::obj();
                let mut dirs = Json::arr();
                for dir in &tests.dirs {
                    dirs.push_value(dir.clone());
                }
                item.push("dirs", dirs);
                item.push("files", tests.files);
                item
            }
            None => Json::Null,
        };
        root.push("tests", tests);

        let readme = match &self.readme {
            Some(readme) => {
                let mut item = Json::obj();
                item.push("path", readme.path.clone());
                item.push("bytes", readme.bytes);
                item
            }
            None => Json::Null,
        };
        root.push("readme", readme);

        let git = match &self.git {
            Some(git) => {
                let mut item = Json::obj();
                item.push("remote", git.remote.clone());
                item.push("branch", git.branch.clone());
                item.push("commits", git.commits);
                item.push("first_date", git.first_date.clone());
                item.push("last_date", git.last_date.clone());
                let mut recent = Json::arr();
                for commit in &git.recent {
                    let mut entry = Json::obj();
                    entry.push("date", commit.date.clone());
                    entry.push("subject", commit.subject.clone());
                    recent.push_value(entry);
                }
                item.push("recent", recent);
                item
            }
            None => Json::Null,
        };
        root.push("git", git);

        root.push("tree", self.tree.clone());

        let mut warnings = Json::arr();
        for warning in &self.warnings {
            warnings.push_value(warning.clone());
        }
        root.push("warnings", warnings);

        root
    }

    /// Hash of the inputs that affect document content.
    ///
    /// Deliberately excludes the README itself (writing the document changes
    /// its size) and git history (committing the document changes the commit
    /// hash) so that `readme-gen check` is stable across a build/commit cycle.
    pub fn content_hash(&self) -> String {
        let mut root = Json::obj();

        let mut project = Json::obj();
        project.push("name", self.name.clone());
        project.push("kind", self.kind.as_str());
        root.push("project", project);

        let mut counts = Json::obj();
        counts.push("files", self.counts.files);
        counts.push("dirs", self.counts.dirs);
        counts.push("images", self.counts.images);
        counts.push("videos", self.counts.videos);
        counts.push("top_dirs", self.counts.top_dirs);
        root.push("counts", counts);

        root.push("media_bytes", self.sizes.media_bytes);

        let mut languages = Json::arr();
        for language in &self.languages {
            let mut item = Json::obj();
            item.push("name", language.name);
            item.push("files", language.files);
            languages.push_value(item);
        }
        root.push("languages", languages);

        let mut dirs = Json::arr();
        for dir in &self.dirs {
            let mut item = Json::obj();
            item.push("path", dir.path.clone());
            item.push("files", dir.files);
            item.push("images", dir.images);
            item.push("videos", dir.videos);
            item.push("bytes", dir.bytes);
            dirs.push_value(item);
        }
        root.push("dirs", dirs);

        let mut deps = Json::arr();
        for dep in &self.deps {
            deps.push_value(format!("{}:{}", dep.ecosystem, dep.name));
        }
        root.push("deps", deps);

        let mut entrypoints = Json::arr();
        for entry in &self.entrypoints {
            entrypoints.push_value(entry.clone());
        }
        root.push("entrypoints", entrypoints);
        root.push("tree", self.tree.clone());

        sha256::hex_digest(root.to_compact().as_bytes())
    }

    /// Compact three-line summary used by the LLM prompt.
    pub fn brief(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "name: {} | kind: {} | files: {} | dirs: {}",
            self.name,
            self.kind.as_str(),
            self.counts.files,
            self.counts.dirs
        ));
        if self.counts.images > 0 || self.counts.videos > 0 {
            lines.push(format!(
                "media: {} images, {} videos, {}",
                self.counts.images,
                self.counts.videos,
                self.media_human()
            ));
        }
        if !self.languages.is_empty() {
            let langs = self
                .languages
                .iter()
                .take(4)
                .map(|language| format!("{} ({})", language.name, language.files))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("languages: {langs}"));
        }
        if !self.dirs.is_empty() {
            let dirs = self
                .dirs
                .iter()
                .take(12)
                .map(|dir| format!("{} [{} files, {}]", dir.path, dir.files, dir.size_human()))
                .collect::<Vec<_>>()
                .join("; ");
            lines.push(format!("top dirs: {dirs}"));
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "readme-gen-test-{}-{}-{}",
            name,
            std::process::id(),
            crate::util::unix_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, content: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content).unwrap();
    }

    #[test]
    fn scans_a_small_tree() {
        let root = temp_dir("scan");
        write(&root.join("README.md"), b"# demo\n");
        write(&root.join("src/main.rs"), b"fn main() {}\n");
        write(&root.join("pics/a.jpg"), &[0u8; 2048]);
        write(&root.join("pics/b.png"), &[0u8; 1024]);
        write(&root.join("target/junk.rs"), b"ignored");
        fs::create_dir_all(root.join("tests")).unwrap();
        write(&root.join("tests/it.rs"), b"#[test] fn x() {}");

        let facts = scan(&ScanOptions {
            root: root.clone(),
            ..Default::default()
        })
        .unwrap();

        assert_eq!(facts.counts.images, 2);
        assert_eq!(facts.counts.videos, 0);
        assert_eq!(facts.sizes.media_bytes, 3072);
        assert_eq!(facts.dirs.len(), 3); // pics, src, tests
        assert!(facts.dirs.iter().any(|dir| dir.path == "pics" && dir.images == 2));
        assert!(facts.entrypoints.contains(&"src/main.rs".to_string()));
        assert_eq!(facts.kind, ProjectKind::Cli);
        assert!(facts.tree.contains("src/"));
        assert!(!facts.tree.contains("target"));
        assert!(!facts.hash.is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn hash_is_stable_and_excludes_readme() {
        let root = temp_dir("hash");
        write(&root.join("README.md"), b"# one\n");
        write(&root.join("main.py"), b"print('x')\n");
        let options = ScanOptions {
            root: root.clone(),
            ..Default::default()
        };
        let first = scan(&options).unwrap();
        write(&root.join("README.md"), b"# one, but longer now\n");
        let second = scan(&options).unwrap();
        assert_eq!(first.hash, second.hash);
        write(&root.join("helper.py"), b"def helper(): pass\n");
        let third = scan(&options).unwrap();
        assert_ne!(first.hash, third.hash);
        let _ = fs::remove_dir_all(&root);
    }
}
