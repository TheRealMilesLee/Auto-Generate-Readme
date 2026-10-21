# Architecture

This document explains the engineering decisions behind `readme-gen`. The
product design (why facts, annotations and sections are separate) lives in
[DESIGN-v2.md](DESIGN-v2.md); this file is about how the implementation works.

## 1. Language decision

Requirements:

- runs on macOS (development), Linux and Windows (Git Bash and PowerShell);
- one artifact, no Python/Node runtime on the target machine;
- fast enough to scan a 10 GiB / 3,000-file media repository in seconds;
- optional local LLM without turning the build into a CUDA/CMake project;
- buildable offline, without a paid CI service.

| Option | Single binary | Cross-platform | Build complexity | Scan speed | Verdict |
|---|---|---|---|---|---|
| Python | ✗ (needs interpreter) | good | low | adequate | rejected: deployment and startup cost |
| C++ | ✓ | good | high (vcpkg/CMake per OS) | excellent | rejected: dependency and packaging burden |
| Go | ✓ | excellent | low | excellent | viable, but weaker type-level modelling for this data pipeline |
| **Rust** | **✓** | **excellent** | **low (std only)** | **excellent** | **chosen** |

Rust 2021, MSRV 1.74. The crate has **zero external dependencies**: `Cargo.toml`
contains no `[dependencies]` table. That is a deliberate constraint:

- `cargo build --release` works offline on all three platforms;
- no OpenSSL, no C/C++ libraries, no system packages (the platform linker is
  still used for the final binary: Xcode CLT on macOS, MSVC Build Tools or
  MinGW on Windows, `cc` on Linux);
- a fresh Windows machine with `rustup` needs nothing else;
- the resulting `.exe` / binary is self-contained.

The cost is that small pieces are implemented in-tree and tested here: JSON,
a YAML subset, SHA-256, gitignore-style globs, an HTTP client, and the renderer.
All of them are covered by unit tests (`cargo test`).

## 2. Why llama.cpp is a server, not a binding

`llama.cpp` is used through `llama-server`, which exposes an OpenAI-compatible
`/v1/chat/completions` endpoint. Ollama exposes the same shape.

Reasons:

1. **Build simplicity.** Linking `llama.cpp` would require CMake, a C++ compiler
   and platform-specific GPU flags on every target. The generator stays pure Rust.
2. **Performance.** `llama-server` keeps the model and KV cache loaded; each
   section is a short request instead of a cold model load.
3. **Isolation.** A model crash cannot take down the build; the CLI reports a
   warning and falls back to deterministic output.
4. **Portability.** The same binary works with Ollama, llama.cpp, or any
   OpenAI-compatible gateway; only the endpoint changes.

The client intentionally supports **`http://` only**. Local inference endpoints
are plain HTTP; refusing HTTPS keeps the binary free of a TLS stack. If a remote
endpoint is ever needed, put a reverse proxy in front of it.

## 3. Pipeline

```
repo ──scan──▶ facts.json ──outline──▶ section graph ──render──▶ README.md
                    │                        │                        │
                    │                        ├── annotations (readme.yml)
                    │                        └── LLM prose (optional)
                    └── hash embedded in the document ◀── check ──────┘
```

Each stage is a library module and can be tested alone:

| Module | Responsibility |
|---|---|
| `scan.rs` | walk the tree, classify files, aggregate directory stats, read manifests, detect git/licence/tests, render the tree |
| `annotations.rs` | parse `readme.yml` into typed structures |
| `outline.rs` | presets and the section graph |
| `render.rs` | deterministic renderers, TOC, bilingual assembly, metadata, keep blocks |
| `llm.rs` | OpenAI-compatible client, prompt contract, cache, retry, validation |
| `validate.rs` | Markdown structure, links, anchors, markers, number tracing, drift |
| `json.rs` / `yaml.rs` / `sha256.rs` / `glob.rs` / `util.rs` | in-tree primitives |

### Facts are metadata-only

The walker calls `symlink_metadata`, never `read()`, for regular files. It reads
content only for:

- `Cargo.toml`, `package.json`, `pyproject.toml`, `requirements.txt`, `go.mod`;
- a licence file (first 8 KiB);
- `.gitignore` (patterns).

That is why scanning 3,000 files and 10 GiB of media takes milliseconds instead
of minutes: no file is opened, `du` is never invoked.

### The content hash

`Facts::content_hash()` covers the inputs that change document content: counts,
media bytes, directory aggregates, languages, dependencies, entrypoints and the
tree. It deliberately excludes:

- the README itself (writing the document changes its size);
- git history (committing the document changes the latest commit);
- total working-tree bytes (same reason).

The hash is embedded as `<!-- readme-gen:meta {"facts":"…"} -->`. `check`
recomputes the facts and fails when the hash no longer matches, which is how the
image-count drift in the wallpaper repository would have been caught.

## 4. Determinism and ownership

- Deterministic sections are pure functions of `Facts` + `Config`; two builds
  from the same inputs are byte-identical (tested).
- The document is always UTF-8 with LF line endings, on every platform.
- Generated sections are wrapped in
  `<!-- readme-gen:section:{lang}:{id}:start -->` markers.
