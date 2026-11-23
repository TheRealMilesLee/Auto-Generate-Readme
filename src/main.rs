//! Command-line interface.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use readme_gen::adopt;
use readme_gen::annotations::Config;
use readme_gen::diff;
use readme_gen::error::{Error, Result};
use readme_gen::lang::{Lang, LocaleMode};
use readme_gen::llm;
use readme_gen::outline::{self, Preset, SectionKind};
use readme_gen::render::{self, BuildOptions, Ctx};
use readme_gen::scan::{self, Facts, ScanOptions};
use readme_gen::util::{clean_path, read_text, read_text_lossy, write_text_atomic};
use readme_gen::validate;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Scan,
    Plan,
    Build,
    Check,
    Init,
    Diff,
    Adopt,
    Doctor,
    Annotate,
    Help,
}

impl Command {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "scan" => Ok(Command::Scan),
            "plan" => Ok(Command::Plan),
            "build" => Ok(Command::Build),
            "check" => Ok(Command::Check),
            "init" => Ok(Command::Init),
            "diff" => Ok(Command::Diff),
            "adopt" => Ok(Command::Adopt),
            "doctor" => Ok(Command::Doctor),
            "annotate" => Ok(Command::Annotate),
            "help" => Ok(Command::Help),
            other => Err(Error::new(format!(
                "unknown command '{other}' (expected scan|plan|build|check|init|diff|adopt|doctor|annotate|help)"
            ))),
        }
    }
}

#[derive(Debug)]
struct Cli {
    command: Command,
    root: PathBuf,
    config: Option<PathBuf>,
    out: Option<PathBuf>,
    preset: Option<String>,
    langs: Option<Vec<Lang>>,
    locale_mode: Option<LocaleMode>,
    stdout: bool,
    dry_run: bool,
    strict: bool,
    json: bool,
    force: bool,
    quiet: bool,
    media_meta: bool,
    capture_help: Option<String>,
    excludes: Vec<String>,
    max_files: Option<usize>,
    llm: Option<String>,
    endpoint: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
    temperature: Option<f64>,
    max_tokens: Option<u32>,
    timeout: Option<u64>,
    no_cache: bool,
    no_thinking: bool,
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            command: Command::Help,
            root: PathBuf::from("."),
            config: None,
            out: None,
            preset: None,
            langs: None,
            locale_mode: None,
            stdout: false,
            dry_run: false,
            strict: false,
            json: false,
            force: false,
            quiet: false,
            media_meta: false,
            capture_help: None,
            excludes: Vec::new(),
            max_files: None,
            llm: None,
            endpoint: None,
            model: None,
            api_key: None,
            temperature: None,
            max_tokens: None,
            timeout: None,
            no_cache: false,
            no_thinking: false,
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = parse_args()?;
    match cli.command {
        Command::Help => {
            print_help();
            Ok(())
        }
        Command::Scan => command_scan(&cli),
        Command::Plan => command_plan(&cli),
        Command::Build => command_build(&cli),
        Command::Check => command_check(&cli),
        Command::Init => command_init(&cli),
        Command::Diff => command_diff(&cli),
        Command::Adopt => command_adopt(&cli),
        Command::Doctor => command_doctor(&cli),
        Command::Annotate => command_annotate(&cli),
    }
}

