<!-- readme-gen:meta {"schema":1,"facts":"0b48bd016fc3454c3760b209d9491b478f21176202471641611718916d532df5","config":"09fc0b18e5070b80741755186d168061ad7a8757a7a10a75a8a1e0d0b0d93d9d","preset":"cli","locale_mode":"single","locales":["en","zh"],"generator":"readme-gen/0.1.0"} -->
# readme-gen

<p align="center">
  <strong>READMEs that cannot lie about their numbers.</strong><br>
  <sub>不会在数字上撒谎的 README 生成器。</sub><br>
  <sub>CLI tool · Rust · 33 files · MIT</sub>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/language-Rust-orange?style=for-the-badge" alt="Rust">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green?style=for-the-badge" alt="license"></a>
  <img src="https://img.shields.io/badge/version-0.1.0-blueviolet?style=for-the-badge" alt="version">
  <a href="https://github.com/TheRealMilesLee/Auto-Generate-Readme/stargazers"><img src="https://img.shields.io/github/stars/TheRealMilesLee/Auto-Generate-Readme?style=for-the-badge&logo=github" alt="stars"></a>
  <a href="https://github.com/TheRealMilesLee/Auto-Generate-Readme/network/members"><img src="https://img.shields.io/github/forks/TheRealMilesLee/Auto-Generate-Readme?style=for-the-badge&logo=github" alt="forks"></a>
</p>

<p align="center">
  <a href="https://github.com/TheRealMilesLee/Auto-Generate-Readme/stargazers"><img src="https://img.shields.io/badge/⭐_Star_this_repo-ff4785?style=for-the-badge&logo=github" alt="Star"></a>
  <a href="https://github.com/TheRealMilesLee/Auto-Generate-Readme/fork"><img src="https://img.shields.io/badge/🍴_Fork_%26_own_it-24292f?style=for-the-badge&logo=github" alt="Fork"></a>
</p>

<p align="center"><a href="#english">English</a> · <a href="#chinese">中文</a> · <a href="#quickstart">Quick start</a> · <a href="#why">Why</a> · <a href="#usage">Usage</a> · <a href="#tooling">Tooling</a> · <a href="#changelog">Update log</a></p>

> [!NOTE]
> **2026-11-23** · Feature suite update: added `diff`, `adopt`, `doctor`, `annotate`, `locale_mode: split`, `--media-meta` header dimension parsing, `rules:` repo invariants, Cargo/npm workspace discovery, `usage.capture`, and `--no-thinking`.
>
> **2026-11-23** · 功能全面升级：新增 `diff`、`adopt`、`doctor`、`annotate`、`locale_mode: split` 双语拆文件、`--media-meta` 图片头解析、`rules:` 仓库规则校验、Workspace 扫描、`usage.capture` 与 `--no-thinking`。

<a id="english"></a>
## English

### Contents

