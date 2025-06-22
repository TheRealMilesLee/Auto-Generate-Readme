# Copilot 项目指令

这是一个 README 自动生成工具的 Python 项目。请在为此项目生成代码时遵循以下指导原则：

## 项目结构
- `main.py` - 主入口文件，命令行界面
- `readme_generator/` - 核心包目录
  - `core.py` - 主要生成逻辑
  - `config.py` - 配置管理
  - `utils.py` - 工具函数和辅助类
- `config.yaml` - 默认配置文件
- `requirements.txt` - 项目依赖

## 代码风格
- 使用类型提示
- 遵循 PEP 8 代码规范
- 使用 docstring 文档化函数和类
- 优雅的错误处理
- 使用日志记录而不是 print

## 功能特性
- 模块化设计，易于扩展
- 支持多种配置格式 (YAML, JSON, TOML)
- 自动检测项目信息
- 可自定义模板
- 丰富的命令行选项
- 预览功能
