# Configuration (`readme.yml`)

`readme-gen` merges three inputs:

1. **facts** — measured by scanning the repository (never written by hand);
2. **annotations** — `readme.yml`, the taste layer (this document);
3. **section graph** — selected by `preset`, rendered by the tool.

Numbers in prose are only allowed when they appear in facts or in this file; the
`check` command enforces that.

## Minimal example

```yaml
preset: cli
locale: [en]

project:
  name: my-tool
  tagline:
    en: "Small tool, sharp edges filed off."
  description:
    en: "A CLI that does one thing well."

sections:
  why: |
    It replaces three shell scripts with one binary.

quickstart:
  clone: https://github.com/you/my-tool.git
```

Run:

```bash
readme-gen init            # generate a skeleton from the repository
readme-gen build           # README.md
readme-gen check --strict  # CI-friendly validation
```

## Top-level keys

| Key | Type | Description |
|---|---|---|
| `preset` | string | `library`, `cli`, `app`, `dataset`, `showcase`. Defaults to the detected project kind. |
| `locale` | list | `[en]`, `[zh]`, or `[en, zh]` for a split bilingual document. |
| `project` | map | Name, tagline, description, author, licence override. |
| `quickstart` | map | Clone URL and "starter pack" shortcuts (showcase). |
| `dirs` | map | Per-folder summaries and bullets (dataset/showcase catalogue). |
| `sections` | map | Hand-written prose for `why`, `audience`, `usage`, `philosophy`. |
| `sources` | list | Source/provenance table. |
| `compatibility` | list | Format compatibility table. |
| `awards` | list | "Trophy case" highlights. |
| `changelog` | list | Hand-curated update log (falls back to recent commits). |
| `cta` | map | Star/fork call to action text and toggles. |
| `catalog.order` | list | Preferred folder order for the catalogue. |
| `exclude` | list | Extra gitignore-style patterns for the scanner. |
| `badges.style` | string | Shields.io style (`flat`, `for-the-badge`, …). |
| `credit` | bool | Append the generator credit line (default `true`). |
| `llm` | map | Default endpoint/model/temperature for prose generation. |

## Project

```yaml
project:
  name: "The-Wallpaper-Collection"
  tagline:
    en: "Eight years of desktop taste — one repo."
    zh: "八年策展 · 一个仓库 · 拿来即用"
  description:
    en: "A personal media archive."
    zh: "个人策展媒体档案。"
  author: "Silverhand Miles"
  license: "MIT"          # overrides the detected licence
```

A plain string is used for every locale:

```yaml
tagline: "One line for both languages."
```

## Quickstart and packs

```yaml
quickstart:
  clone: https://github.com/MilesCorporate/The-Wallpaper-Collection.git
  packs:
    - name:
        en: New machine
        zh: 新机器
      dirs: ["2026 Wallpaper", "2025 Wallpaper", "Apple Park"]
      best_for:
        en: Fresh 4K/5K setup
        zh: 全新 4K/5K 桌面
```

If `clone` is omitted, the URL is derived from `git remote origin` when the
remote is GitHub. Packs are rendered only by the `showcase` preset.

## Directory annotations

Used by the `catalog` section (dataset/showcase). Keys must match the folder
name exactly; quote names containing spaces.

```yaml
dirs:
  "2026 Wallpaper":
    summary:
      en: "The active growth lane — F1, automotive, current-gen 4K."
      zh: "当前增长最快的一年，F1、汽车与新一代 4K。"
    bullets:
      - en: "Formula 1 and automotive drops"
        zh: "F1 与汽车专题"
      - en: "272 images, ~963 MiB"
        zh: "272 张，约 963 MiB"
    tags: [automotive, f1, 4k]
```

Only the summary and bullets are rendered. Prefer letting the renderer add
counts (`### folder · 963 MiB · 272 images`). Annotations are treated as
approved input by the number tracer, so the checker cannot tell a stale human
number from a correct one — review them like code.

Folders without annotations still appear, with their measured size and counts,
but without invented prose.

