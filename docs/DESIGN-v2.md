# Auto-Generate-Readme v2 设计提案

> 从「模板占位符」与「LLM 抽卡」走向「可验证的事实驱动 README」。
> 以 `The-Wallpaper-Collection/README.md` 作为质量上限的参照样本。
>
> **实现说明（2026-10-21）**：本设计已落地为零依赖 Rust CLI `readme-gen`。
> 模块划分、跨平台策略与 llama.cpp 接入方式见 [ARCHITECTURE.md](ARCHITECTURE.md)，
> 配置 schema 与校验规则见 [CONFIGURATION.md](CONFIGURATION.md)。

---

## 0. 结论先行

这个仓库现在不是「一个工具 + 一份文档」，而是**三套互相矛盾的东西**：

1. 根 `README.md` 描述了一个并不存在的工具（AST 解析、`--path`、`{{SECTION:usage}}`）。
2. `PythonProject/` 是一套模板生成器，**语法错误，跑不起来**。
3. `run.sh` 是另一套 Bash + Ollama 生成器，把目录树丢给模型，再用 **67 条 `grep -v`** 清理模型的"思考过程"。

而 wallpaper 那份 README 之所以好，不是文笔好，是它把五件事同时做对了：**可验证的事实、明确的叙事、场景化导航、可信的来源说明、持续的更新**。

因此 v2 的核心不是"换个更好的 prompt"，而是补上现在完全缺失的中间层：

```
仓库  →  [事实层 facts]  →  [叙事层 annotations]  →  [章节图 outline]  →  [渲染/写作]  →  [校验]  →  README
              ↑ 机器 100% 可信        ↑ 人的品味，可复用        ↑ 结构复用        ↑ 受约束      ↑ 拒绝幻觉
```

一句话：**数字交给扫描器，故事交给注解文件，模型只负责把两者写成句子，校验器负责不让它乱写。**

---

## 1. 现场勘查：当前实现到底做了什么

### 1.1 硬伤清单（可复现）

| 位置 | 问题 | 后果 |
|---|---|---|
| `PythonProject/readme_generator/core.py:344` | 多了一个 `'''`，字符串未闭合 | `SyntaxError: EOF while scanning triple-quoted string literal` |
| `PythonProject/readme_generator/config.py:123` | 多了一个 `}` | `SyntaxError: unmatched '}'` |
| `PythonProject/main.py:75-76` | `main()` 被调用两次 | 命令执行两遍（修好语法后才会暴露） |
| `core.py:39-45` | `template_path` 默认为 `""`，`Path("")` 即 `"."` 且 `exists()` 为真 | 走"自定义模板"分支，模板名为空 → `TemplateNotFound` |
| `config.py:75-78` | 配置加载异常被全部吞掉，静默回退默认值 | YAML 写错也不报错，用户以为配置生效了 |
| `setup.py:36` | `readme-gen=main:main`，但 `main.py` 不在 `packages` 里 | `pip install` 后命令行入口 `ModuleNotFoundError` |
| `README.md:25-44` | 文档里的 `--path / --stdout / --max-depth / --ignore` 均不存在 | 照文档执行必然失败 |
| `README.md:59-70` | 文档里的 `{{SECTION:usage}}` 语法不存在 | 模板系统名不副实 |
| `README.md:46-57` | 目录结构写错（`readme_generator/main.py` 实际在根） | 新人找不到入口 |
| `requirements.txt` | `pathlib2`（Python 3 内置）、`requests`（未使用） | 无谓依赖 |
| `test_basic.py` | 没有断言、不是 pytest 用例，只是 import 冒烟 | `pytest -q` 收集不到任何测试 |

### 1.2 更根本的问题：宣传的核心能力，两条路线都没有实现

根 README 的第一条 Feature 是 *"Parse Python modules & extract docstrings / comments"*。

- Python 版：`core.py` / `utils.py` 里**没有 `import ast`**。它只做目录树、依赖、Git、徽章，模板正文是 `功能特性 1/2/3` 这种占位文本。
- `run.sh`：`analyze_project()` 只收集目录结构、扩展名统计、重要文件名、语言统计，**一个字节的源码内容都没有传给模型**，但 prompt 里写着 *"Read the source code and comments in the current directory"*。