fn parse_args() -> Result<Cli> {
    let mut cli = Cli::default();
    let mut command: Option<Command> = None;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        if arg == "--" {
            continue;
        }
        if arg == "-h" || arg == "--help" {
            return Ok(Cli {
                command: Command::Help,
                ..Cli::default()
            });
        }
        if arg == "-V" || arg == "--version" {
            println!("readme-gen {VERSION}");
            std::process::exit(0);
        }

        let (flag, inline) = match arg.split_once('=') {
            Some((flag, value)) => (flag.to_string(), Some(value.to_string())),
            None => (arg.clone(), None),
        };

        macro_rules! value {
            () => {
                match &inline {
                    Some(value) => value.clone(),
                    None => args.next().ok_or_else(|| {
                        Error::new(format!("{flag} requires a value"))
                    })?,
                }
            };
        }

        match flag.as_str() {
            "--root" => cli.root = PathBuf::from(value!()),
            "--config" => cli.config = Some(PathBuf::from(value!())),
            "--out" => cli.out = Some(PathBuf::from(value!())),
            "--preset" => cli.preset = Some(value!()),
            "--lang" | "--locale" => cli.langs = Some(parse_langs(&value!())?),
            "--locale-mode" => {
                let raw = value!();
                cli.locale_mode = Some(LocaleMode::parse(&raw).ok_or_else(|| {
                    Error::new(format!("unknown locale-mode '{raw}' (single|split)"))
                })?);
            }
            "--capture-help" => cli.capture_help = Some(value!()),
            "--exclude" => cli.excludes.push(value!()),
            "--max-files" => {
                cli.max_files = Some(value!().parse().map_err(|_| {
                    Error::new("--max-files expects a number")
                })?)
            }
            "--llm" => cli.llm = Some(value!()),
            "--endpoint" => cli.endpoint = Some(value!()),
            "--model" => cli.model = Some(value!()),
            "--api-key" => cli.api_key = Some(value!()),
            "--temperature" => {
                cli.temperature = Some(value!().parse().map_err(|_| {
                    Error::new("--temperature expects a number")
                })?)
            }
            "--max-tokens" => {
                cli.max_tokens = Some(value!().parse().map_err(|_| {
                    Error::new("--max-tokens expects a number")
                })?)
            }
            "--timeout" => {
                cli.timeout = Some(value!().parse().map_err(|_| {
                    Error::new("--timeout expects seconds")
                })?)
            }
            "--stdout" => cli.stdout = true,
            "--dry-run" => cli.dry_run = true,
            "--strict" => cli.strict = true,
            "--json" => cli.json = true,
            "--force" => cli.force = true,
            "--quiet" => cli.quiet = true,
            "--media-meta" => cli.media_meta = true,
            "--no-cache" => cli.no_cache = true,
            "--no-thinking" => cli.no_thinking = true,
            other if other.starts_with('-') => {
                return Err(Error::new(format!(
                    "unknown option '{other}' (try --help)"
                )))
            }
            other => {
                if command.is_some() {
                    return Err(Error::new(format!("unexpected argument '{other}'")));
                }
                command = Some(Command::parse(other)?);
            }
        }
    }

    cli.command = command.unwrap_or(Command::Help);
    Ok(cli)
}

fn parse_langs(value: &str) -> Result<Vec<Lang>> {
    let mut out = Vec::new();
    for part in value.split(',') {
        let lang = Lang::parse(part).ok_or_else(|| {
            Error::new(format!(
                "unsupported locale '{}' (use en or zh)",
                part.trim()
            ))
        })?;
        if !out.contains(&lang) {
            out.push(lang);
        }
    }
    if out.is_empty() {
        return Err(Error::new("--lang requires at least one locale"));
    }
    out.sort();
    Ok(out)
}

fn command_scan(cli: &Cli) -> Result<()> {
    let root = absolute(&cli.root)?;
    let config = load_config(cli, &root)?;
    let extra = cli
        .out
        .as_ref()
        .map(|out| absolute(out))
        .transpose()?
        .map(|out| exclude_relative(&root, &out))
        .unwrap_or_default();
    let facts = scan(&root, cli, &config, &extra)?;

    let json = facts.to_json().to_pretty();
    match &cli.out {
        Some(path) => {
            write_text_atomic(path, &json)?;
            if !cli.quiet {
                eprintln!("wrote {}", path.display());
            }
        }
        None => println!("{json}"),
    }

    if !cli.quiet {
        eprintln!(
            "{} files, {} folders, {} images, {} videos, {} (facts {})",
            facts.counts.files,
            facts.counts.top_dirs,
            facts.counts.images,
            facts.counts.videos,
            facts.media_human(),
            &facts.hash[..12]
        );
    }
    Ok(())
}

fn command_plan(cli: &Cli) -> Result<()> {
    let root = absolute(&cli.root)?;
    let config = load_config(cli, &root)?;
    let facts = scan(&root, cli, &config, &[])?;
    let preset = resolve_preset(cli, &config, &facts)?;

    if cli.json {
        println!("{}", outline::plan_json(preset).to_pretty());
        return Ok(());
    }

    println!("preset: {}", preset.as_str());
    for spec in outline::sections(preset) {
        if spec.is_header() {
            continue;
        }
        println!(
            "  {:<14} {} / {}",
            spec.id,
            outline::title(spec.kind, Lang::En),
            outline::title(spec.kind, Lang::Zh)
        );
    }
    Ok(())
}