## Prose sections

```yaml
sections:
  why: |
    Two short paragraphs on why the project exists.
  audience: |
    Who this is for, in concrete terms.
  usage: |
    Optional: overrides the generated `--help` snippet.
```

`why` is used by `library`, `cli` and `showcase`; `audience` by `showcase`.
When a section is empty and `--llm` is enabled, the model fills it. When both are
absent, the section is omitted — never filled with placeholder text.

## Tables

```yaml
sources:
  - name: Unsplash
    url: https://unsplash.com/license
    note:
      en: Photographer-attributed work
      zh: 摄影师署名作品

compatibility:
  - name: HEIC
    note:
      en: Windows needs the HEIF extension or conversion.
      zh: Windows 需要 HEIF 扩展或转换。

awards:
  - award:
      en: Largest theme
      zh: 最大主题
    winner: "黑猫的野望壁纸包"

changelog:
  - date: "2026-10-08"
    text:
      en: Stats verified against the working tree.
      zh: 对照工作树核对了统计。

cta:
  star: true
  fork: true
  text:
    en: If this saves you an afternoon, a star helps.
    zh: 如果省下了你一个下午，点个 Star 吧。
```

## Section graph and presets

| Section id | library | cli | app | dataset | showcase |
|---|:-:|:-:|:-:|:-:|:-:|
| `toc` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `quickstart` | – | ✓ | ✓ | ✓ | ✓ |
| `packs` | – | – | – | – | ✓ |
| `numbers` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `why` | ✓ | – | – | – | ✓ |
| `audience` | – | – | – | – | ✓ |
| `catalog` | – | – | – | ✓ | ✓ |
| `structure` | ✓ | ✓ | ✓ | ✓ | – |
| `install` | ✓ | ✓ | ✓ | – | – |
| `usage` | ✓ | ✓ | ✓ | – | – |
| `deps` | ✓ | ✓ | ✓ | – | – |
| `sources` | – | – | – | ✓ | ✓ |
| `compatibility` | – | – | – | – | ✓ |
| `awards` | – | – | – | – | ✓ |
| `license` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `contributing` | ✓ | ✓ | ✓ | – | – |
| `changelog` | – | – | – | ✓ | ✓ |
| `cta` | – | – | – | – | ✓ |

Inspect the exact order with:

```bash
readme-gen plan --preset showcase
readme-gen plan --preset showcase --json
```

## Keep blocks (hand-written overrides)

Run `build` once, then wrap a section body in markers. The next build preserves
it verbatim and rewrites the markers into the output:

```markdown
<!-- readme-gen:keep:en:usage:start -->
## Usage

This paragraph is written by a human and survives every rebuild.
<!-- readme-gen:keep:en:usage:end -->
```

- `{lang}` is `en` or `zh`; `{id}` is the section id (`usage`, `why`, …).
- The block replaces that section for that language only.
- `check` fails on unclosed or mismatched keep blocks.
- In a two-locale document, a prose section must be filled for **both**
  languages (or omitted from both); otherwise the bilingual parity check fails.

## LLM settings

```yaml
llm:
  endpoint: http://127.0.0.1:8080
  model: local
  temperature: 0.2
```

CLI flags override the file. `--llm none` disables generation even if the file
sets an endpoint.

## Check rules

| Rule | Severity |
|---|---|
| metadata present and facts hash current | error |
| no unresolved `{{…}}` | error |
| code fences balanced | error |
| relative links/images exist | error |
| anchors unique; TOC targets exist | error / warning |
| section markers balanced; bilingual ids match | error |
| keep blocks balanced | error |
| every number traceable to facts or `readme.yml` | error |
| heading level jumps | warning |
| `--strict` | warnings become failures |

Exit code is `0` when clean, `1` when any error (or warning with `--strict`)
exists.

## Output location

Generate `README.md` at the repository root. Relative links (licence,
contributing, docs) are resolved from the README's directory, so writing the
document somewhere else will make those links fail `check`. Use `--out` for
previews, and the default path for the real build.
