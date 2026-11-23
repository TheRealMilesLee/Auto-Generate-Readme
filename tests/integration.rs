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

#[test]
fn split_bilingual_build_and_validate() {
    use readme_gen::lang::LocaleMode;
    let dir = fixture("split-bilingual");
    let facts = scan::scan(&options(dir.path())).unwrap();
    let mut config = Config::default();
    config.locale_mode = LocaleMode::Split;
    config.locales = vec![Lang::En, Lang::Zh];
    config
        .sections
        .insert("why".to_string(), Loc::both("Fast and clean.", "又快又干净。"));

    let files = render::build_split_files(
        &Ctx {
            facts: &facts,
            config: &config,
            generated: &Default::default(),
            keeps: &Default::default(),
            locales: &[Lang::En, Lang::Zh],
            preset: readme_gen::outline::Preset::Cli,
            version: "0.1.0",
        },
        None,
    );
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].1, "README.md");
    assert_eq!(files[1].1, "README.zh.md");
    assert!(files[0].2.contains("README.zh.md"));
    assert!(files[1].2.contains("README.md"));

    dir.write("README.md", files[0].2.as_bytes());
    dir.write("README.zh.md", files[1].2.as_bytes());

    let p_en = dir.path().join("README.md");
    let p_zh = dir.path().join("README.zh.md");
    let report = validate::validate_split(
        &[
            (Lang::En, files[0].2.as_str(), p_en.as_path()),
            (Lang::Zh, files[1].2.as_str(), p_zh.as_path()),
        ],
        &facts,
        &config,
    );
    assert!(report.is_ok(), "{:?}", report.errors);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

#[test]
fn rules_and_hygiene_catch_violations() {
    let dir = fixture("rules");
    dir.write("nested/sub/pic.png", &[0u8; 64]);
    fs::create_dir_all(dir.path().join("empty_shelf")).unwrap();

    let facts = scan::scan(&options(dir.path())).unwrap();
    let mut config = Config::default();
    config.rules.max_dir_depth = Some(1);
    config.rules.forbid_exts = vec!["png".to_string()];
    config.rules.no_empty_dirs = true;
    config.unknown_keys.push("bogus_key".to_string());
    config
        .dirs
        .insert("ghost_dir".to_string(), Default::default());

    let mut report = validate::Report::default();
    validate::check_rules(&facts, &config, &mut report);
    validate::check_config_hygiene(&facts, &config, &mut report);

    assert!(report.errors.iter().any(|e| e.contains("max_dir_depth")));
    assert!(report.errors.iter().any(|e| e.contains(".png")));
    assert!(report.errors.iter().any(|e| e.contains("empty_shelf")));
    assert!(report.warnings.iter().any(|w| w.contains("bogus_key")));
    assert!(report.warnings.iter().any(|w| w.contains("ghost_dir")));
}

#[test]
fn media_meta_probes_png_headers() {
    let dir = fixture("media-meta");
    let mut png = Vec::new();
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    png.extend_from_slice(&13u32.to_be_bytes());
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(&3840u32.to_be_bytes());
    png.extend_from_slice(&2160u32.to_be_bytes());
    dir.write("wallpapers/hero.png", &png);

    let facts = scan::scan(&ScanOptions {
        root: dir.path().to_path_buf(),
        media_meta: true,
        ..Default::default()
    })
    .unwrap();

    let media = facts.media.expect("media summary should be present");
    assert_eq!(media.measured_images, 1);
    assert_eq!(media.max_width, 3840);
    assert_eq!(media.max_height, 2160);
    assert_eq!(media.uhd_4k, 1);
}

#[test]
fn workspace_scan_discovers_cargo_members() {
    let dir = TempDir::new("workspace");
    dir.write(
        "Cargo.toml",
        b"[workspace]\nmembers = [\"crates/*\"]\n",
    );
    dir.write(
        "crates/alpha/Cargo.toml",
        b"[package]\nname = \"alpha\"\nversion = \"0.2.0\"\ndescription = \"Alpha crate\"\n\n[dependencies]\nfoo = \"1.0\"\n",
    );
    dir.write("crates/alpha/src/lib.rs", b"pub fn a() {}\n");

    let facts = scan::scan(&options(dir.path())).unwrap();
    assert_eq!(facts.workspaces.len(), 1);
    assert_eq!(facts.workspaces[0].name, "alpha");
    assert_eq!(facts.workspaces[0].version.as_deref(), Some("0.2.0"));
    assert!(facts.deps.iter().any(|d| d.name == "foo"));
}

#[test]
fn diff_adopt_and_doctor_cli_end_to_end() {
    let dir = fixture("cli-extra");
    dir.write(
        "README.md",
        b"# demo\n\n## Usage\n\nCustom legacy usage instructions.\n",
    );
    let root = dir.path().to_str().unwrap();
    let binary = env!("CARGO_BIN_EXE_readme-gen");

    let adopt = Command::new(binary)
        .args(["adopt", "--root", root])
        .output()
        .unwrap();
    assert!(
        adopt.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&adopt.stderr)
    );
    let adopted_readme = fs::read_to_string(dir.path().join("README.md")).unwrap();
    assert!(adopted_readme.contains("<!-- readme-gen:keep:en:usage:start -->"));
    assert!(adopted_readme.contains("Custom legacy usage instructions."));
    assert!(dir.path().join("readme.yml").exists());

    let diff = Command::new(binary)
        .args(["diff", "--root", root])
        .output()
        .unwrap();
    assert!(diff.status.success());

    let doctor = Command::new(binary)
        .args(["doctor", "--root", root])
        .output()
        .unwrap();
    assert!(
        doctor.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&doctor.stdout),
        String::from_utf8_lossy(&doctor.stderr)
    );
}
