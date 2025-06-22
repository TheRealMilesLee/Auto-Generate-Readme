"""
工具模块
包含项目分析、徽章生成等辅助功能
"""

import logging
import os
import re
from pathlib import Path
from typing import Any, Dict, List, Optional

logger = logging.getLogger(__name__)


def setup_logging(verbose: bool = False):
  """设置日志配置"""
  log_level = logging.DEBUG if verbose else logging.INFO

  # 配置 rich 处理器
  try:
    from rich.logging import RichHandler
    logging.basicConfig(level=log_level,
                        format="%(message)s",
                        datefmt="[%X]",
                        handlers=[RichHandler(rich_tracebacks=True)])
  except ImportError:
    # 如果没有 rich，使用标准处理器
    logging.basicConfig(
        level=log_level,
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')


class ProjectAnalyzer:
  """项目分析器"""

  def __init__(self, project_root: Path, exclude_files: List[str] = None):
    self.project_root = project_root
    self.exclude_files = exclude_files or []

  def get_structure(self, max_depth: int = 3) -> str:
    """获取项目结构树"""

    def _build_tree(path: Path,
                    prefix: str = "",
                    depth: int = 0) -> List[str]:
      if depth > max_depth:
        return []

      items = []
      try:
        children = sorted(
            [p for p in path.iterdir() if not self._should_exclude(p)])

        for i, child in enumerate(children):
          is_last = i == len(children) - 1
          current_prefix = "└── " if is_last else "├── "
          items.append(f"{prefix}{current_prefix}{child.name}")

          if child.is_dir() and depth < max_depth:
            extension_prefix = "    " if is_last else "│   "
            items.extend(
                _build_tree(child, prefix + extension_prefix, depth + 1))
      except PermissionError:
        pass

      return items

    tree_lines = [self.project_root.name + "/"]
    tree_lines.extend(_build_tree(self.project_root))
    return "\n".join(tree_lines)

  def get_dependencies(self) -> List[str]:
    """获取项目依赖"""
    dependencies = []

    # 从 requirements.txt 读取
    req_file = self.project_root / 'requirements.txt'
    if req_file.exists():
      try:
        with open(req_file, 'r', encoding='utf-8') as f:
          for line in f:
            line = line.strip()
            if line and not line.startswith('#'):
              dependencies.append(line)
      except Exception as e:
        logger.warning(f"读取 requirements.txt 失败: {e}")

    # 从 setup.py 读取
    setup_py = self.project_root / 'setup.py'
    if setup_py.exists():
      try:
        with open(setup_py, 'r', encoding='utf-8') as f:
          content = f.read()
          # 简单的正则匹配
          matches = re.findall(r'install_requires\s*=\s*\[([^\]]+)\]',
                               content, re.DOTALL)
          if matches:
            deps_str = matches[0]
            deps = re.findall(r'["\']([^"\']+)["\']', deps_str)
            dependencies.extend(deps)
      except Exception as e:
        logger.warning(f"读取 setup.py 依赖失败: {e}")

    # 从 pyproject.toml 读取
    pyproject = self.project_root / 'pyproject.toml'
    if pyproject.exists():
      try:
        import toml
        with open(pyproject, 'r', encoding='utf-8') as f:
          data = toml.load(f)
          # Poetry 格式
          if 'tool' in data and 'poetry' in data['tool']:
            poetry_deps = data['tool']['poetry'].get('dependencies', {})
            for name, version in poetry_deps.items():
              if name != 'python':
                if isinstance(version, str):
                  dependencies.append(f"{name}{version}")
                else:
                  dependencies.append(name)

          # PEP 621 格式
          if 'project' in data:
            project_deps = data['project'].get('dependencies', [])
            dependencies.extend(project_deps)

      except ImportError:
        logger.warning("需要安装 toml 包来解析 pyproject.toml")
      except Exception as e:
        logger.warning(f"读取 pyproject.toml 依赖失败: {e}")

    return list(set(dependencies))  # 去重

  def get_entry_points(self) -> List[str]:
    """获取入口点文件"""
    entry_points = []
    common_entries = ['main.py', 'app.py', '__main__.py', 'cli.py', 'run.py']

    for entry in common_entries:
      entry_path = self.project_root / entry
      if entry_path.exists():
        entry_points.append(entry)

    return entry_points

  def _should_exclude(self, path: Path) -> bool:
    """判断是否应该排除某个路径"""
    return path.name in self.exclude_files


class BadgeGenerator:
  """徽章生成器"""

  def generate_badges(self, project_info: Dict[str, Any]) -> List[str]:
    """生成项目徽章"""
    badges = []

    github_username = project_info.get('github_username', '')
    repository_name = project_info.get('repository_name', '')

    if github_username and repository_name:
      # Python 版本徽章
      python_version = project_info.get('python_version', '3.8+')
      badges.append(
          f"![Python Version](https://img.shields.io/badge/python-{python_version}-blue.svg)"
      )

      # GitHub 徽章
      badges.append(
          f"![GitHub stars](https://img.shields.io/github/stars/{github_username}/{repository_name})"
      )
      badges.append(
          f"![GitHub forks](https://img.shields.io/github/forks/{github_username}/{repository_name})"
      )
      badges.append(
          f"![GitHub issues](https://img.shields.io/github/issues/{github_username}/{repository_name})"
      )

      # 许可证徽章
      license_name = project_info.get('license', 'MIT')
      badges.append(
          f"![License](https://img.shields.io/github/license/{github_username}/{repository_name})"
      )

      # 最后提交徽章
      badges.append(
          f"![Last Commit](https://img.shields.io/github/last-commit/{github_username}/{repository_name})"
      )

    return badges


class TemplateManager:
  """模板管理器"""

  @staticmethod
  def create_default_config(output_path: str = "config.yaml"):
    """创建默认配置文件"""
    config_content = """# README 生成器配置
project_name: ""
project_description: ""
author: ""
license: "MIT"
python_version: "3.8+"

# 路径配置
project_root: "."
output_path: "README.md"
template_path: ""

# 功能开关
include_badges: true
include_toc: true
include_installation: true
include_usage: true
include_api_docs: false
include_contributing: true
include_changelog: false

# Git 配置
git_auto_detect: true
github_username: ""
repository_name: ""

# 自定义章节
custom_sections: []
  # - title: "自定义章节"
  #   content: "章节内容"

# 排除文件
exclude_files:
  - ".git"
  - "__pycache__"
  - ".vscode"
  - "node_modules"
  - ".pytest_cache"
"""

    with open(output_path, 'w', encoding='utf-8') as f:
      f.write(config_content)

    logger.info(f"默认配置文件已创建: {output_path}")


def validate_project_structure(project_root: Path) -> List[str]:
  """验证项目结构并返回建议"""
  suggestions = []

  # 检查必要文件
  essential_files = {
      'requirements.txt': '依赖文件',
      'README.md': 'README 文件',
      'LICENSE': '许可证文件',
      '.gitignore': 'Git 忽略文件'
  }

  for filename, description in essential_files.items():
    if not (project_root / filename).exists():
      suggestions.append(f"建议添加 {filename} ({description})")

  # 检查 Python 项目结构
  python_files = list(project_root.glob('*.py'))
  if not python_files and not any(project_root.glob('src/*.py')):
    suggestions.append("未找到 Python 源文件")

  # 检查测试目录
  test_dirs = ['tests', 'test']
  if not any((project_root / dirname).exists() for dirname in test_dirs):
    suggestions.append("建议添加测试目录 (tests/)")

  return suggestions