也就是说，模型被要求"阅读源码"，但它只看到了文件名。它只能根据文件夹名编故事——这正是输出不可控的根源。

### 1.3 run.sh 的工程债

- 1,259 行 Bash，`generate_english_readme` 与 `generate_chinese_readme` 是近 200 行的复制粘贴。
- 用 67 条 `grep -v` 删除"好的，""用户让我""首先，我需要"等模式。这是在**输出端做概率性修补**：删多了会吃掉正文，删少了会漏；模型换个措辞就失效。
- 双语 = 两次独立生成再拼接，两版结构、数字、章节天然不一致。
- `check_existing_readme()` 发现已有 README（>200 字节）就跳过生成 —— 于是**统计永远不会更新**。
- 没有缓存、没有重试预算、没有 Markdown 结构校验、硬编码 `/tmp`。

### 1.4 一个现场证据：文档已经漂移了

wallpaper 仓库现在的真实数据（本次实测）：

| 指标 | README 写的 | 实际扫描 |
|---|---|---|
| 图片 | 3,079 | **3,087** |
| 视频 | 36 | 36 ✅ |
| 媒体体积 | ~10.2 GiB | **10.23 GiB** ✅ |
| 顶层目录 | 27 | 27 ✅ |

只差 8 张图，但这恰好证明：**再好的手工 README 也会漂移**。这不是写作问题，是缺少 `facts → README` 的生成与校验闭环。

---

## 2. 拆解：wallpaper README 为什么强

把那份 776 行 README 按"信息职责"拆开，会得到一张很清晰的产品需求表：

| 层 | README 中的证据 | 本质 | 能否自动生成 |
|---|---|---|---|
| **事实层** | 10.2 GiB / 3,079 images / 36 videos / 27 folders / 每年每目录的体积与张数 | 可验证的量化信息 | ✅ 100% 扫描 |
| **叙事层** | "Your screen deserves better than a search result"、"八年策展 · 一个仓库" | 为什么值得看 | ⚠️ 人给方向，模型润色 |
| **导航层** | Get it in 30 seconds、Starter packs、Pick your setup（4K/带鱼屏/iPad/车） | 我该怎么用 | ✅ 场景 → 目录映射 |
| **目录层** | 每个文件夹：`### 2026 Wallpaper · ~963 MiB · 272 images` + 一句定位 + 3-5 个亮点 | 可浏览的目录 | ✅ 数字 + 注解 |
| **信任层** | Sources、File formats、Copyright、Curation philosophy、Update log | 凭什么信你 | ✅ facts + 人工原则 |
| **转化层** | Star / Fork CTA、trophy case、分享模板 | 让读者行动 | ✅ 配置 + 模板 |

**关键观察：776 行里，大约 70% 是"事实 + 结构"，只有 30% 是真正的文学表达。** 现在两个实现都在试图用模型一次性生成那 100%，所以既不稳定也不像样。正确做法是把 70% 变成确定性渲染，把 30% 收窄成"受事实约束的短文"。

---

## 3. 目标与非目标

### 目标

1. **零虚构**：README 里出现的每个数字、路径、目录名，都能在 `facts.json` 里找到出处。
2. **结构可复用**：`preset`（library / cli / app / dataset / showcase）决定章节图，而不是每次让模型自由发挥。
3. **写作可插拔**：完全离线（纯模板）也能产出结构完整的 README；开 LLM 只提升叙事，不改变事实。
4. **可校验**：Markdown 语法、锚点、相对链接、占位符、数字一致性、双语对齐，全部机器检查。
5. **可增量维护**：只重写生成区块，人工段落用标记保留；CI 能检测"事实变了但 README 没更新"。
6. **可测试**：facts 快照 + golden README + LLM 录制回放，CI 不依赖模型。

### 非目标

