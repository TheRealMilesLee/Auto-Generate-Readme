//! End-to-end tests for scanning, generation, validation and keep blocks.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use readme_gen::annotations::Config;
use readme_gen::lang::{Lang, Loc};
use readme_gen::render::{self, Ctx};
use readme_gen::scan::{self, ScanOptions};
use readme_gen::validate;

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "readme-gen-it-{}-{}-{}",
            name,
            std::process::id(),
            readme_gen::util::unix_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, rel: &str, content: &[u8]) {
        let path = self.path.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn fixture(name: &str) -> TempDir {
    let dir = TempDir::new(name);
    dir.write(
        "Cargo.toml",
        b"[package]\nname = \"demo\"\nversion = \"0.1.0\"\ndescription = \"A demo CLI\"\nlicense = \"MIT\"\n",
    );
    dir.write(
        "LICENSE",
        b"MIT License\n\nPermission is hereby granted, free of charge...\n",
    );
    dir.write("README.md", b"# demo\n");
    dir.write("src/main.rs", b"fn main() { println!(\"hi\"); }\n");
    dir.write("tests/basic.rs", b"#[test]\nfn works() {}\n");
    dir.write("docs/guide.md", b"# Guide\n");
    dir
}

fn options(root: &Path) -> ScanOptions {
    ScanOptions {
        root: root.to_path_buf(),
        ..Default::default()
    }
}

#[test]
fn scans_project_metadata() {
    let dir = fixture("scan");
    let facts = scan::scan(&options(dir.path())).unwrap();

    assert_eq!(facts.name, "demo");
    assert_eq!(facts.version.as_deref(), Some("0.1.0"));
    assert_eq!(facts.license.as_deref(), Some("MIT"));
    assert_eq!(facts.kind, readme_gen::scan::ProjectKind::Cli);
    assert!(facts.entrypoints.contains(&"src/main.rs".to_string()));
    assert!(facts.tests.is_some());
    assert!(facts.tree.contains("src/"));
    assert_eq!(facts.hash.len(), 64);
}

#[test]
fn deterministic_build_is_stable_and_valid() {
    let dir = fixture("build");
    let facts = scan::scan(&options(dir.path())).unwrap();
    let config = Config::default();
    let locales = vec![Lang::En];
    let generated = Default::default();
    let keeps = Default::default();

    let build = || {
        render::build(&Ctx {
            facts: &facts,
            config: &config,
            generated: &generated,
            keeps: &keeps,
            locales: &locales,
            preset: readme_gen::outline::Preset::Cli,
            version: "0.1.0",
        })
    };

    let first = build();
    let second = build();
    assert_eq!(first, second);
    assert!(first.contains("## Quick start"));
    assert!(first.contains("## Installation"));
    assert!(first.contains("cargo build --release"));
    assert!(first.contains("cargo run --release -- --help"));
    assert!(!first.contains("{{"));

    let report = validate::validate(&first, &facts, &config, &locales, &dir.path().join("README.md"));
    assert!(report.is_ok(), "{:?}", report.errors);
}

#[test]
fn bilingual_build_passes_parity_check() {
    let dir = fixture("bilingual");
    let facts = scan::scan(&options(dir.path())).unwrap();
    let mut config = Config::default();
    config.sections.insert(
        "why".to_string(),
        Loc::both("Because it is small.", "因为它足够小。"),
    );
    config.sections.insert(
        "audience".to_string(),
        Loc::both("For people who read READMEs.", "写给会读 README 的人。"),
    );
    let locales = vec![Lang::En, Lang::Zh];
    let generated = Default::default();
    let keeps = Default::default();

    let text = render::build(&Ctx {
        facts: &facts,
        config: &config,
        generated: &generated,
        keeps: &keeps,
        locales: &locales,
        preset: readme_gen::outline::Preset::Showcase,
        version: "0.1.0",
    });

    assert!(text.contains("<a id=\"english\"></a>"));
    assert!(text.contains("<a id=\"chinese\"></a>"));
    assert!(text.contains("<!-- readme-gen:section:zh:why:start -->"));
    assert!(text.contains("因为它足够小。"));

    let report = validate::validate(&text, &facts, &config, &locales, &dir.path().join("README.md"));
    assert!(report.is_ok(), "{:?}", report.errors);
}

#[test]
fn keeps_survive_rebuilds() {
    let dir = fixture("keeps");
    let facts = scan::scan(&options(dir.path())).unwrap();
    let config = Config::default();
    let locales = vec![Lang::En];
    let generated = Default::default();

    let first = render::build(&Ctx {
        facts: &facts,
        config: &config,
        generated: &generated,
        keeps: &Default::default(),
        locales: &locales,
        preset: readme_gen::outline::Preset::Cli,
        version: "0.1.0",
    });

    let kept = first.replace(
        "## Usage",
        "<!-- readme-gen:keep:en:usage:start -->\n## Usage\n\nHand-written usage.\n<!-- readme-gen:keep:en:usage:end -->\n\n## Ignored",
    );
    let keeps = render::extract_keeps(&kept);

    let second = render::build(&Ctx {
        facts: &facts,
        config: &config,
        generated: &generated,
        keeps: &keeps,
        locales: &locales,
        preset: readme_gen::outline::Preset::Cli,
        version: "0.1.0",
    });

    assert!(second.contains("Hand-written usage."));
    assert!(second.contains("<!-- readme-gen:keep:en:usage:start -->"));
    assert_eq!(
        second.matches("## Usage").count(),
        1,
        "keep block heading must not be duplicated:\n{second}"
    );
}

#[test]
fn check_detects_stale_facts() {
    let dir = fixture("stale");
    let facts = scan::scan(&options(dir.path())).unwrap();
    let text = render::build(&Ctx {
        facts: &facts,
        config: &Config::default(),
        generated: &Default::default(),
        keeps: &Default::default(),
        locales: &[Lang::En],
        preset: readme_gen::outline::Preset::Cli,
        version: "0.1.0",
    });

    // A new source file changes the facts hash.
    dir.write("src/extra.rs", b"pub fn extra() {}\n");
    let new_facts = scan::scan(&options(dir.path())).unwrap();
    assert_ne!(facts.hash, new_facts.hash);

    let report = validate::validate(&text, &new_facts, &Config::default(), &[Lang::En], &dir.path().join("README.md"));
    assert!(!report.is_ok());
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("stale")));
}