fn render_outputs(
    cli: &Cli,
    root: &Path,
    out: &Path,
    config: &Config,
    facts: &Facts,
    preset: Preset,
    locales: &[Lang],
    locale_mode: LocaleMode,
    allow_llm: bool,
) -> Result<Vec<(Lang, PathBuf, String)>> {
    let target_paths = split_target_paths(out, locales, locale_mode);
    let mut keeps = BTreeMap::new();
    for (_, path) in &target_paths {
        if path.exists() {
            if let Ok(existing) = read_text_lossy(path) {
                for (k, v) in render::extract_keeps(&existing) {
                    keeps.insert(k, v);
                }
            }
        }
    }

    let mut generated: BTreeMap<(String, Lang), String> = BTreeMap::new();
    if allow_llm {
        if let Some(options) = llm_options(cli, config, root) {
            let prose_ids: Vec<&str> = outline::sections(preset)
                .into_iter()
                .filter_map(|spec| match spec.kind {
                    SectionKind::Prose(id) => Some(id),
                    _ => None,
                })
                .collect();
            match llm::generate(facts, config, locales, &prose_ids, &options) {
                Ok(output) => {
                    generated = output.generated;
                    for warning in output.warnings {
                        eprintln!("warning: {warning}");
                    }
                }
                Err(error) => eprintln!("warning: llm disabled for this build: {error}"),
            }
        }
    }

    let captured_usage = capture_help_text(cli, config, root);
    let ctx = Ctx {
        facts,
        config,
        generated: &generated,
        keeps: &keeps,
        locales,
        preset,
        version: VERSION,
    };

    if locale_mode == LocaleMode::Split && locales.len() > 1 {
        let dir = out.parent().unwrap_or(root);
        let files = render::build_split_files(&ctx, captured_usage.as_deref());
        Ok(files
            .into_iter()
            .enumerate()
            .map(|(idx, (lang, filename, content))| {
                let path = if idx == 0 {
                    out.to_path_buf()
                } else {
                    dir.join(filename)
                };
                (lang, path, content)
            })
            .collect())
    } else {
        let text = render::build_with_options(
            &ctx,
            &BuildOptions {
                captured_usage: captured_usage.as_deref(),
                locale_mode: LocaleMode::Single,
            },
        );
        let primary = locales.first().copied().unwrap_or(Lang::En);
        Ok(vec![(primary, out.to_path_buf(), text)])
    }
}

fn split_target_paths(out: &Path, locales: &[Lang], mode: LocaleMode) -> Vec<(Lang, PathBuf)> {
    if mode == LocaleMode::Split && locales.len() > 1 {
        let dir = out.parent().unwrap_or_else(|| Path::new("."));
        locales
            .iter()
            .enumerate()
            .map(|(idx, &lang)| {
                let path = if idx == 0 {
                    out.to_path_buf()
                } else {
                    dir.join(format!("README.{}.md", lang.as_str()))
                };
                (lang, path)
            })
            .collect()
    } else {
        vec![(locales.first().copied().unwrap_or(Lang::En), out.to_path_buf())]
    }
}

fn capture_help_text(cli: &Cli, config: &Config, root: &Path) -> Option<String> {
    let cmd_str = cli
        .capture_help
        .as_deref()
        .or(config.usage.capture.as_deref())?;
    let parts: Vec<&str> = cmd_str.split_whitespace().collect();
    let (&program, args) = parts.split_first()?;
    match std::process::Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if stdout.trim().is_empty() {
                Some(String::from_utf8_lossy(&output.stderr).to_string())
            } else {
                Some(stdout)
            }
        }
        Ok(output) => {
            eprintln!(
                "warning: usage capture '{cmd_str}' exited with {}",
                output.status
            );
            None
        }
        Err(err) => {
            eprintln!("warning: cannot run usage capture '{cmd_str}': {err}");
            None
        }
    }
}