- 不做文档站生成器（mkdocs / vitepress 的活）。
- 不替代品牌文案；不追求"自动写出爆款 README"。
- 不自动 commit / push；工具只负责生成与校验。
- 不支持"读完整仓库源码再总结"这种幻觉温床；上下文只给事实 + 选定的入口文件片段。

---

## 4. 目标架构

### 4.1 主流水线

```
readme-gen scan  <repo>                → .readme/facts.json
readme-gen plan  --preset showcase     → .readme/outline.json（章节图，可评审）
readme-gen build --annotations readme.yml --lang en,zh
                                       → README.md（带 region 标记）
readme-gen check README.md --facts ... → exit 0/1（CI 用）
readme-gen diff                        → 只打印 unified diff，不落盘
```

分层原则：**每一层都能单独跑、单独测、单独替换**。LLM 只是 `compose` 层的一个可选实现。

### 4.2 模块划分

```
src/readme_gen/
├── cli.py                 # click/typer：scan | plan | build | check | adopt
├── config.py              # pydantic 配置模型，未知字段报错
├── facts/
│   ├── scan.py            # os.scandir 遍历，尊重 .gitignore（pathspec）
│   ├── languages.py       # 扩展名 → 语言映射（可后续接 linguist）
│   ├── media.py           # 图片/视频计数、体积、扩展名分布
│   ├── deps.py            # requirements / pyproject / package.json / go.mod
│   ├── gitinfo.py         # remote、branch、首末提交、贡献者（可选）
│   └── schema.py          # Facts dataclass + JSON Schema + 稳定 hash
├── outline/
│   ├── presets.py         # 5 个 preset 的章节图
│   └── plan.py            # 依据 facts 裁剪章节（没有 tests 就不出测试章节）
├── compose/
│   ├── base.py            # Section 协议
│   ├── deterministic/     # hero / badges / numbers / tree / catalog / quickstart ...
│   ├── llm.py             # 按 section 调用，JSON 输出，占位符约束
│   └── cache.py           # key = sha256(facts_slice + prompt + model + params)
├── render/
│   ├── assemble.py        # 章节拼接 + region 标记 + keep 区块
│   ├── toc.py             # 从标题树生成 TOC 与稳定锚点
│   └── templates/         # 每个 section 的 Jinja2 片段（不是整篇模板）
├── validate/
│   ├── markdown.py        # markdown-it-py 解析，标题层级/表格/代码块
│   ├── links.py           # 相对链接与图片是否存在
│   ├── digits.py          # 数字溯源（最关键的防幻觉）
│   ├── placeholders.py    # 禁止 `{{...}}`、TODO、Lorem
│   └── bilingual.py       # 中英章节对齐、锚点互通
└── annotations.py         # readme.yml 的模型与合并逻辑
```

依赖收敛为：`click`、`pydantic`、`jinja2`、`pyyaml`、`pathspec`、`markdown-it-py`、`httpx`（仅云端 provider）、`gitpython`（可选）。删掉 `pathlib2`、`requests`。

### 4.3 性能

实测该扫描方式遍历 wallpaper 仓库 3,139 个文件、约 10.2 GiB，**无需读取文件内容**（`st_size` 即可），是秒级操作。永远不要为了统计体积去 `du` 或读文件。

---

## 5. 事实模型 `facts.json`

这是整个设计的基石：**只放机器能证明的东西**。

```json
{
  "schema": "readme-facts/v1",
  "root": "/Users/silverhand/Pictures/The-Wallpaper-Collection",
  "project": {
    "name": "The-Wallpaper-Collection",
    "kind": "dataset",
    "license": null,
    "version": null
  },
  "counts": { "files": 3139, "dirs": 27, "images": 3087, "videos": 36 },
  "sizes": {
    "media_bytes": 10984000000,
    "media_human": "10.2 GiB",
    "images_human": "9.99 GiB",
    "videos_human": "0.24 GiB"
  },
  "languages": [],
  "git": { "remote": "…", "branch": "main", "first_commit": "…", "last_commit": "…" },
  "dirs": [
    {
      "path": "2026 Wallpaper",
      "files": 272,
      "images": 272,
      "videos": 0,
      "bytes_human": "963 MiB",
      "exts": { ".jpg": 240, ".png": 32 },
      "depth": 1
    }
  ],
  "deps": [], "entrypoints": [], "tests": null,
  "tree": "…",
  "readme": { "exists": true, "sha256": "…" }
}
```