#[test]
fn check_rejects_unknown_numbers() {
    let dir = fixture("numbers");
    let facts = scan::scan(&options(dir.path())).unwrap();
    let mut text = render::build(&Ctx {
        facts: &facts,
        config: &Config::default(),
        generated: &Default::default(),
        keeps: &Default::default(),
        locales: &[Lang::En],
        preset: readme_gen::outline::Preset::Cli,
        version: "0.1.0",
    });
    text.push_str("\nIt is 42 times faster.\n");

    let report = validate::validate(&text, &facts, &Config::default(), &[Lang::En], &dir.path().join("README.md"));
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("42")));
}

#[test]
fn binary_build_and_check_work_together() {
    let dir = fixture("binary");
    let out_path = dir.path().join("README.out.md");
    let out = out_path.to_str().unwrap();
    let root = dir.path().to_str().unwrap();
    let binary = env!("CARGO_BIN_EXE_readme-gen");

    let build = Command::new(binary)
        .args(["build", "--root", root, "--out", out])
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let check = Command::new(binary)
        .args(["check", "--root", root, "--out", out])
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}

#[test]
fn bilingual_headings_have_no_level_jumps() {
    let dir = fixture("bilingual-headings");
    let facts = scan::scan(&options(dir.path())).unwrap();
    let mut config = Config::default();
    config
        .sections
        .insert("why".to_string(), Loc::both("Clear reason.", "原因清晰。"));
    let locales = vec![Lang::En, Lang::Zh];

    let text = render::build(&Ctx {
        facts: &facts,
        config: &config,
        generated: &Default::default(),
        keeps: &Default::default(),
        locales: &locales,
        preset: readme_gen::outline::Preset::Cli,
        version: "0.1.0",
    });

    assert!(text.contains("## English"));
    assert!(text.contains("### Quick start"));
    assert!(text.contains("## 中文"));
    assert!(text.contains("### 快速开始"));

    let report = validate::validate(&text, &facts, &config, &locales, &dir.path().join("README.md"));
    assert!(report.is_ok(), "{:?}", report.errors);
    assert!(
        report.warnings.is_empty(),
        "unexpected warnings: {:?}",
        report.warnings
    );
}

#[test]
fn check_detects_config_drift() {
    let dir = fixture("config-drift");
    let facts = scan::scan(&options(dir.path())).unwrap();
    let mut config = Config::default();
    config.project.tagline = Loc::en("First tagline.");

    let text = render::build(&Ctx {
        facts: &facts,
        config: &config,
        generated: &Default::default(),
        keeps: &Default::default(),
        locales: &[Lang::En],
        preset: readme_gen::outline::Preset::Cli,
        version: "0.1.0",
    });

    config.project.tagline = Loc::en("Edited tagline.");
    let report = validate::validate(
        &text,
        &facts,
        &config,
        &[Lang::En],
        &dir.path().join("README.md"),
    );
    assert!(!report.is_ok());
    assert!(report.errors.iter().any(|e| e.contains("config hash")));
}

#[test]
fn showcase_features_and_multiplication_resolution_validate() {
    use readme_gen::annotations::{Announcement, CatalogGroup, DirAnn, SetupEntry, ToolEntry};
    let dir = fixture("showcase-rich");
    for folder in ["alpha", "beta", "gamma", "delta"] {
        dir.write(&format!("{folder}/pic.jpg"), &[0u8; 1024]);
    }
    let facts = scan::scan(&options(dir.path())).unwrap();
    let mut config = Config::default();
    config.preset = Some(readme_gen::outline::Preset::Showcase);
    config.locales = vec![Lang::En];
    config.announcements.push(Announcement {
        kind: "IMPORTANT".to_string(),
        date: Some("2026-11-20".to_string()),
        text: Loc::en("All cuts include 3840×2160."),
    });
    config.catalog_groups.push(CatalogGroup {
        name: Loc::en("Primary"),
        dirs: vec!["alpha".to_string(), "beta".to_string()],
    });
    config.dirs.insert(
        "alpha".to_string(),
        DirAnn {
            summary: Loc::en("Lead shelf in 3840×2160."),
            bullets: vec![],
            tags: vec![],
        },
    );
    config.setup.push(SetupEntry {
        target: Loc::en("4K"),
        recommend: Loc::en("Start with `alpha`"),
    });
    config.tooling.push(ToolEntry {
        command: "scripts/check.sh".to_string(),
        note: Loc::en("Run local validation"),
    });

    let text = render::build(&Ctx {
        facts: &facts,
        config: &config,
        generated: &Default::default(),
        keeps: &Default::default(),
        locales: &[Lang::En],
        preset: readme_gen::outline::Preset::Showcase,
        version: "0.1.0",
    });

    assert!(text.contains("> [!IMPORTANT]"));
    assert!(text.contains("### Primary"));
    assert!(text.contains("## Pick your setup"));
    assert!(text.contains("## Tooling"));

    let report = validate::validate(
        &text,
        &facts,
        &config,
        &[Lang::En],
        &dir.path().join("README.md"),
    );
    assert!(report.is_ok(), "{:?}", report.errors);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}