fn command_build(cli: &Cli) -> Result<()> {
    let root = absolute(&cli.root)?;
    let config = load_config(cli, &root)?;
    let out = cli
        .out
        .clone()
        .unwrap_or_else(|| root.join("README.md"));
    let extra = exclude_relative(&root, &absolute(&out)?);
    let facts = scan(&root, cli, &config, &extra)?;
    let preset = resolve_preset(cli, &config, &facts)?;
    let locales = resolve_locales(cli, &config);
    let locale_mode = cli.locale_mode.unwrap_or(config.locale_mode);

    let outputs = render_outputs(
        cli,
        &root,
        &out,
        &config,
        &facts,
        preset,
        &locales,
        locale_mode,
        true,
    )?;

    if cli.stdout || cli.dry_run {
        for (idx, (_, path, text)) in outputs.iter().enumerate() {
            if outputs.len() > 1 {
                if idx > 0 {
                    println!();
                }
                println!("<!-- file: {} -->", path.display());
            }
            print!("{text}");
            if cli.dry_run && !cli.quiet {
                eprintln!(
                    "dry run: would write {} ({} bytes, facts {})",
                    path.display(),
                    text.len(),
                    &facts.hash[..12]
                );
            }
        }
        return Ok(());
    }

    for (_, path, text) in &outputs {
        write_text_atomic(path, text)?;
        if !cli.quiet {
            eprintln!(
                "wrote {} ({} bytes, preset {}, facts {})",
                path.display(),
                text.len(),
                preset.as_str(),
                &facts.hash[..12]
            );
        }
    }
    Ok(())
}

fn command_check(cli: &Cli) -> Result<()> {
    let root = absolute(&cli.root)?;
    let config = load_config(cli, &root)?;
    let out = cli
        .out
        .clone()
        .unwrap_or_else(|| root.join("README.md"));
    let extra = exclude_relative(&root, &absolute(&out)?);
    let facts = scan(&root, cli, &config, &extra)?;
    if !out.exists() {
        return Err(Error::new(format!(
            "{} does not exist; run `readme-gen build` first",
            out.display()
        )));
    }
    let text = read_text(&out)?;
    let locales = if let Some(langs) = &cli.langs {
        langs.clone()
    } else if !config.locales.is_empty() {
        config.locales.clone()
    } else {
        render::extract_meta_locales(&text).unwrap_or_else(|| vec![Lang::En])
    };
    let locale_mode = cli.locale_mode.unwrap_or_else(|| {
        if cli.config.is_none() && find_config(&root).is_none() {
            render::extract_meta(&text)
                .and_then(|m| {
                    m.get("locale_mode")
                        .and_then(|v| v.as_str())
                        .and_then(LocaleMode::parse)
                })
                .unwrap_or(config.locale_mode)
        } else {
            config.locale_mode
        }
    });

    let mut check_config = config.clone();
    check_config.locale_mode = locale_mode;
    if let Some(preset_str) = &cli.preset {
        let preset = Preset::parse(preset_str).ok_or_else(|| {
            Error::new(format!(
                "unknown preset '{preset_str}' (library|cli|app|dataset|showcase)"
            ))
        })?;
        check_config.preset = Some(preset);
    }

    let report = if locale_mode == LocaleMode::Split && locales.len() > 1 {
        let targets = split_target_paths(&out, &locales, LocaleMode::Split);
        let mut contents: Vec<(Lang, String, PathBuf)> = Vec::new();
        for (lang, path) in targets {
            if !path.exists() {
                return Err(Error::new(format!(
                    "{} does not exist; run `readme-gen build` first",
                    path.display()
                )));
            }
            let body = read_text(&path)?;
            contents.push((lang, body, path));
        }
        let borrowed: Vec<(Lang, &str, &Path)> = contents
            .iter()
            .map(|(l, s, p)| (*l, s.as_str(), p.as_path()))
            .collect();
        validate::validate_split(&borrowed, &facts, &check_config)
    } else {
        validate::validate(&text, &facts, &check_config, &locales, &out)
    };
    report.print();

    if !report.is_ok() || (cli.strict && !report.warnings.is_empty()) {
        std::process::exit(1);
    }
    Ok(())
}

fn command_diff(cli: &Cli) -> Result<()> {
    let root = absolute(&cli.root)?;
    let config = load_config(cli, &root)?;
    let out = cli
        .out
        .clone()
        .unwrap_or_else(|| root.join("README.md"));
    let extra = exclude_relative(&root, &absolute(&out)?);
    let facts = scan(&root, cli, &config, &extra)?;
    let preset = resolve_preset(cli, &config, &facts)?;
    let locales = resolve_locales(cli, &config);
    let locale_mode = cli.locale_mode.unwrap_or(config.locale_mode);

    let outputs = render_outputs(
        cli,
        &root,
        &out,
        &config,
        &facts,
        preset,
        &locales,
        locale_mode,
        true,
    )?;

    let mut any_diff = false;
    for (_, path, new_text) in outputs {
        let old_text = if path.exists() {
            read_text_lossy(&path)?
        } else {
            String::new()
        };
        let label = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        if let Some(patch) = diff::unified_diff(
            &format!("a/{label}"),
            &format!("b/{label}"),
            &old_text,
            &new_text,
            3,
        ) {
            print!("{patch}");
            any_diff = true;
        }
    }
    if !any_diff && !cli.quiet {
        eprintln!("no changes");
    }
    Ok(())
}

