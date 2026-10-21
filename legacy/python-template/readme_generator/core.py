"""
核心功能模块
包含 README 生成的主要逻辑
"""

import logging
import os
import re
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

try:
    import git
    GIT_AVAILABLE = True
except ImportError:
    GIT_AVAILABLE = False

from jinja2 import Environment, FileSystemLoader, Template

from .config import Config
from .utils import BadgeGenerator, ProjectAnalyzer

logger = logging.getLogger(__name__)

class ReadmeGenerator:
    """README 生成器主类"""

    def __init__(self, config: Config):
        self.config = config
        self.project_analyzer = ProjectAnalyzer(config.project_root, config.exclude_files)
        self.badge_generator = BadgeGenerator()

        # 初始化模板环境
        self._setup_template_environment()

    def _setup_template_environment(self):
        """设置 Jinja2 模板环境"""
        if self.config.template_path and self.config.template_path.exists():
            # 使用自定义模板
            template_dir = self.config.template_path.parent
            template_name = self.config.template_path.name
            env = Environment(loader=FileSystemLoader(template_dir))
            self.template = env.get_template(template_name)
            logger.info(f"使用自定义模板: {self.config.template_path}")
        else:
            # 使用默认模板
            self.template = Template(self._get_default_template())
            logger.info("使用默认模板")

    def generate(self) -> Path:
        """生成 README 文件"""
        logger.info("开始生成 README...")

        # 收集项目信息
        project_info = self._collect_project_info()

        # 渲染模板
        content = self.template.render(**project_info)

        # 写入文件
        output_path = self.config.output_path
        with open(output_path, 'w', encoding='utf-8') as f:
            f.write(content)

        logger.info(f"README 生成完成: {output_path}")
        return output_path

    def preview(self) -> str:
        """预览生成的内容"""
        logger.info("生成预览...")
        project_info = self._collect_project_info()
        return self.template.render(**project_info)

    def _collect_project_info(self) -> Dict[str, Any]:
        """收集项目信息"""
        info = {
            'project_name': self.config.project_name or self._detect_project_name(),
            'project_description': self.config.project_description or self._detect_project_description(),
            'author': self.config.author,
            'license': self.config.license,
            'python_version': self.config.python_version,
            'generated_date': datetime.now().strftime('%Y-%m-%d %H:%M:%S'),
            'include_badges': self.config.include_badges,
            'include_toc': self.config.include_toc,
            'include_installation': self.config.include_installation,
            'include_usage': self.config.include_usage,
            'include_api_docs': self.config.include_api_docs,
            'include_contributing': self.config.include_contributing,
            'include_changelog': self.config.include_changelog,
        }

        # Git 信息
        if self.config.git_auto_detect and GIT_AVAILABLE:
            git_info = self._detect_git_info()
            info.update(git_info)
        else:
            info.update({
                'github_username': self.config.github_username,
                'repository_name': self.config.repository_name,
                'git_url': f"https://github.com/{self.config.github_username}/{self.config.repository_name}"
            })

        # 项目结构分析
        info['project_structure'] = self.project_analyzer.get_structure()
        info['dependencies'] = self.project_analyzer.get_dependencies()
        info['entry_points'] = self.project_analyzer.get_entry_points()

        # 生成徽章
        if self.config.include_badges:
            info['badges'] = self.badge_generator.generate_badges(info)

        # 自定义章节
        info['custom_sections'] = self.config.custom_sections

        return info

    def _detect_project_name(self) -> str:
        """自动检测项目名称"""
        # 从 setup.py 检测
        setup_py = self.config.project_root / 'setup.py'
        if setup_py.exists():
            try:
                with open(setup_py, 'r', encoding='utf-8') as f:
                    content = f.read()
                    match = re.search(r'name\s*=\s*["\']([^"\']+)["\']', content)
                    if match:
                        return match.group(1)
            except Exception:
                pass

        # 从 pyproject.toml 检测
        pyproject = self.config.project_root / 'pyproject.toml'
        if pyproject.exists():
            try:
                import toml
                with open(pyproject, 'r', encoding='utf-8') as f:
                    data = toml.load(f)
                    if 'project' in data and 'name' in data['project']:
                        return data['project']['name']
                    elif 'tool' in data and 'poetry' in data['tool'] and 'name' in data['tool']['poetry']:
                        return data['tool']['poetry']['name']
            except Exception:
                pass

        # 从目录名检测
        return self.config.project_root.name

    def _detect_project_description(self) -> str:
        """自动检测项目描述"""
        # 从 setup.py 检测
        setup_py = self.config.project_root / 'setup.py'
        if setup_py.exists():
            try:
                with open(setup_py, 'r', encoding='utf-8') as f:
                    content = f.read()
                    match = re.search(r'description\s*=\s*["\']([^"\']+)["\']', content)
                    if match:
                        return match.group(1)
            except Exception:
                pass

        # 从 pyproject.toml 检测
        pyproject = self.config.project_root / 'pyproject.toml'
        if pyproject.exists():
            try:
                import toml
                with open(pyproject, 'r', encoding='utf-8') as f:
                    data = toml.load(f)
                    if 'project' in data and 'description' in data['project']:
                        return data['project']['description']
                    elif 'tool' in data and 'poetry' in data['tool'] and 'description' in data['tool']['poetry']:
                        return data['tool']['poetry']['description']
            except Exception:
                pass

        return "一个 Python 项目"

    def _detect_git_info(self) -> Dict[str, str]:
        """检测 Git 信息"""
        if not GIT_AVAILABLE:
            return {}

        try:
            repo = git.Repo(self.config.project_root)
            remote_url = repo.remotes.origin.url

            # 解析 GitHub URL
            if 'github.com' in remote_url:
                # 处理 SSH 和 HTTPS URL
                if remote_url.startswith('git@'):
                    # SSH: git@github.com:username/repo.git
                    match = re.search(r'git@github\.com:([^/]+)/([^.]+)', remote_url)
                else:
                    # HTTPS: https://github.com/username/repo.git
                    match = re.search(r'github\.com/([^/]+)/([^.]+)', remote_url)

                if match:
                    username = match.group(1)
                    repo_name = match.group(2).replace('.git', '')
                    return {
                        'github_username': username,
                        'repository_name': repo_name,
                        'git_url': f"https://github.com/{username}/{repo_name}"
                    }
        except Exception as e:
            logger.warning(f"无法检测 Git 信息: {e}")

        return {}

    def _get_default_template(self) -> str:
        """获取默认模板"""
        return '''# {{ project_name }}

{{ project_description }}

{% if include_badges and badges %}
{% for badge in badges %}
{{ badge }}
{% endfor %}

{% endif %}
{% if include_toc %}
## 目录

- [安装](#安装)
- [使用](#使用)
{% if include_api_docs %}
- [API 文档](#api-文档)
{% endif %}
{% if include_contributing %}
- [贡献](#贡献)
{% endif %}
- [许可证](#许可证)

{% endif %}
## 特性

- ✨ 功能特性 1
- 🚀 功能特性 2
- 📦 功能特性 3

{% if include_installation %}
## 安装

### 环境要求

- Python {{ python_version }}

### 安装方法

```bash
# 克隆仓库
git clone {{ git_url }}
cd {{ repository_name }}

# 安装依赖
pip install -r requirements.txt
```

{% endif %}
{% if include_usage %}
## 使用

### 基本用法

```python
# 添加使用示例
import {{ project_name.lower().replace('-', '_') }}

# 示例代码
```

### 命令行使用

```bash
python main.py --help
```

{% endif %}
{% if project_structure %}
## 项目结构

```
{{ project_structure }}
```

{% endif %}
{% if dependencies %}
## 依赖

{% for dep in dependencies %}
- {{ dep }}
{% endfor %}

{% endif %}
{% if custom_sections %}
{% for section in custom_sections %}
## {{ section.title }}

{{ section.content }}

{% endfor %}
{% endif %}
{% if include_api_docs %}
## API 文档

详细的 API 文档请参考 [docs/](docs/) 目录。

{% endif %}
{% if include_contributing %}
## 贡献

欢迎贡献！请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详情。

### 开发环境设置

```bash
# 克隆仓库
git clone {{ git_url }}
cd {{ repository_name }}

# 创建虚拟环境
python -m venv venv
source venv/bin/activate  # Windows: venv\\Scripts\\activate

# 安装开发依赖
pip install -r requirements-dev.txt
```

{% endif %}
## 许可证

本项目采用 {{ license }} 许可证 - 详见 [LICENSE](LICENSE) 文件。

## 作者

{{ author }}

---

*本 README 由 [README Generator](https://github.com/your-username/readme-generator) 自动生成于 {{ generated_date }}*
'''
'''
