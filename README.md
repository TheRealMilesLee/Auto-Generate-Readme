# Auto-Generate-Readme

> EN: Tooling to parse source code (Python) & inline comments, then synthesize structured README documentation via configurable templates.
> 中文: 解析源码与注释，通过可配置模板自动生成结构化 README 文档的工具。

## ✨ Features / 特性
- Parse Python modules & extract docstrings / comments
- Template-driven sections (支持占位符替换)
- Multi-file traversal with ignore patterns
- Optional metadata enrichment (文件计数、语言统计)
- CLI output to stdout or write file

## 📦 Installation / 安装
```bash
git clone <repo-url>
cd Auto-Generate-Readme
python -m venv .venv && source .venv/bin/activate
pip install -e .        # (如含 setup.py)
# 或直接运行脚本无需安装
```

## 🚀 Usage / 使用
Basic:
```bash
python main.py --path ./target_project --output README.md
```
Example script:
```bash
python example.py --path ./PythonProject --stdout
```
Help:
```bash
python main.py --help
```

### Common Arguments / 常用参数
| Flag | 描述 | EN |
|------|------|----|
| `--path` | 目标项目路径 | Target project path |
| `--output` | 输出文件路径 | Output file path |
| `--stdout` | 打印到控制台 | Print to stdout |
| `--template` | 模板文件 | Template file path |
| `--max-depth` | 遍历最大深度 | Max directory depth |
| `--ignore` | 忽略模式 (逗号分隔) | Ignore glob patterns |

## 🗂 Structure / 目录结构 (示例)
```
PythonProject/
   readme_generator/
      main.py          # CLI 入口
      utils.py         # 工具函数
      example.py       # 使用示例
      test_basic.py    # 测试用例
      templates/       # 模板集合 (可选)
setup.py             # 包配置 (如存在)
run.sh               # 快捷脚本
```

## 🧠 Template System / 模板系统
支持占位符：
```
{{PROJECT_NAME}}  {{FILE_COUNT}}  {{PYTHON_VERSION}}
{{SECTION:usage}}  # 引用 usage 子块
```
示例最简模板：
```
# {{PROJECT_NAME}}
Total Python files: {{FILE_COUNT}}
{{SECTION:description}}
```

## 🔍 Extraction Logic / 提取逻辑
流程：文件遍历 → 过滤 (ignore) → 解析 AST → 收集函数/类 docstring → 聚合统计 → 渲染模板。

伪代码：
```python
def collect(path):
      for file in python_files(path):
            tree = ast.parse(open(file).read())
            for node in ast.walk(tree):
                  if isinstance(node, ast.FunctionDef):
                        docs[node.name] = ast.get_docstring(node) or ''
      return docs
```

## 🧪 Testing / 测试
Run basic tests:
```bash
pytest -q            # 若已添加 pytest 支持
python readme_generator/test_basic.py
```
建议新增：模板渲染结果快照测试、忽略模式匹配测试、AST 解析异常捕获测试。

## ⚙️ Configuration / 配置文件 (`config.yaml` 建议示例)
```yaml
project_name: SampleProject
include_patterns: ['**/*.py']
ignore_patterns: ['tests/*', 'build/*']
template: templates/default.md.j2
sections:
   description: 'Auto generated description.'
   usage: 'python main.py --help'
```

## 📈 Metrics / 指标示例
| Metric | Value |
|--------|-------|
| Python files | (count) |
| Empty docstrings | (count) |
| Functions parsed | (count) |
| Classes parsed | (count) |

## 🧩 Extension Ideas / 拓展
- 支持多语言解析 (JavaScript, Go)
- 引入 Jinja2 模板引擎
- Git 提交统计 (最近变更/贡献者)
- 生成徽章（行数 / 测试覆盖率）
- 输出多格式：Markdown / HTML / JSON

## 🤝 Contributing / 贡献
1. 遵循 PEP8 + 添加类型标注
2. 新增功能附加最小测试
3. 模板变量需在文档中登记表格
4. 提交前：
```bash
flake8 || echo "Lint reviewed"
black --check . || echo "Formatting suggestions"
```

## 📄 License / 许可证
See `LICENSE` (MIT 或其他)。

## Roadmap / 后续规划
- [ ] 支持并行解析提高速度
- [ ] 增加缓存避免重复解析大项目
- [ ] 增加 `--dry-run` 仅打印统计
- [ ] 插件式提取器（函数/类/注释自定义）

---
### 中文速览
克隆 → 安装依赖 → 指定 path 与 template → 生成 README → 可扩展多语言与统计。