设计约束：

- **只增不改**：`schema` 版本化，新增字段不破坏旧渲染器。
- **稳定 hash**：对 `facts` 做规范化 JSON 序列化后取 sha256，写进 README 注释，用于 drift 检测。
- **不推断**：不猜"这个项目是做什么的"，只记录可观测事实。职责边界清晰，后续校验才有依据。
- **目录粒度可配置**：默认 depth=1；`showcase` preset 可下钻到 depth=2，并为每个目录产出体积/数量。

---

## 6. 注解层：把"品味"变成可复用资产

事实层解决不了 "Why this collection hits different"，也解决不了 "哪四个目录适合新机器"。这些必须由人（或人审过的 AI）提供，但**不应该每次重新写**。

`readme.yml`：

```yaml
preset: showcase
locale: [en, zh]

project:
  tagline:
    en: "Eight years of desktop taste — one repo."
    zh: "八年策展 · 一个仓库 · 拿来即用"
  audience: [desktop, mac, gaming, photography, anime, automotive, space]

quickstart:
  clone: "https://github.com/MilesCorporate/The-Wallpaper-Collection.git"
  packs:
    - name: { en: "New machine", zh: "新机器" }
      dirs: ["2026 Wallpaper", "2025 Wallpaper", "Apple Park"]
      best_for: { en: "Fresh 4K/5K setup", zh: "4K/5K 新装" }

annotations:
  dirs:
    "2026 Wallpaper":
      summary:
        en: "The active growth lane — F1, automotive, and current-gen 4K."
        zh: "当前增长最快的一年，F1、汽车与新一代 4K。"
      bullets:
        - { en: "272 images, ~963 MiB", zh: "272 张，约 963 MiB" }
      tags: [automotive, f1, 4k]
    "Artemis II":
      summary: { en: "NASA lunar flyby imagery.", zh: "NASA 月球飞掠影像。" }

policy:
  copyright: "personal archive; rights remain with creators"
  excluded_from_scan: [".git", ".DS_Store"]
```

规则：

- 注解**只写"意义"，不写"数字"**。数字一律从 facts 取，避免两处真相。
- 没有注解的目录，渲染器只输出事实行（`### 2026 Wallpaper · 963 MiB · 272 images`），**不编造描述**；`check --strict` 会把它标为"待补注解"，而不是让模型自由发挥。
- 注解是纯文本、可 review、可复用；换模型、换语言都不重写。

---

## 7. 章节图与 preset

`Section` 协议：

```python
class Section(Protocol):
    id: str                       # "catalog", "numbers"
    needs: tuple[str, ...]        # 依赖的 facts 键，缺了就裁剪
    locales: tuple[str, ...]      # ("en", "zh") 或单语
    mode: Literal["deterministic", "llm", "keep"]
    def render(self, ctx: Context) -> str: ...
```

Preset 只决定"有哪些章节、顺序、权重"，不决定内容：

| Section | library | cli | app | dataset | showcase（wallpaper） | 来源 |
|---|:-:|:-:|:-:|:-:|:-:|---|
| hero（一句价值 + 关键数字） | ✅ | ✅ | ✅ | ✅ | ✅ | facts + annotation |
| badges（真实统计） | ✅ | ✅ | ✅ | ✅ | ✅ | facts |
| toc + 语言锚点 | ✅ | ✅ | ✅ | ✅ | ✅ | 标题树 |
| quickstart / install | ✅ | ✅ | ✅ | ✅ | ✅ | facts + annotation |
| starter packs（场景导航） | – | – | – | – | ✅ | annotation |
| by the numbers | – | – | – | ✅ | ✅ | facts |
| who this is for | – | – | – | – | ✅ | annotation |
| catalog（逐目录） | – | – | – | ✅ | ✅ | facts + annotation |
| structure / tree | ✅ | ✅ | ✅ | – | – | facts |
| usage / examples | ✅ | ✅ | ✅ | ✅ | – | annotation + 代码片段 |
| deps / compatibility | ✅ | ✅ | ✅ | – | ✅（格式兼容） | facts |
| sources / copyright | – | – | – | ✅ | ✅ | annotation |
| license | ✅ | ✅ | ✅ | ✅ | ✅ | facts |
| contributing / changelog | ✅ | ✅ | ✅ | – | ✅（update log） | facts + git |
| CTA（star/fork） | – | – | – | – | ✅ | config |