fn command_adopt(cli: &Cli) -> Result<()> {
    let root = absolute(&cli.root)?;
    let out = cli
        .out
        .clone()
        .unwrap_or_else(|| root.join("README.md"));
    if !out.exists() {
        return Err(Error::new(format!("{} does not exist", out.display())));
    }
    let existing = read_text(&out)?;
    let config = load_config(cli, &root)?;
    let extra = exclude_relative(&root, &absolute(&out)?);
    let facts = scan(&root, cli, &config, &extra)?;
    let preset = resolve_preset(cli, &config, &facts)?;

    let default_lang = cli
        .langs
        .as_ref()
        .and_then(|l| l.first().copied())
        .or_else(|| config.locales.first().copied())
        .unwrap_or(Lang::En);
    let plan = adopt::analyze(&existing, default_lang);
    let locales = cli
        .langs
        .clone()
        .unwrap_or_else(|| plan.locales.clone());

    let cfg_path = cli
        .config
        .clone()
        .unwrap_or_else(|| root.join("readme.yml"));

    if cli.stdout || cli.dry_run {
        let mut adopted_config = config.clone();
        adopted_config.preset = Some(preset);
        adopted_config.locales = locales.clone();
        let rendered = render::build(&Ctx {
            facts: &facts,
            config: &adopted_config,
            generated: &BTreeMap::new(),
            keeps: &plan.keeps,
            locales: &locales,
            preset,
            version: VERSION,
        });
        print!("{rendered}");
        if !cli.quiet {
            eprintln!(
                "adopted {} keep block(s) across [{}]",
                plan.keeps.len(),
                locales
                    .iter()
                    .map(|l| l.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        return Ok(());
    }

    if !cfg_path.exists() || cli.force {
        let template = init_template(&facts, preset, &locales);
        write_text_atomic(&cfg_path, &template)?;
        if !cli.quiet {
            eprintln!("wrote {}", cfg_path.display());
        }
    }

    let adopted_config = load_config(cli, &root)?;
    let facts = scan(&root, cli, &adopted_config, &extra)?;
    let preset = resolve_preset(cli, &adopted_config, &facts)?;
    let rendered = render::build(&Ctx {
        facts: &facts,
        config: &adopted_config,
        generated: &BTreeMap::new(),
        keeps: &plan.keeps,
        locales: &locales,
        preset,
        version: VERSION,
    });
    write_text_atomic(&out, &rendered)?;
    if !cli.quiet {
        eprintln!(
            "adopted {} ({} keep block(s), preset {})",
            out.display(),
            plan.keeps.len(),
            preset.as_str()
        );
    }
    Ok(())
}

fn command_doctor(cli: &Cli) -> Result<()> {
    let root = absolute(&cli.root)?;
    let cfg_path = cli
        .config
        .clone()
        .or_else(|| find_config(&root));
    let config = load_config(cli, &root)?;
    let out = cli
        .out
        .clone()
        .unwrap_or_else(|| root.join("README.md"));
    let extra = exclude_relative(&root, &absolute(&out)?);
    let facts = scan(&root, cli, &config, &extra)?;

    println!("root: {}", root.display());
    match &cfg_path {
        Some(p) if p.exists() => println!("config: {}", p.display()),
        _ => println!("config: none (using defaults; run `readme-gen init` to create readme.yml)"),
    }
    println!(
        "facts: {} files, {} folders, {} images, {} videos, hash {}",
        facts.counts.files,
        facts.counts.top_dirs,
        facts.counts.images,
        facts.counts.videos,
        &facts.hash[..12]
    );

    let mut report = validate::Report::default();
    validate::check_rules(&facts, &config, &mut report);
    validate::check_config_hygiene(&facts, &config, &mut report);

    if out.exists() {
        let text = read_text(&out)?;
        let locales = if let Some(langs) = &cli.langs {
            langs.clone()
        } else if !config.locales.is_empty() {
            config.locales.clone()
        } else {
            render::extract_meta_locales(&text).unwrap_or_else(|| vec![Lang::En])
        };
        let doc_report = validate::validate(&text, &facts, &config, &locales, &out);
        for err in doc_report.errors {
            if !report.errors.contains(&err) {
                report.error(err);
            }
        }
        for warn in doc_report.warnings {
            if !report.warnings.contains(&warn) {
                report.warning(warn);
            }
        }
    } else {
        report.warning(format!("{} does not exist yet", out.display()));
    }

    if let Some(options) = llm_options(cli, &config, &root) {
        match llm::probe(&options) {
            Ok(status) => println!("llm: {status}"),
            Err(err) => report.warning(format!("llm probe failed: {err}")),
        }
    }

    report.print();
    if !report.is_ok() || (cli.strict && !report.warnings.is_empty()) {
        std::process::exit(1);
    }
    Ok(())
}

fn command_annotate(cli: &Cli) -> Result<()> {
    let root = absolute(&cli.root)?;
    let config = load_config(cli, &root)?;
    let facts = scan(&root, cli, &config, &[])?;
    let locales = resolve_locales(cli, &config);
    let options = llm_options(cli, &config, &root).ok_or_else(|| {
        Error::new("annotate requires an LLM endpoint (pass --llm llama.cpp or --llm ollama)")
    })?;

    if facts.dirs.is_empty() {
        if !cli.quiet {
            eprintln!("no top-level directories to annotate");
        }
        return Ok(());
    }

    let (drafted, warnings) = llm::annotate_dirs(&facts, &config, &locales, cli.force, &options)?;
    for w in warnings {
        eprintln!("warning: {w}");
    }
    if drafted.is_empty() {
        if !cli.quiet {
            eprintln!("all directories already annotated (pass --force to regenerate)");
        }
        return Ok(());
    }

    let mut yaml_block = String::new();
    for (dir, by_lang) in &drafted {
        yaml_block.push_str(&format!("  {}:\n", yaml_string(dir)));
        yaml_block.push_str("    summary:\n");
        for &lang in &locales {
            if let Some(text) = by_lang.get(&lang) {
                yaml_block.push_str(&format!(
                    "      {}: {}\n",
                    lang.as_str(),
                    yaml_string(text)
                ));
            }
        }
    }

    if cli.stdout || cli.dry_run {
        println!("dirs:\n{yaml_block}");
        return Ok(());
    }

    let cfg_path = cli
        .config
        .clone()
        .unwrap_or_else(|| root.join("readme.yml"));
    let existing = if cfg_path.exists() {
        read_text(&cfg_path)?
    } else {
        let preset = resolve_preset(cli, &config, &facts)?;
        init_template(&facts, preset, &locales)
    };

    let updated = if existing.lines().any(|line| line.trim() == "dirs:") {
        let mut out_lines = Vec::new();
        let mut inserted = false;
        for line in existing.lines() {
            out_lines.push(line.to_string());
            if !inserted && line.trim() == "dirs:" {
                out_lines.push(yaml_block.trim_end().to_string());
                inserted = true;
            }
        }
        out_lines.join("\n") + "\n"
    } else {
        format!("{}\ndirs:\n{yaml_block}", existing.trim_end())
    };

    write_text_atomic(&cfg_path, &updated)?;
    if !cli.quiet {
        eprintln!("annotated {} folder(s) in {}", drafted.len(), cfg_path.display());
    }
    Ok(())
}

fn command_init(cli: &Cli) -> Result<()> {
    let root = absolute(&cli.root)?;
    let config = load_config(cli, &root)?;
    let facts = scan(&root, cli, &config, &[])?;
    let preset = resolve_preset(cli, &config, &facts)?;
    let locales = if let Some(langs) = &cli.langs {
        langs.clone()
    } else if !config.locales.is_empty() {
        config.locales.clone()
    } else {
        vec![Lang::En]
    };

    let path = cli
        .config
        .clone()
        .unwrap_or_else(|| root.join("readme.yml"));
    if path.exists() && !cli.force {
        return Err(Error::new(format!(
            "{} already exists; pass --force to overwrite",
            path.display()
        )));
    }

    let template = init_template(&facts, preset, &locales);
    write_text_atomic(&path, &template)?;
    if !cli.quiet {
        eprintln!("wrote {}", path.display());
        eprintln!("next: fill in tagline/sections, then run `readme-gen build`");
    }
    Ok(())
}

fn scan(
    root: &Path,
    cli: &Cli,
    config: &Config,
    extra_excludes: &[String],
) -> Result<Facts> {
    let mut excludes = config.excludes.clone();
    excludes.extend(cli.excludes.iter().cloned());
    excludes.extend(extra_excludes.iter().cloned());
    scan::scan(&ScanOptions {
        root: root.to_path_buf(),
        excludes,
        catalog_depth: 1,
        max_depth: 16,
        max_files: cli.max_files.unwrap_or(200_000),
        tree_depth: 2,
        media_meta: cli.media_meta || config.scan.media_meta,
    })
}

fn load_config(cli: &Cli, root: &Path) -> Result<Config> {
    let path = match &cli.config {
        Some(path) => Some(path.clone()),
        None => find_config(root),
    };
    match path {
        Some(path) if path.exists() => Config::load(&path),
        Some(path) if cli.config.is_some() => Err(Error::new(format!(
            "config not found: {}",
            path.display()
        ))),
        _ => Ok(Config::default()),
    }
}

fn find_config(root: &Path) -> Option<PathBuf> {
    for name in ["readme.yml", "readme.yaml", ".readme.yml"] {
        let path = root.join(name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn resolve_preset(cli: &Cli, config: &Config, facts: &Facts) -> Result<Preset> {
    if let Some(value) = &cli.preset {
        return Preset::parse(value).ok_or_else(|| {
            Error::new(format!(
                "unknown preset '{value}' (library|cli|app|dataset|showcase)"
            ))
        });
    }
    if let Some(preset) = config.preset {
        return Ok(preset);
    }
    Ok(Preset::from_kind(facts.kind))
}

fn resolve_locales(cli: &Cli, config: &Config) -> Vec<Lang> {
    if let Some(langs) = &cli.langs {
        return langs.clone();
    }
    if !config.locales.is_empty() {
        return config.locales.clone();
    }
    vec![Lang::En]
}

fn llm_options(cli: &Cli, config: &Config, root: &Path) -> Option<llm::Options> {
    let provider = cli.llm.as_deref();
    let endpoint = cli
        .endpoint
        .clone()
        .or_else(|| config.llm.endpoint.clone())
        .or_else(|| match provider {
            Some("llama.cpp") | Some("llamacpp") | Some("llama") => {
                Some("http://127.0.0.1:8080".to_string())
            }
            Some("ollama") => Some("http://127.0.0.1:11434".to_string()),
            Some(value) if value.starts_with("http://") => Some(value.to_string()),
            _ => None,
        });

    let endpoint = endpoint?;
    if let Some(provider) = provider {
        if provider == "none" && cli.endpoint.is_none() {
            return None;
        }
    }

    let model = cli
        .model
        .clone()
        .or_else(|| config.llm.model.clone())
        .or_else(|| match provider {
            Some("ollama") => Some("qwen2.5:7b-instruct".to_string()),
            _ => Some("local".to_string()),
        })
        .unwrap_or_else(|| "local".to_string());

    let temperature = cli
        .temperature
        .or(config.llm.temperature)
        .unwrap_or(0.2);
    let max_tokens = cli.max_tokens.unwrap_or(500);
    let timeout = std::time::Duration::from_secs(cli.timeout.unwrap_or(180));

    Some(llm::Options {
        endpoint,
        model,
        api_key: cli.api_key.clone(),
        temperature,
        max_tokens,
        timeout,
        cache_dir: root.join(".readme/cache/llm"),
        no_cache: cli.no_cache,
        no_thinking: cli.no_thinking || config.llm.no_thinking,
    })
}

fn init_template(facts: &Facts, preset: Preset, locales: &[Lang]) -> String {
    let mut out = String::new();
    out.push_str("# readme-gen configuration. Numbers are scanned; write meaning here.\n");
    out.push_str("# Docs: docs/CONFIGURATION.md\n");
    out.push_str(&format!("preset: {}\n", preset.as_str()));
    out.push_str(&format!(
        "locale: [{}]\n",
        locales
            .iter()
            .map(|lang| lang.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    out.push('\n');
    out.push_str("project:\n");
    out.push_str(&format!("  name: {}\n", yaml_string(&facts.name)));
    out.push_str("  tagline:\n");
    for lang in locales {
        let value = if *lang == Lang::En && !facts.description.is_empty() {
            yaml_string(&facts.description)
        } else {
            "\"\"".to_string()
        };
        out.push_str(&format!("    {}: {}\n", lang.as_str(), value));
    }
    out.push_str("  description:\n");
    for lang in locales {
        out.push_str(&format!("    {}: \"\"\n", lang.as_str()));
    }
    out.push('\n');

    if let Some((owner, repo)) = facts.repo_slug() {
        out.push_str("quickstart:\n");
        out.push_str(&format!(
            "  clone: {}\n",
            yaml_string(&format!("https://github.com/{owner}/{repo}.git"))
        ));
        out.push('\n');
    }

    if matches!(preset, Preset::Dataset | Preset::Showcase) && !facts.dirs.is_empty() {
        out.push_str("dirs:\n");
        for dir in facts.dirs.iter().take(40) {
            out.push_str(&format!("  {}:\n", yaml_string(&dir.path)));
            out.push_str("    summary:\n");
            out.push_str("      en: \"\"\n");
            out.push_str("    # bullets:\n");
            out.push_str("    #   - en: \"\"\n");
        }
        out.push('\n');
    }

    out.push_str("sections:\n");
    out.push_str("  why:\n");
    for lang in locales {
        out.push_str(&format!("    {}: \"\"\n", lang.as_str()));
    }
    if preset == Preset::Showcase {
        out.push_str("  audience:\n");
        for lang in locales {
            out.push_str(&format!("    {}: \"\"\n", lang.as_str()));
        }
    }
    out.push_str("\n# Optional: announcements, setup, tooling, sources, compatibility, awards, changelog, cta.\n");
    out.push_str("# sources:\n#   - name: Example\n#     url: https://example.com\n#     note:\n#       en: \"\"\n");
    out
}

fn yaml_string(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', " ")
            .replace('\r', " ")
    )
}

fn exclude_relative(root: &Path, out: &Path) -> Vec<String> {
    match out.strip_prefix(root) {
        Ok(relative) if !relative.as_os_str().is_empty() => {
            vec![relative
                .to_string_lossy()
                .replace('\\', "/")]
        }
        _ => Vec::new(),
    }
}

fn absolute(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(clean_path(path))
    } else {
        Ok(clean_path(&std::env::current_dir()?.join(path)))
    }
}

fn print_help() {
    println!(
        r#"readme-gen {VERSION}
Generate verifiable README files from repository facts.

USAGE:
    readme-gen <command> [options]

COMMANDS:
    scan      Scan a repository and write facts as JSON
    plan      Show the section plan for a preset
    build     Generate README.md (deterministic, optional LLM prose)
    check     Validate README.md against facts and rules
    diff      Show unified diff between disk and freshly built output
    adopt     Wrap an existing README.md into keep blocks + readme.yml
    doctor    Diagnose readme.yml hygiene, rules, README, and LLM server
    annotate  Draft folder summaries in readme.yml using local LLM
    init      Create a readme.yml skeleton
    help      Show this message

OPTIONS:
    --root PATH            Repository root (default: .)
    --config PATH          readme.yml path (default: <root>/readme.yml)
    --out PATH             Output path (default: <root>/README.md)
    --preset NAME          library | cli | app | dataset | showcase
    --lang LIST            Comma-separated locales: en,zh
    --locale-mode MODE     single | split (README.md + README.<lang>.md)
    --media-meta           Probe image headers for resolution statistics
    --capture-help CMD     Run CMD to populate Usage when not authored
    --exclude GLOB         Extra ignore pattern (repeatable)
    --max-files N          Scan safety limit (default: 200000)
    --stdout               Print the document instead of writing
    --dry-run              Print the document and do not write
    --strict               Treat warnings as errors (check, doctor)
    --json                 JSON output (scan, plan)
    --force                Overwrite existing readme.yml / annotations
    --quiet                Suppress summaries

LLM (optional, for prose sections & annotate):
    --llm PROVIDER     none | llama.cpp | ollama | http://host:port
    --endpoint URL     OpenAI-compatible endpoint
    --model NAME       Model name (default: local, or qwen2.5:7b-instruct for Ollama)
    --api-key KEY      Bearer token
    --temperature N    Sampling temperature (default: 0.2)
    --max-tokens N     Max tokens per section (default: 500)
    --timeout SECS     HTTP timeout (default: 180)
    --no-cache         Disable the LLM response cache
    --no-thinking      Send enable_thinking=false in chat_template_kwargs

MODELS:
    Recommended: Qwen2.5-7B-Instruct Q5_K_M (no thinking, fits 8 GB VRAM).
    Newer:      Qwen3-8B Q5_K_M with --reasoning-budget 0 on the server.
    Full guide: docs/MODELS.md

EXAMPLES:
    readme-gen scan --root . --out .readme/facts.json
    readme-gen init --preset showcase --lang en,zh
    readme-gen build --llm llama.cpp --model local
    readme-gen check --strict
"#
    );
}