- [Quick start](#quickstart)
- [By the numbers](#numbers)
- [Why this project](#why)
- [Installation](#install)
- [Usage](#usage)
- [Project structure](#structure)
- [Tooling](#tooling)
- [Contributing](#contributing)
- [License](#license)
- [Update log](#changelog)
- [Star & fork](#cta)

<!-- readme-gen:section:en:quickstart:start -->
<a id="quickstart"></a>
### Quick start

Pick the path that matches how you want to use it.

| Action | What to do |
|---|---|
| **Bookmark** | [Star the repo](https://github.com/TheRealMilesLee/Auto-Generate-Readme/stargazers) |
| **Own a copy** | [Fork it](https://github.com/TheRealMilesLee/Auto-Generate-Readme/fork) |
| **Use locally** | Clone the repository |

```bash
git clone https://github.com/TheRealMilesLee/Auto-Generate-Readme.git
cd Auto-Generate-Readme

# optional shallow clone: faster, less history
git clone --depth 1 https://github.com/TheRealMilesLee/Auto-Generate-Readme.git
```
<!-- readme-gen:section:en:quickstart:end -->

<!-- readme-gen:section:en:numbers:start -->
<a id="numbers"></a>
### By the numbers

| Metric | Value |
|---|---|
| **Folders** | 4 |
| **Files** | 33 |
| **Languages** | Rust (19), PowerShell (1), Shell (1) |
| **Test files** | 1 |
| **External dependencies** | 0 (Rust std only, MSRV 1.74) |
| **Reference scan speed** | 0.10 s warm / 1.00 s cold (2,717 files, 5.8 GiB) |
<!-- readme-gen:section:en:numbers:end -->

<!-- readme-gen:section:en:why:start -->
<a id="why"></a>
### Why this project

READMEs rot for a simple reason: the numbers are written by hand. A hero
line that says "3,000 images" is correct on the day it is typed and wrong
three months later. Structure drifts the same way.

This tool splits the problem into three layers. **Facts** are scanned from
the repository — counts, sizes, languages, folders, dependencies, git.
**Meaning** lives in a reviewable `readme.yml`. **Rendering** is
deterministic. A rebuild is boring: same facts in, byte-identical document
out.

Prose is optional. A local `llama.cpp` server (or Ollama) can write the
narrative sections, but it never supplies a number: every digit is traced
back to the scan or to a human-authored annotation. If a model invents one,
the section is rejected, retried once, and then dropped.

| Manual README pain | What this tool does |
|---|---|
| Numbers & config go stale | Embeds facts + config hashes; `check` fails on drift |
| Structure is reinvented per project | Five presets share one section graph |
| AI invents numbers | Every digit is traced to facts or `readme.yml` |
| Regeneration overwrites edits | `keep` blocks survive rebuilds without duplicating headings |

Everything runs locally: `cargo test` plus `scripts/check.sh`. No hosted CI
is required.

The reference benchmark is a multi-gigabyte wallpaper archive with dozens
of folders and thousands of images. That README is excellent because it is
*specific*; this tool makes the specific parts reproducible and the
tasteful parts editable.
Start with [the architecture](docs/ARCHITECTURE.md),
[the configuration reference](docs/CONFIGURATION.md),
[the design notes](docs/DESIGN-v2.md), and
[the benchmarks](docs/BENCHMARKS.md).
<!-- readme-gen:section:en:why:end -->

<!-- readme-gen:section:en:install:start -->
<a id="install"></a>
### Installation

Build from source with the commands for your platform.

```bash
cargo build --release
cargo install --path .
```
<!-- readme-gen:section:en:install:end -->

<!-- readme-gen:section:en:usage:start -->
<a id="usage"></a>
### Usage

Scan first, then generate and validate:

```bash
readme-gen scan --out .readme/facts.json --media-meta
readme-gen plan --preset showcase
readme-gen init --preset showcase --lang en,zh
readme-gen build
readme-gen diff
readme-gen check --strict
readme-gen doctor
```

Existing repository with a hand-written README? Wrap it safely:

```bash
readme-gen adopt
```

Choose from five built-in presets (`library`, `cli`, `app`, `dataset`, `showcase`):

| Preset | Best for | Key sections |
|---|---|---|
| `library` | Crates, Python/JS/Go packages | `numbers`, `why`, `install`, `usage`, `deps`, `license` |
| `cli` | Command-line tools | `quickstart`, `numbers`, `why`, `install`, `usage`, `tooling`, `changelog` |
| `app` | Desktop / web applications | `quickstart`, `numbers`, `why`, `install`, `structure`, `license` |
| `dataset` | Media / data repositories | `quickstart`, `numbers`, `catalog`, `setup`, `sources`, `changelog` |
| `showcase` | Curated collections (benchmark) | `packs`, `why`, `audience`, `catalog` (grouped), `compatibility`, `awards`, `cta` |

To let a local model write the prose sections (a 7–8B model is enough):

```bash
llama-server -m models/Qwen2.5-7B-Instruct-Q5_K_M.gguf \
  --host 127.0.0.1 --port 8080 -ngl 99 -c 8192 -fa on --jinja
readme-gen build --llm llama.cpp --model local
```

See [the model guide](docs/MODELS.md) for 8 GB / 32 GB hardware profiles,
quantisation and alternatives.

`check` is the local gate: it verifies links, anchors, markers, bilingual parity,
facts/config hashes, and that every number in the document exists in the facts or in this file.
<!-- readme-gen:section:en:usage:end -->

<!-- readme-gen:section:en:structure:start -->
<a id="structure"></a>
### Project structure

```text
Auto-Generate-Readme/
├── docs/
│   ├── ARCHITECTURE.md
│   ├── BENCHMARKS.md
│   ├── CONFIGURATION.md
│   ├── DESIGN-v2.md
│   └── MODELS.md
├── scripts/
│   ├── check.ps1
│   └── check.sh
├── src/
│   ├── adopt.rs
│   ├── annotations.rs
│   ├── diff.rs
│   ├── error.rs
│   ├── glob.rs
│   ├── json.rs
│   ├── lang.rs
│   ├── lib.rs
│   ├── llm.rs
│   ├── main.rs
│   ├── media.rs
│   ├── outline.rs
│   ├── render.rs
│   ├── scan.rs
│   ├── sha256.rs
│   ├── util.rs
│   ├── validate.rs
│   └── yaml.rs
├── tests/
│   └── integration.rs
├── .gitattributes
├── .gitignore
├── Cargo.lock
├── Cargo.toml
├── CONTRIBUTING.md
├── LICENSE
└── readme.yml
```
<!-- readme-gen:section:en:structure:end -->

<!-- readme-gen:section:en:tooling:start -->
<a id="tooling"></a>
### Tooling

| Command / script | What it does |
|---|---|
| `scripts/check.sh` | Local quality gate (macOS / Linux): `cargo fmt --check`, `clippy`, `cargo test`, release build, `readme-gen check --strict`. |
| `scripts/check.ps1` | Local quality gate for Windows (PowerShell), matching `check.sh` without hosted CI. |
<!-- readme-gen:section:en:tooling:end -->

<!-- readme-gen:section:en:contributing:start -->
<a id="contributing"></a>
### Contributing

Issues and pull requests are welcome. Please keep changes small and focused, and run the test suite before opening a PR.

- [Open an issue](https://github.com/TheRealMilesLee/Auto-Generate-Readme/issues)
- [Open a pull request](https://github.com/TheRealMilesLee/Auto-Generate-Readme/pulls)

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.
<!-- readme-gen:section:en:contributing:end -->

<!-- readme-gen:section:en:license:start -->
<a id="license"></a>
### License

Released under the **MIT** license. See [`LICENSE`](LICENSE).
<!-- readme-gen:section:en:license:end -->

<!-- readme-gen:section:en:changelog:start -->
<a id="changelog"></a>
### Update log

| When | What changed |
|---|---|
| **2026-11-23** | Added `diff`, `adopt`, `doctor`, `annotate`, split bilingual files (`locale_mode: split`), `--media-meta` header parsing, `rules:` invariants, workspace scanning, `usage.capture`, and `--no-thinking`; fixed `keep` heading duplication, `×` resolution tracing, and config-aware caching. |
| **2026-10-21** | Complete rewrite in Rust: deterministic facts, section presets, validator, keep blocks, and optional llama.cpp prose. The Python and Bash implementations moved to legacy/. |
<!-- readme-gen:section:en:changelog:end -->

<!-- readme-gen:section:en:cta:start -->
<a id="cta"></a>
### Star & fork

If `readme-gen` keeps your documentation honest without manual counting, leave a Star or Fork a copy.

<p align="center">
  <a href="https://github.com/TheRealMilesLee/Auto-Generate-Readme/stargazers"><img src="https://img.shields.io/badge/⭐_Star-ff4785?style=for-the-badge" alt="Star"></a>
  <a href="https://github.com/TheRealMilesLee/Auto-Generate-Readme/fork"><img src="https://img.shields.io/badge/🍴_Fork-24292f?style=for-the-badge" alt="Fork"></a>
</p>
<!-- readme-gen:section:en:cta:end -->

---

<a id="chinese"></a>
## 中文

### 目录

- [快速开始](#zh-quickstart)
- [数据一览](#zh-numbers)
- [为什么值得](#zh-why)
- [安装](#zh-install)
- [使用](#zh-usage)
- [项目结构](#zh-structure)
- [工具脚本](#zh-tooling)
- [贡献](#zh-contributing)
- [许可证](#zh-license)
- [更新记录](#zh-changelog)
- [Star 与 Fork](#zh-cta)

<!-- readme-gen:section:zh:quickstart:start -->
<a id="zh-quickstart"></a>
### 快速开始

按你的使用方式选一条路径。

| 操作 | 说明 |
|---|---|
| **收藏** | [点 Star](https://github.com/TheRealMilesLee/Auto-Generate-Readme/stargazers) |
| **自己留一份** | [Fork 到自己的账号](https://github.com/TheRealMilesLee/Auto-Generate-Readme/fork) |
| **本地使用** | 克隆仓库 |

```bash
git clone https://github.com/TheRealMilesLee/Auto-Generate-Readme.git
cd Auto-Generate-Readme

# 可选浅克隆: 更快，历史更少
git clone --depth 1 https://github.com/TheRealMilesLee/Auto-Generate-Readme.git
```
<!-- readme-gen:section:zh:quickstart:end -->

<!-- readme-gen:section:zh:numbers:start -->
<a id="zh-numbers"></a>
### 数据一览

| 指标 | 数值 |
|---|---|
| **目录** | 4 |
| **文件** | 33 |
| **语言** | Rust (19), PowerShell (1), Shell (1) |
| **测试文件** | 1 |
| **外部依赖** | 0（仅 Rust 标准库，MSRV 1.74） |
| **标杆扫描性能** | 热缓存 0.10 s / 冷启动 1.00 s（2,717 文件，5.8 GiB） |
<!-- readme-gen:section:zh:numbers:end -->

<!-- readme-gen:section:zh:why:start -->
<a id="zh-why"></a>
### 为什么值得

README 会过期，原因很朴素：数字是手写的。首页写着「3,000 张图片」，写下的
那天是对的，三个月后就是错的；结构也会以同样的方式漂移。

这个工具把问题拆成三层。**事实**由仓库扫描得到——数量、体积、语言、目录、
依赖、git；**意义**放在可评审的 `readme.yml`；**渲染**是确定性的。于是重建
变得很无聊：同样的输入，产出逐字节一致。

散文是可选的。本地 `llama.cpp` 服务（或 Ollama）可以撰写叙事章节，但它不
提供任何数字：每个数字都能追溯到扫描结果或人工注解。模型若编造数字，该章节
会被拒绝、重试一次，然后丢弃。

| 手工 README 的痛点 | 这个工具的做法 |
|---|---|
| 数字或配置会过期 | 写入事实与配置双 hash，`check` 在漂移时失败 |
| 每个项目重造结构 | 五个 preset 共用一张章节图 |
| 模型编造数字 | 每个数字都追溯到事实或 `readme.yml` |
| 重新生成覆盖手写内容 | `keep` 区块跨重建保留且不重复标题 |

一切都在本地完成：`cargo test` 与 `scripts/check.sh`，不依赖任何托管 CI/CD。

参照样本是一份数 GiB、几十个目录、数千张图片的壁纸归档。那份 README 之所以
好，是因为它足够具体；这个工具让具体的部分可复现，让有品味的部分可编辑。可以
先读 [架构说明](docs/ARCHITECTURE.md)、[配置参考](docs/CONFIGURATION.md)、
[设计提案](docs/DESIGN-v2.md) 与 [性能基准](docs/BENCHMARKS.md)。
<!-- readme-gen:section:zh:why:end -->

<!-- readme-gen:section:zh:install:start -->
<a id="zh-install"></a>
### 安装

按所在平台从源码构建。

```bash
cargo build --release
cargo install --path .
```
<!-- readme-gen:section:zh:install:end -->

<!-- readme-gen:section:zh:usage:start -->
<a id="zh-usage"></a>
### 使用

先扫描，再规划、生成和校验：

```bash
readme-gen scan --out .readme/facts.json --media-meta
readme-gen plan --preset showcase
readme-gen init --preset showcase --lang en,zh
readme-gen build
readme-gen diff
readme-gen check --strict
readme-gen doctor
```

已有手写 README？一条命令接管并保留现有内容：

```bash
readme-gen adopt
```

内置五个 preset（`library`、`cli`、`app`、`dataset`、`showcase`）：

| Preset | 适用场景 | 核心章节 |
|---|---|---|
| `library` | Rust / Python / JS / Go 库 | `numbers`、`why`、`install`、`usage`、`deps`、`license` |
| `cli` | 命令行工具 | `quickstart`、`numbers`、`why`、`install`、`usage`、`tooling`、`changelog` |
| `app` | 桌面 / Web 应用 | `quickstart`、`numbers`、`why`、`install`、`structure`、`license` |
| `dataset` | 媒体 / 数据集仓库 | `quickstart`、`numbers`、`catalog`、`setup`、`sources`、`changelog` |
| `showcase` | 策展型收藏仓库（标杆级别） | `packs`、`why`、`audience`、`catalog`（分组总览）、`compatibility`、`awards`、`cta` |

允许本地模型撰写散文（7–8B 已经够用）：

```bash
llama-server -m models/Qwen2.5-7B-Instruct-Q5_K_M.gguf \
  --host 127.0.0.1 --port 8080 -ngl 99 -c 8192 -fa on --jinja
readme-gen build --llm llama.cpp --model local
```

模型大小、量化、显存预算与替代方案见 [模型指南](docs/MODELS.md)。

`check` 是本地质量闸门：它检查链接、锚点、标记、双语对齐、事实/配置 hash，
以及文档里的每个数字是否都存在于事实或本文件中。
<!-- readme-gen:section:zh:usage:end -->

<!-- readme-gen:section:zh:structure:start -->
<a id="zh-structure"></a>
### 项目结构

```text
Auto-Generate-Readme/
├── docs/
│   ├── ARCHITECTURE.md
│   ├── BENCHMARKS.md
│   ├── CONFIGURATION.md
│   ├── DESIGN-v2.md
│   └── MODELS.md
├── scripts/
│   ├── check.ps1
│   └── check.sh
├── src/
│   ├── adopt.rs
│   ├── annotations.rs
│   ├── diff.rs
│   ├── error.rs
│   ├── glob.rs
│   ├── json.rs
│   ├── lang.rs
│   ├── lib.rs
│   ├── llm.rs
│   ├── main.rs
│   ├── media.rs
│   ├── outline.rs
│   ├── render.rs
│   ├── scan.rs
│   ├── sha256.rs
│   ├── util.rs
│   ├── validate.rs
│   └── yaml.rs
├── tests/
│   └── integration.rs
├── .gitattributes
├── .gitignore
├── Cargo.lock
├── Cargo.toml
├── CONTRIBUTING.md
├── LICENSE
└── readme.yml
```
<!-- readme-gen:section:zh:structure:end -->

<!-- readme-gen:section:zh:tooling:start -->
<a id="zh-tooling"></a>
### 工具脚本

| 命令 / 脚本 | 作用 |
|---|---|
| `scripts/check.sh` | 本地质量门禁（macOS / Linux）：格式化检查、`clippy`、`cargo test`、release 构建与 `readme-gen check --strict`。 |
| `scripts/check.ps1` | Windows（PowerShell）本地质量门禁，与 `check.sh` 等价，不依赖托管 CI。 |
<!-- readme-gen:section:zh:tooling:end -->

<!-- readme-gen:section:zh:contributing:start -->
<a id="zh-contributing"></a>
### 贡献

欢迎提交 Issue 和 Pull Request。请保持改动小而聚焦，并在提交前跑完测试。

- [提交 Issue](https://github.com/TheRealMilesLee/Auto-Generate-Readme/issues)
- [提交 PR](https://github.com/TheRealMilesLee/Auto-Generate-Readme/pulls)

详见 [CONTRIBUTING.md](CONTRIBUTING.md)。
<!-- readme-gen:section:zh:contributing:end -->

<!-- readme-gen:section:zh:license:start -->
<a id="zh-license"></a>
### 许可证

本项目采用 **MIT** 许可证。详见 [`LICENSE`](LICENSE)。
<!-- readme-gen:section:zh:license:end -->

<!-- readme-gen:section:zh:changelog:start -->
<a id="zh-changelog"></a>
### 更新记录

| 时间 | 变更 |
|---|---|
| **2026-11-23** | 新增 `diff`、`adopt`、`doctor`、`annotate`、双语拆文件（`locale_mode: split`）、`--media-meta` 图片头解析、`rules:` 仓库规则校验、Workspace 扫描、`usage.capture` 与 `--no-thinking`；修复 `keep` 标题重复、`×` 分辨率溯源与缓存失效问题。 |
| **2026-10-21** | 用 Rust 完整重写：事实扫描、章节预设、校验器、keep 区块，以及可选的 llama.cpp 散文生成。Python 与 Bash 实现移入 legacy/。 |
<!-- readme-gen:section:zh:changelog:end -->

<!-- readme-gen:section:zh:cta:start -->
<a id="zh-cta"></a>
### Star 与 Fork

如果 `readme-gen` 帮你彻底告别手改 README 数字，欢迎点 Star 或 Fork。

<p align="center">
  <a href="https://github.com/TheRealMilesLee/Auto-Generate-Readme/stargazers"><img src="https://img.shields.io/badge/⭐_Star-ff4785?style=for-the-badge" alt="Star"></a>
  <a href="https://github.com/TheRealMilesLee/Auto-Generate-Readme/fork"><img src="https://img.shields.io/badge/🍴_Fork-24292f?style=for-the-badge" alt="Fork"></a>
</p>
<!-- readme-gen:section:zh:cta:end -->

---

*Generated by [readme-gen](https://github.com/TheRealMilesLee/Auto-Generate-Readme) v0.1.0 from repository facts. Facts hash: `0b48bd016fc3`.*