`plan` 命令的产物 `outline.json` 要能被人 review——这是"可控生成"的关键：**先看结构，再看文字**。

---

## 8. 生成策略：确定性优先，LLM 其次

### 8.1 确定性渲染器（默认路径，无模型）

适用于所有"事实即内容"的章节：hero 数字、badges、numbers 表、目录 catalog、tree、deps、license、changelog。

优点：可重复、可快照测试、零成本、零幻觉。**先把这条路做到 100%，工具就已经有用了。**

### 8.2 LLM 只写"散文"，并且被事实约束

只对 `why / audience / philosophy / summary` 这类章节启用。调用契约：

```
SYSTEM
You write one section of a README. You may only assert facts present in FACTS.
Numbers and paths MUST be emitted as placeholders: {{n:counts.images}}, {{p:dirs.2026 Wallpaper}}.
Never type a digit or a path yourself. Never mention the reader, the prompt, or your process.
Return JSON: {"markdown": "<section body>"}

USER
FACTS: <该章节所需的 facts 切片>
ANNOTATIONS: <对应注解>
STYLE: <2 个该 preset 的范例段落>
```

生成后再做二次替换：`{{n:counts.images}}` → `3,087`，`{{p:dirs.2026 Wallpaper}}` → `` `2026 Wallpaper` ``。

三种失败处理，按顺序：

1. 结构/占位符校验失败 → 带错误信息重试一次（temperature 0.2）。
2. 仍失败 → 降级到确定性兜底文案（事实行），**构建永远不失败**。
3. 全程不 `grep -v` 删正文。删掉的每一行都可能是用户要的内容；正确做法是**在源头约束 + 在出口校验**。

### 8.3 缓存

`cache key = sha256(facts_slice + prompt_template + model + params)`，落盘 `.readme/cache/`。
CI 里用录制好的响应（VCR）跑测试，不依赖 Ollama 在线。

---

## 9. 双语：一次结构，两次表达

不要像 `run.sh` 那样生成两份独立 README 再拼接。正确做法：

1. 一份 `outline` 同时服务两个 locale。
2. 确定性章节只渲染一次数字/表格结构，按 locale 替换标签。
3. LLM 章节按 locale 分别写，但共享 facts 切片与注解。
4. 锚点由 `section.id` 生成（`<a id="starter-packs"></a>`），**不用中文标题自动锚点**（GitHub 对中文/标点的锚点规则不稳定）。
5. `validate/bilingual.py` 检查：章节数量一致、顺序一致、每节非空、锚点互通、数字完全一致。

---

## 10. 质量门禁

`readme-gen check` 的规则（按严重级别）：

| 级别 | 规则 | 说明 |
|---|---|---|
| error | Markdown 可解析、标题层级不跳级 | markdown-it-py AST |
| error | 无未替换占位符 `{{...}}`、无 TODO/Lorem | 防止模板泄漏 |
| error | 每个相对链接/图片路径存在 | 防止失效链接 |
| error | 数字溯源：正文中每个数字必须出现在 facts 或 allowlist（年份、版本号、分辨率） | **防幻觉的核心** |
| error | facts hash 与 README 注释一致 | drift 检测 |
| warn | TOC 与标题不匹配、锚点重复 | 可自动修 |
| warn | 注解缺失的目录（`--strict` 下升级为 error） | 推动补内容 |
| warn | 段落过长 / 表格列数不一致 | 排版 |

