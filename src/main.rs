//! Command-line interface.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use readme_gen::annotations::Config;
use readme_gen::error::{Error, Result};
use readme_gen::lang::Lang;
use readme_gen::llm;
use readme_gen::outline::{self, Preset, SectionKind};
use readme_gen::render::{self, Ctx};
use readme_gen::scan::{self, Facts, ScanOptions};
use readme_gen::util::{read_text, read_text_lossy, write_text_atomic};
use readme_gen::validate;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Scan,
    Plan,
    Build,
    Check,
    Init,
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
            "help" => Ok(Command::Help),
            other => Err(Error::new(format!(
                "unknown command '{other}' (expected scan|plan|build|check|init|help)"
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
    stdout: bool,
    dry_run: bool,
    strict: bool,
    json: bool,
    force: bool,
    quiet: bool,
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
            stdout: false,
            dry_run: false,
            strict: false,
            json: false,
            force: false,
            quiet: false,
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
            "--no-cache" => cli.no_cache = true,
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

    let keeps = if out.exists() {
        read_text_lossy(&out)
            .map(|text| render::extract_keeps(&text))
            .unwrap_or_default()
    } else {
        BTreeMap::new()
    };

    let mut generated: BTreeMap<(String, Lang), String> = BTreeMap::new();
    if let Some(options) = llm_options(cli, &config, &root) {
        let prose_ids: Vec<&str> = outline::sections(preset)
            .into_iter()
            .filter_map(|spec| match spec.kind {
                SectionKind::Prose(id) => Some(id),
                _ => None,
            })
            .collect();
        match llm::generate(&facts, &config, &locales, &prose_ids, &options) {
            Ok(output) => {
                generated = output.generated;
                for warning in output.warnings {
                    eprintln!("warning: {warning}");
                }
            }
            Err(error) => eprintln!("warning: llm disabled for this build: {error}"),
        }
    }

    let text = render::build(&Ctx {
        facts: &facts,
        config: &config,
        generated: &generated,
        keeps: &keeps,
        locales: &locales,
        preset,
        version: VERSION,
    });

    if cli.stdout || cli.dry_run {
        print!("{text}");
        if cli.dry_run && !cli.quiet {
            eprintln!(
                "dry run: would write {} ({} bytes, facts {})",
                out.display(),
                text.len(),
                &facts.hash[..12]
            );
        }
        return Ok(());
    }

    write_text_atomic(&out, &text)?;
    if !cli.quiet {
        eprintln!(
            "wrote {} ({} bytes, preset {}, facts {})",
            out.display(),
            text.len(),
            preset.as_str(),
            &facts.hash[..12]
        );
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
    let locales = render::extract_meta_locales(&text)
        .or_else(|| {
            if config.locales.is_empty() {
                None
            } else {
                Some(config.locales.clone())
            }
        })
        .unwrap_or_else(|| vec![Lang::En]);

    let report = validate::validate(&text, &facts, &config, &locales, &out);
    report.print();

    if !report.is_ok() || (cli.strict && !report.warnings.is_empty()) {
        std::process::exit(1);
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
            Some("ollama") => Some("qwen3:8b".to_string()),
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
    out.push_str("\n# Optional: sources, compatibility, awards, changelog.\n");
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
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
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
    check     Validate README.md against facts (CI-friendly)
    init      Create a readme.yml skeleton
    help      Show this message

OPTIONS:
    --root PATH        Repository root (default: .)
    --config PATH      readme.yml path (default: <root>/readme.yml)
    --out PATH         Output path (default: <root>/README.md)
    --preset NAME      library | cli | app | dataset | showcase
    --lang LIST        Comma-separated locales: en,zh
    --exclude GLOB     Extra ignore pattern (repeatable)
    --max-files N      Scan safety limit (default: 200000)
    --stdout           Print the document instead of writing
    --dry-run          Print the document and do not write
    --strict           Treat warnings as errors (check)
    --json             JSON output (scan, plan)
    --force            Overwrite existing readme.yml (init)
    --quiet            Suppress summaries

LLM (optional, for prose sections only):
    --llm PROVIDER     none | llama.cpp | ollama | http://host:port
    --endpoint URL     OpenAI-compatible endpoint
    --model NAME       Model name (default: local, or qwen3:8b for Ollama)
    --api-key KEY      Bearer token
    --temperature N    Sampling temperature (default: 0.2)
    --max-tokens N     Max tokens per section (default: 500)
    --timeout SECS     HTTP timeout (default: 180)
    --no-cache         Disable the LLM response cache

EXAMPLES:
    readme-gen scan --root . --out .readme/facts.json
    readme-gen init --preset showcase --lang en,zh
    readme-gen build --llm llama.cpp --model local
    readme-gen check --strict
"#
    );
}
