#!/usr/bin/env python3
"""
示例脚本：演示如何使用 README 生成器
"""

import logging
from pathlib import Path

from readme_generator.config import Config
from readme_generator.core import ReadmeGenerator
from readme_generator.utils import TemplateManager, validate_project_structure


def main():
  # 设置日志
  logging.basicConfig(level=logging.INFO)
  logger = logging.getLogger(__name__)

  # 项目根目录
  project_root = Path(".")

  print("🚀 README 生成器示例")
  print("=" * 50)

  # 1. 验证项目结构
  print("\n1. 验证项目结构...")
  suggestions = validate_project_structure(project_root)
  if suggestions:
    print("建议:")
    for suggestion in suggestions:
      print(f"  - {suggestion}")
  else:
    print("  ✅ 项目结构良好")

  # 2. 创建默认配置（如果不存在）
  config_file = "config.yaml"
  if not Path(config_file).exists():
    print(f"\n2. 创建默认配置文件: {config_file}")
    TemplateManager.create_default_config(config_file)
  else:
    print(f"\n2. 使用现有配置文件: {config_file}")

  # 3. 加载配置
  print("\n3. 加载配置...")
  config = Config.load(config_file)
  print(f"  - 项目名称: {config.project_name or '(自动检测)'}")
  print(f"  - 输出路径: {config.output_path}")
  print(f"  - 包含徽章: {config.include_badges}")

  # 4. 生成预览
  print("\n4. 生成预览...")
  generator = ReadmeGenerator(config)
  preview = generator.preview()

  print("\n" + "=" * 50)
  print("预览内容:")
  print("=" * 50)
  print(preview[:500] + "..." if len(preview) > 500 else preview)

  # 5. 询问是否生成实际文件
  response = input("\n是否生成实际的 README.md 文件? (y/N): ").strip().lower()
  if response in ['y', 'yes', '是']:
    print("\n5. 生成 README.md...")
    output_path = generator.generate()
    print(f"  ✅ 已生成: {output_path}")
  else:
    print("\n5. 跳过文件生成")

  print("\n✨ 示例完成!")


if __name__ == "__main__":
  main()