- Hand-written overrides live in
  `<!-- readme-gen:keep:{lang}:{id}:start -->` blocks. On rebuild the keep block
  replaces the generated body **and is written back into the output**, so the
  override survives indefinitely.
- `check` verifies marker balance and bilingual parity.

## 5. Cross-platform behaviour

| Concern | Handling |
|---|---|
| Path separators | `PathBuf` internally; forward slashes in Markdown/tree output |
| Unicode names | `OsString` → `String` (lossy only for invalid UTF-8); UTF-8 output |
| Line endings | input normalised to LF; output is LF (Git `core.autocrlf` may convert on checkout, the file content stays LF) |
| Console encoding | Rust writes Unicode through the Windows console API; no ANSI colours are emitted |
| Symlinks / junctions | skipped by default (prevents cycles and duplicated trees) |
| Long paths | `std::fs` uses the wide Win32 API; no 260-character `MAX_PATH` assumption in our code |
| Shell dependencies | none for scanning; `git` is optional and invoked without a shell |
| Case sensitivity | extensions lowercased; link checks use the filesystem |
| Git Bash paths | native binary receives converted paths; use `MSYS_NO_PATHCONV=1` for literal POSIX paths |

No `tree`, `find`, `du` or `sed` is invoked. The only optional external process
is `git -C <root> …`.

### Building

- **macOS / Linux:** install Rust with rustup, then `cargo build --release`.
- **Windows (MSVC):** rustup + Visual Studio Build Tools (C++ workload);
  `cargo build --release` in PowerShell or Git Bash.
- **Windows (GNU):** rustup toolchain `stable-x86_64-pc-windows-gnu` with MinGW-w64.

The output is `target/release/readme-gen` (`readme-gen.exe` on Windows).

## 6. LLM integration

The model is optional and only writes prose. On the reference machine
(Ryzen 7 7735HS, 32 GB DDR5, RTX 4070 Laptop 8 GB), a 7–8B model at `Q5_K_M`
fits fully in VRAM; sizes, flags and alternatives are in
[MODELS.md](MODELS.md).

```
# terminal A: keep the model resident
llama-server -m models/Qwen2.5-7B-Instruct-Q5_K_M.gguf \
  --host 127.0.0.1 --port 8080 -ngl 99 -c 8192 -fa on --jinja

# terminal B
readme-gen build --llm llama.cpp --model local
```

`--model` is a label for a single-model `llama-server`; with Ollama it selects
the model (`--llm ollama --model qwen2.5:7b-instruct`). The client does not send
a thinking flag, so prefer a non-thinking model or disable reasoning at the
server (`--reasoning-budget 0` in recent llama.cpp builds, or a chat-template
kwarg). A thinking model that writes its reasoning into the section will be
rejected by the validator.

The client sends `POST /v1/chat/completions` with:

- a system prompt that forbids invented facts, headings and code fences;
- a compact fact sheet (`Facts::brief()`), taglines, and annotated folder
  summaries — never the whole repository, and never file contents;
- `temperature` (default 0.2) and `max_tokens` (default 500).

Every response goes through the same number tracer as `check`. If a number is
not present in facts or `readme.yml`, the section is retried once with the
validator's complaint, then dropped with a warning. The build never fails
because a model misbehaved.

Responses are cached under `.readme/cache/llm/<sha256>.md`, keyed by prompt
version, facts hash, section id, language, model and endpoint. Rebuilding after
an unrelated edit does not call the model again.

## 7. Performance

Measured on the reference repository (macOS, Apple silicon):

| Operation | Result |
|---|---|
| scan 2,717 files / 5.8 GiB (no file reads) | 0.10 s warm, 1.0 s cold |
| deterministic build | 0.06 s for an 11.7 KiB bilingual document |
| LLM prose | model-dependent; seconds per section, cached after the first run (see [MODELS.md](MODELS.md)) |

The scanner is single-threaded. At this scale the syscall cost is tiny and
parallelism would add nondeterminism; if a repository with 500k files ever
matters, the walk is trivially parallelisable per top-level directory.

## 8. Validation rules

`readme-gen check` fails on:

- missing or stale facts hash;
- unresolved `{{…}}` placeholders;
- unclosed code fences / section markers / keep blocks;
- broken relative links and images;
- duplicate anchors;
- bilingual section mismatch;
- numbers not traceable to facts or `readme.yml`.

It warns on heading-level jumps, missing anchor targets, and malformed markers.
`--strict` promotes warnings to failures.

## 9. Known limitations

- `readme.yml` supports a YAML subset: no anchors, aliases, tags or flow maps.
- The LLM client is HTTP-only and non-streaming, and does not control the
  model's thinking mode; disable it at the server or use a non-thinking model.
- Only Markdown is emitted; HTML/JSON output is not implemented.
- Symlinked directories are skipped.
- Number tracing allows years, semver-like versions, resolutions and `4K`/`1080p`
  suffixes without a facts lookup; everything else must be approved.
- Git history is optional; without `git`, the changelog section is omitted.