### drift 检测

README 头部写入：

```markdown
<!-- readme-gen:meta {"schema":"readme-facts/v1","facts":"sha256:ab12…","preset":"showcase","version":"2.0.0"} -->
```

`check` 重新扫描并比对 hash。CI 里跑：

```yaml
- run: readme-gen check README.md --repo .
```

统计变了但 README 没重生成 → CI 失败。这正好解决 3,079 → 3,087 的漂移。

### 区域所有权（增量维护的关键）

```markdown
<!-- readme-gen:section:hero:start -->
…生成的 hero…
<!-- readme-gen:section:hero:end -->

<!-- readme-gen:keep:start -->
这里是人写的、工具永不覆盖的内容
<!-- readme-gen:keep:end -->
```

`adopt` 命令可以把一份现有 README 包成 "生成区 + 保留区"。**没有这个机制，没人敢让工具碰自己的 README。**它比任何 prompt 技巧都更能提升采用率。

---

## 11. 测试策略

```
tests/
├── fixtures/repos/
│   ├── tiny-py/          # 3 个文件，最小闭环
│   ├── py-cli/           # 有 entrypoint、tests、pyproject
│   ├── media-dataset/    # 目录 + 假图片（用空文件占位）
│   └── broken/           # 非法 YAML、缺失 LICENSE
├── golden/
│   ├── tiny-py.showcase.md
│   └── media-dataset.dataset.md
├── test_scan.py          # facts 快照（数字、体积、深度）
├── test_render.py        # 同 facts 两次渲染 byte-identical
├── test_check.py         # 每条 error 规则各一个反例
└── test_llm.py           # 录制响应 + 占位符替换 + 降级路径
```

最低要求：`pytest` 真的能跑；CI 矩阵 Python 3.10–3.13 + macOS/Linux。

---

## 12. 迁移路线（约两周，可分段交付）

### Phase 0 — 先让它诚实（0.5 天）

- 修 `core.py:344`、`config.py:123`、`main.py:75-76`。
- 修 `template_path` 空值判断（`if str(path).strip()`）。
- 修 `setup.py` 入口点与包布局；删 `pathlib2`/`requests`。
- 把 `run.sh` 标为 `legacy/` 或直接删除（它的能力会被 `scan` 取代）。
- 用真实状态重写根 README（见附录）。
- `pytest` 能收集到测试。

**验收**：`pip install -e PythonProject && readme-gen --help` 成功；`pytest` 有 ≥1 个通过用例。

### Phase 1 — Facts（2–3 天）

- 实现 `scan`：遍历、gitignore、语言/媒体/体积/依赖/入口点/git。
- `facts.json` + JSON Schema + 稳定 hash。
- 对 wallpaper 仓库跑出 3,087 / 36 / 10.23 GiB / 27 目录，与人工核对。

**验收**：facts 与独立脚本（Python `os.walk`）逐项一致；扫描 < 5s。

### Phase 2 — 确定性 README（3–4 天）

- Section 协议 + 5 个 preset + 确定性渲染器。
- `plan` / `build` / `check`（先做 markdown、链接、占位符、数字四条规则）。
- region 标记 + `adopt` + TOC 生成。

**验收**：对 `PythonProject` 自身生成一份结构完整、零占位符、所有数字可溯源的 README；golden 测试通过。

### Phase 3 — LLM 章节（3–5 天）

- provider 接口（ollama / openai 兼容），默认本地 Ollama。
- 占位符契约 + JSON 输出 + 重试 + 降级 + 缓存。
- 双语校验。

**验收**：断网时构建仍成功（降级）；同一 facts + 缓存 → 输出稳定；`check` 拦截一个故意注入的假数字。

### Phase 4 — Showcase 与验收（2–3 天）

- `showcase` preset + catalog/quickstart/numbers/sources 渲染器。
- `readme.yml` 注解模型。
- 以 wallpaper 仓库为 benchmark，产出中英双语 README，和人工版做结构对比。

**验收**（见下节）。

---

