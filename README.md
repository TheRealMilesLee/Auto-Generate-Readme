<!-- readme-gen:meta {"schema":1,"facts":"3e97e1ca783f61498ab02bafc1724c8c49872370d402e4e9f10c4f942f5375fc","preset":"cli","locales":["en","zh"],"generator":"readme-gen/0.1.0"} -->
# readme-gen

<p align="center">
  <strong>READMEs that cannot lie about their numbers.</strong><br>
  <sub>不会在数字上撒谎的 README 生成器。</sub><br>
  <sub>CLI tool · Rust · 30 files · MIT</sub>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/language-Rust-orange?style=flat" alt="Rust">
  <img src="https://img.shields.io/badge/license-MIT-green?style=flat" alt="license">
  <img src="https://img.shields.io/badge/version-0.1.0-blueviolet?style=flat" alt="version">
  <img src="https://img.shields.io/github/stars/TheRealMilesLee/Auto-Generate-Readme?style=flat" alt="stars">
  <img src="https://img.shields.io/github/forks/TheRealMilesLee/Auto-Generate-Readme?style=flat" alt="forks">
</p>

<p align="center"><a href="#english">English</a> · <a href="#chinese">中文</a></p>

<a id="english"></a>
## English

## Contents

- [Quick start](#quickstart)
- [By the numbers](#numbers)
- [Why this project](#why)
- [Installation](#install)
- [Usage](#usage)
- [Project structure](#structure)
- [Contributing](#contributing)
- [License](#license)

<!-- readme-gen:section:en:quickstart:start -->
<a id="quickstart"></a>
## Quick start

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
## By the numbers

| Metric | Value |
|---|---|
| **Folders** | 4 |
| **Files** | 30 |
| **Languages** | Rust (16), Shell (1), PowerShell (1) |
| **Test files** | 1 |
<!-- readme-gen:section:en:numbers:end -->

<!-- readme-gen:section:en:why:start -->
<a id="why"></a>
## Why this project

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
| Numbers go stale | Embeds a facts hash; `check` fails on drift |
| Structure is reinvented per project | Five presets share one section graph |
| AI invents numbers | Every digit is traced to facts or `readme.yml` |
| Regeneration overwrites edits | `keep` blocks survive rebuilds |

Everything runs locally: `cargo test` plus `scripts/check.sh`. No hosted CI
is required.

The reference benchmark is a multi-gigabyte wallpaper archive with dozens
of folders and thousands of images. That README is excellent because it is
*specific*; this tool makes the specific parts reproducible and the
tasteful parts editable.
Start with [the architecture](docs/ARCHITECTURE.md),
[the configuration reference](docs/CONFIGURATION.md), and
[the benchmarks](docs/BENCHMARKS.md).
<!-- readme-gen:section:en:why:end -->

<!-- readme-gen:section:en:install:start -->
<a id="install"></a>
## Installation

Build from source with the commands for your platform.

```bash
cargo build --release
cargo install --path .
```
<!-- readme-gen:section:en:install:end -->

<!-- readme-gen:section:en:usage:start -->
<a id="usage"></a>
## Usage

Scan first, then generate and validate:

```bash
readme-gen scan --out .readme/facts.json
readme-gen init --preset showcase --lang en,zh
readme-gen build
readme-gen check --strict
```

To let a local model write the prose sections (a 7–8B model is enough):

```bash
llama-server -m models/Qwen2.5-7B-Instruct-Q5_K_M.gguf \
  --host 127.0.0.1 --port 8080 -ngl 99 -c 8192 -fa on --jinja
readme-gen build --llm llama.cpp --model local
```

See [the model guide](docs/MODELS.md) for 8 GB / 32 GB hardware profiles,
quantisation and alternatives.

`check` is the gate: it verifies links, anchors, markers, bilingual parity,
and that every number in the document exists in the facts or in this file.
<!-- readme-gen:section:en:usage:end -->

<!-- readme-gen:section:en:structure:start -->
<a id="structure"></a>
## Project structure

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
│   ├── annotations.rs
│   ├── error.rs
│   ├── glob.rs
│   ├── json.rs
│   ├── lang.rs
│   ├── lib.rs
│   ├── llm.rs
│   ├── main.rs
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

<!-- readme-gen:section:en:contributing:start -->
<a id="contributing"></a>
## Contributing

Issues and pull requests are welcome. Please keep changes small and focused, and run the test suite before opening a PR.

- [Open an issue](https://github.com/TheRealMilesLee/Auto-Generate-Readme/issues)
- [Open a pull request](https://github.com/TheRealMilesLee/Auto-Generate-Readme/pulls)

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.
<!-- readme-gen:section:en:contributing:end -->

<!-- readme-gen:section:en:license:start -->
<a id="license"></a>
## License

Released under the **MIT** license. See [`LICENSE`](LICENSE).
<!-- readme-gen:section:en:license:end -->

---

<a id="chinese"></a>
## 中文

## 目录

- [快速开始](#zh-quickstart)
- [数据一览](#zh-numbers)
- [为什么值得](#zh-why)
- [安装](#zh-install)
- [使用](#zh-usage)
- [项目结构](#zh-structure)
- [贡献](#zh-contributing)
- [许可证](#zh-license)

<!-- readme-gen:section:zh:quickstart:start -->
<a id="zh-quickstart"></a>
## 快速开始

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
## 数据一览

| 指标 | 数值 |
|---|---|
| **目录** | 4 |
| **文件** | 30 |
| **语言** | Rust (16), Shell (1), PowerShell (1) |
| **测试文件** | 1 |
<!-- readme-gen:section:zh:numbers:end -->

<!-- readme-gen:section:zh:why:start -->
<a id="zh-why"></a>
## 为什么值得

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
| 数字会过期 | 写入事实 hash，`check` 在漂移时失败 |
| 每个项目重造结构 | 五个 preset 共用一张章节图 |
| 模型编造数字 | 每个数字都追溯到事实或 `readme.yml` |
| 重新生成覆盖手写内容 | `keep` 区块跨重建保留 |

一切都在本地完成：`cargo test` 与 `scripts/check.sh`，不依赖托管 CI。

参照样本是一份数 GiB、几十个目录、数千张图片的壁纸归档。那份 README 之所以
好，是因为它足够具体；这个工具让具体的部分可复现，让有品味的部分可编辑。可以
先读 [架构说明](docs/ARCHITECTURE.md)、[配置参考](docs/CONFIGURATION.md) 与
[性能基准](docs/BENCHMARKS.md)。
<!-- readme-gen:section:zh:why:end -->

<!-- readme-gen:section:zh:install:start -->
<a id="zh-install"></a>
## 安装

按所在平台从源码构建。

```bash
cargo build --release
cargo install --path .
```
<!-- readme-gen:section:zh:install:end -->

<!-- readme-gen:section:zh:usage:start -->
<a id="zh-usage"></a>
## 使用

先扫描，再生成和校验：

```bash
readme-gen scan --out .readme/facts.json
readme-gen init --preset showcase --lang en,zh
readme-gen build
readme-gen check --strict
```

允许本地模型撰写散文（7–8B 已经够用）：

```bash
llama-server -m models/Qwen2.5-7B-Instruct-Q5_K_M.gguf \
  --host 127.0.0.1 --port 8080 -ngl 99 -c 8192 -fa on --jinja
readme-gen build --llm llama.cpp --model local
```

模型大小、量化、显存预算与替代方案见 [模型指南](docs/MODELS.md)。

`check` 是闸门：它检查链接、锚点、标记、双语对齐，以及文档里的每个数字
是否都存在于事实或本文件中。
<!-- readme-gen:section:zh:usage:end -->

<!-- readme-gen:section:zh:structure:start -->
<a id="zh-structure"></a>
## 项目结构

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
│   ├── annotations.rs
│   ├── error.rs
│   ├── glob.rs
│   ├── json.rs
│   ├── lang.rs
│   ├── lib.rs
│   ├── llm.rs
│   ├── main.rs
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

<!-- readme-gen:section:zh:contributing:start -->
<a id="zh-contributing"></a>
## 贡献

欢迎提交 Issue 和 Pull Request。请保持改动小而聚焦，并在提交前跑完测试。

- [提交 Issue](https://github.com/TheRealMilesLee/Auto-Generate-Readme/issues)
- [提交 PR](https://github.com/TheRealMilesLee/Auto-Generate-Readme/pulls)

详见 [CONTRIBUTING.md](CONTRIBUTING.md)。
<!-- readme-gen:section:zh:contributing:end -->

<!-- readme-gen:section:zh:license:start -->
<a id="zh-license"></a>
## 许可证

本项目采用 **MIT** 许可证。 详见 [`LICENSE`](LICENSE)。
<!-- readme-gen:section:zh:license:end -->

---

*Generated by [readme-gen](https://github.com/TheRealMilesLee/Auto-Generate-Readme) v0.1.0 from repository facts. Facts hash: `3e97e1ca783f`.*
