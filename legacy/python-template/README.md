# README Generator

一个用于自动生成和更新项目 README 文件的 Python 工具

![Python Version](https://img.shields.io/badge/python-3.8+-blue.svg)
![License](https://img.shields.io/badge/license-MIT-green.svg)

## 特性

- ✨ 自动检测项目信息（名称、描述、依赖等）
- 🚀 支持多种配置格式（YAML、JSON、TOML）
- 📦 模块化设计，易于扩展
- 🎨 自定义模板支持
- 🔍 预览功能，生成前可查看效果
- 📊 自动生成项目徽章
- 🌲 智能项目结构分析

## 安装

### 环境要求

- Python 3.8+

### 安装方法

```bash
# 克隆仓库
git clone https://github.com/yourusername/readme-generator
cd readme-generator

# 安装依赖
pip install -r requirements.txt
```

## 使用

### 基本用法

```bash
# 使用默认配置生成 README
python main.py

# 使用自定义配置
python main.py --config my-config.yaml

# 预览模式（不生成实际文件）
python main.py --dry-run

# 详细输出
python main.py --verbose
```

### 配置文件

创建 `config.yaml` 文件来自定义生成设置：

```yaml
# 项目信息
project_name: "我的项目"
project_description: "这是一个很棒的项目"
author: "你的名字"
license: "MIT"

# 功能开关
include_badges: true
include_toc: true
include_installation: true
include_usage: true

# Git 设置
git_auto_detect: true
github_username: "yourusername"
repository_name: "your-repo"
```

### 示例脚本

运行示例脚本来体验完整功能：

```bash
python example.py
```

## 项目结构

```
readme-generator/
├── main.py                 # 主入口文件
├── example.py             # 示例脚本
├── config.yaml            # 默认配置
├── requirements.txt       # 依赖列表
├── setup.py              # 安装脚本
└── readme_generator/      # 核心包
    ├── __init__.py
    ├── core.py           # 主要生成逻辑
    ├── config.py         # 配置管理
    └── utils.py          # 工具函数
```

## 命令行选项

```
Options:
  -c, --config PATH    配置文件路径 (默认: config.yaml)
  -o, --output PATH    输出 README 文件路径 (默认: README.md)
  -t, --template PATH  自定义模板文件路径
  -v, --verbose        详细输出模式
  --dry-run           仅预览，不实际生成文件
  --help              显示帮助信息
```

## 高级功能

### 自定义模板

你可以创建自定义的 Jinja2 模板：

```jinja2
# {{ project_name }}

{{ project_description }}

## 自定义章节

这是一个自定义的章节内容。

项目依赖：
{% for dep in dependencies %}
- {{ dep }}
{% endfor %}
```

### 自定义章节

在配置文件中添加自定义章节：

```yaml
custom_sections:
  - title: "特殊说明"
    content: "这里是特殊说明的内容"
  - title: "更新日志"
    content: "项目的更新历史"
```

## 开发

### 开发环境设置

```bash
# 克隆仓库
git clone https://github.com/yourusername/readme-generator
cd readme-generator

# 创建虚拟环境
python -m venv venv
source venv/bin/activate  # Windows: venv\Scripts\activate

# 安装开发依赖
pip install -r requirements.txt
```

### 运行测试

```bash
# 运行示例
python example.py

# 测试基本功能
python main.py --dry-run --verbose
```

## 贡献

欢迎贡献！请遵循以下步骤：

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

## 作者

Your Name

---

*这个 README 展示了从 bash 脚本重构为 Python 项目后的强大功能和可维护性*
