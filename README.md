# Auto-Generate-Readme

一个智能的项目文档生成工具，能够自动分析你的项目结构并使用 Ollama 本地大语言模型生成专业的 R## 工作原理

1. **README 检测**: 首先扫描目标文件夹中的现有 README 文件：
   - 检查文件大小、行数和内容质量
   - 如果发现完整的 README 文件会跳过生成
   - 支持多种 README 文件格式（.md, .txt, .rst 等）

2. **项目分析**: 扫描目标文件夹，收集以下信息：
   - 目录结构（使用 tree 命令或 find）
   - 文件类型统计
   - 重要配置文件检测
   - 主要编程语言识别

3. **信息整理**: 将分析结果整理成结构化报告

4. **双语生成**:
   - 使用 Ollama 模型分别生成中英文版本的 README
   - 根据用户选择的语言参数决定显示顺序
   - 将两个版本合并成一个文件

5. **文件输出**: 将生成的双语 README 保存到目标项目文件夹特性

- 🔍 **智能分析**: 自动扫描项目文件夹，分析文件类型、目录结构和编程语言
- 🤖 **AI 生成**: 集成 Ollama 本地大语言模型，生成高质量的 README 内容
- 🌐 **双语支持**: 自动生成包含中英文双语版本的 README 文件
- 📋 **智能检测**: 自动检测现有 README 文件，避免覆盖完整的文档
- ⚙️ **灵活配置**: 支持自定义模型、输出路径和默认显示语言
- 🎯 **即开即用**: 简单的命令行界面，无需复杂配置
- 📁 **智能输出**: 生成的 README 文件自动保存到目标项目文件夹
- 🔄 **强制模式**: 支持强制重新生成，覆盖现有文件

## 前置要求

- macOS/Linux 系统
- [Ollama](https://ollama.ai/) 已安装并运行
- 已下载所需的语言模型（默认使用 qwen2.5:7b）

## 安装

1. 克隆此仓库：
```bash
git clone https://github.com/your-username/Auto-Generate-Readme.git
cd Auto-Generate-Readme
```

2. 确保脚本有执行权限：
```bash
chmod +x run.sh
```

## 使用方法

### 基础用法

```bash
./run.sh /path/to/your/project
```

### 高级用法

```bash
# 基本用法（会自动检测现有 README）
./run.sh /path/to/your/project

# 强制重新生成（覆盖现有 README）
./run.sh /path/to/your/project -f

# 生成双语 README（中文在前）
./run.sh /path/to/your/project -l chinese

# 指定不同的模型
./run.sh /path/to/your/project -m llama3:8b

# 指定输出文件路径
./run.sh /path/to/your/project -o /custom/path/README.md
```

### 参数说明

| 参数 | 描述 | 默认值 |
|------|------|--------|
| `-m, --model` | 指定 Ollama 模型 | qwen2.5:7b |
| `-o, --output` | 指定输出文件路径 | {项目文件夹}/README.md |
| `-l, --lang` | 指定默认显示语言 (english/chinese) | english |
| `-f, --force` | 强制重新生成，忽略现有 README 文件 | false |
| `-h, --help` | 显示帮助信息 | - |

**注意**:
- 无论选择哪种语言，都会生成包含中英文双语版本的 README 文件，指定的语言将作为默认显示在前面的版本
- 脚本会自动检测现有 README 文件，如果发现完整的文档会跳过生成，使用 `-f` 参数可强制重新生成

## 工作原理

1. **项目分析**: 扫描目标文件夹，收集以下信息：
   - 目录结构（使用 tree 命令或 find）
   - 文件类型统计
   - 重要配置文件检测
   - 主要编程语言识别

2. **信息整理**: 将分析结果整理成结构化报告

3. **双语生成**:
   - 使用 Ollama 模型分别生成中英文版本的 README
   - 根据用户选择的语言参数决定显示顺序
   - 将两个版本合并成一个文件

4. **文件输出**: 将生成的双语 README 保存到目标项目文件夹

## 示例输出

生成的双语 README 将包含：

### 英文版本（默认在前）
- Project title and description
- Features and functionality
- Installation instructions
- Usage examples
- Project structure explanation
- Dependencies and requirements
- Contributing guidelines
- License information

### 中文版本（分割线后）
- 项目标题和描述
- 功能特性列表
- 安装说明
- 使用示例
- 项目结构说明
- 依赖要求
- 贡献指南
- 许可证信息

两个版本之间用分割线 `---` 分隔，方便阅读和导航。

## 支持的项目类型

- Python 项目 (Django, Flask, FastAPI 等)
- JavaScript/TypeScript 项目 (React, Vue, Node.js 等)
- Go 项目
- Java 项目 (Spring, Maven, Gradle 等)
- C/C++ 项目
- Rust 项目
- 以及其他各种编程语言项目

## 故障排除

### Ollama 相关问题

1. 确保 Ollama 服务正在运行：
```bash
ollama serve
```

2. 检查可用模型：
```bash
ollama list
```

3. 下载所需模型：
```bash
ollama pull qwen2.5:7b
```

### 权限问题

如果遇到权限错误，请确保脚本有执行权限：
```bash
chmod +x run.sh
```

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。
