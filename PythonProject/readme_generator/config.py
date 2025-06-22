"""
配置管理模块
负责加载和管理应用程序配置
"""

import json
import logging
from pathlib import Path
from typing import Any, Dict, Optional

import toml
import yaml

logger = logging.getLogger(__name__)

class Config:
    """配置类，用于管理应用程序设置"""

    def __init__(self, data: Dict[str, Any] = None):
        self.data = data or {}

        # 默认配置
        self.project_name = self.data.get('project_name', '')
        self.project_description = self.data.get('project_description', '')
        self.author = self.data.get('author', '')
        self.license = self.data.get('license', 'MIT')
        self.python_version = self.data.get('python_version', '3.8+')

        # 路径配置
        self.project_root = Path(self.data.get('project_root', '.'))
        self.output_path = Path(self.data.get('output_path', 'README.md'))
        self.template_path = Path(self.data.get('template_path', ''))

        # 功能开关
        self.include_badges = self.data.get('include_badges', True)
        self.include_toc = self.data.get('include_toc', True)
        self.include_installation = self.data.get('include_installation', True)
        self.include_usage = self.data.get('include_usage', True)
        self.include_api_docs = self.data.get('include_api_docs', False)
        self.include_contributing = self.data.get('include_contributing', True)
        self.include_changelog = self.data.get('include_changelog', False)

        # Git 配置
        self.git_auto_detect = self.data.get('git_auto_detect', True)
        self.github_username = self.data.get('github_username', '')
        self.repository_name = self.data.get('repository_name', '')

        # 模板配置
        self.custom_sections = self.data.get('custom_sections', [])
        self.exclude_files = self.data.get('exclude_files', ['.git', '__pycache__', '.vscode'])

    @classmethod
    def load(cls, config_file: str) -> 'Config':
        """从配置文件加载配置"""
        config_path = Path(config_file)

        if not config_path.exists():
            logger.warning(f"配置文件 {config_file} 不存在，使用默认配置")
            return cls()

        try:
            with open(config_path, 'r', encoding='utf-8') as f:
                if config_path.suffix.lower() == '.yaml' or config_path.suffix.lower() == '.yml':
                    data = yaml.safe_load(f)
                elif config_path.suffix.lower() == '.json':
                    data = json.load(f)
                elif config_path.suffix.lower() == '.toml':
                    data = toml.load(f)
                else:
                    raise ValueError(f"不支持的配置文件格式: {config_path.suffix}")

            logger.info(f"已加载配置文件: {config_file}")
            return cls(data)

        except Exception as e:
            logger.error(f"加载配置文件失败: {e}")
            logger.info("使用默认配置")
            return cls()

    def save(self, config_file: str):
        """保存配置到文件"""
        config_path = Path(config_file)

        try:
            with open(config_path, 'w', encoding='utf-8') as f:
                if config_path.suffix.lower() == '.yaml' or config_path.suffix.lower() == '.yml':
                    yaml.dump(self.data, f, default_flow_style=False, allow_unicode=True)
                elif config_path.suffix.lower() == '.json':
                    json.dump(self.data, f, indent=2, ensure_ascii=False)
                elif config_path.suffix.lower() == '.toml':
                    toml.dump(self.data, f)

            logger.info(f"配置已保存到: {config_file}")

        except Exception as e:
            logger.error(f"保存配置文件失败: {e}")
            raise

    def to_dict(self) -> Dict[str, Any]:
        """将配置转换为字典"""
        return {
            'project_name': self.project_name,
            'project_description': self.project_description,
            'author': self.author,
            'license': self.license,
            'python_version': self.python_version,
            'project_root': str(self.project_root),
            'output_path': str(self.output_path),
            'template_path': str(self.template_path),
            'include_badges': self.include_badges,
            'include_toc': self.include_toc,
            'include_installation': self.include_installation,
            'include_usage': self.include_usage,
            'include_api_docs': self.include_api_docs,
            'include_contributing': self.include_contributing,
            'include_changelog': self.include_changelog,
            'git_auto_detect': self.git_auto_detect,
            'github_username': self.github_username,
            'repository_name': self.repository_name,
            'custom_sections': self.custom_sections,
            'exclude_files': self.exclude_files
        }
        }