## 13. 验收：拿 wallpaper 仓库当 benchmark

目标不是"复刻那 776 行"，而是**用 10% 的人工输入拿到 80% 的结构与 100% 的准确数字**。

| 指标 | 目标 |
|---|---|
| 事实准确率 | 100%（27 目录、3,087 图、36 视频、10.23 GiB 全部吻合） |
| 结构覆盖 | hero / badges / quickstart / numbers / catalog / sources / license / update 全有 |
| 人工改动量 | 生成稿到可发布稿的 diff ≤ 30% 行数，且只改散文 |
| 幻觉 | 0 个不存在的路径、0 个凭空数字 |
| 构建时间 | 确定性模式 < 5s；本地 LLM < 2min |
| 可重复性 | 同 facts 连续两次构建 byte-identical（确定性模式） |

示例命令：

```bash
readme-gen scan ~/Pictures/The-Wallpaper-Collection -o .readme/facts.json
readme-gen plan --facts .readme/facts.json --preset showcase -o .readme/outline.json
readme-gen build --facts .readme/facts.json --preset showcase \
  --annotations readme.yml --lang en,zh --out README.generated.md
readme-gen check README.generated.md --facts .readme/facts.json --strict
readme-gen diff README.generated.md README.md
```

---

## 14. 风险与反模式

| 风险 | 处置 |
|---|---|
| LLM 不稳定 / 成本 | 按 section 缓存；确定性兜底；默认本地模型 |
| 大仓库扫描慢 | 只 `stat` 不读内容；gitignore；`--max-files` 上限 |
| 用户不敢让工具改 README | region + keep 标记；`diff` 预览；默认不落盘 |
| preset 不够用 | `custom_sections` 逃生口，但必须走同一套校验 |
| 多语言维护成本 | 单 outline 双 locale；注解按 locale 存 |
| 隐私 | 默认本地 Ollama；云端 provider 需显式 `--provider` |

**明确的反模式（不要做）：**

1. ❌ 把 `tree` 输出丢给模型，让它"读源码"——它读不到，只会编。
2. ❌ 用几十条 `grep -v` 清理模型输出——删不干净，还会误删正文。
3. ❌ 中英文各生成一遍再拼接——结构与数字必然漂移。
4. ❌ README 里写"功能特性 1/2/3"——模板占位符不是内容。
5. ❌ 让模型直接写数字和路径——数字必须来自 facts，路径必须来自目录扫描。
6. ❌ 发现已有 README 就跳过——统计会永远停在生成那一天（本仓库的 8 张图偏差就是这么来的）。

---

## 15. 附录：本仓库 README 应该长什么样

工具的第一个用户应该是它自己。建议骨架：

```markdown
# Auto-Generate-Readme

> 用可验证的事实生成可维护的 README。数字来自扫描，故事来自注解，模型只负责措辞。

<!-- 真实 badges：Python 版本、CI 状态、License，全部来自仓库事实 -->

## 30 秒上手
pipx install readme-gen
readme-gen scan .
readme-gen build --preset library

## 它解决什么问题
| 手工 README 的痛点 | 本工具的做法 |
| 数字会过期 | facts.json + drift check |
| 每个项目重写一遍结构 | preset + section graph |
| AI 写的 README 会编 | 数字/路径占位符 + 校验器 |
| 不敢让工具覆盖 | region 标记 + keep 区块 |

## 两种模式
- 确定性：零模型、可重复、适合 CI。
- LLM 辅助：只写散文，事实仍由 facts 提供。

## 架构（一张图）
scan → facts → outline → compose → validate → README

## 质量门禁
readme-gen check README.md --repo .   # CI 里跑，统计漂移就失败

## Roadmap / License / Contributing
```

**注意**：以上每个命令、每个功能，都必须在 Phase 2/3 真正实现后才写进 README。这个仓库当前最大的问题不是代码质量，而是**文档承诺了不存在的东西**——这恰好是它想解决的问题本身。

---

*本设计基于 2026-10-21 对仓库与 wallpaper 样本的实际阅读与扫描；所有数字均可复现。*
