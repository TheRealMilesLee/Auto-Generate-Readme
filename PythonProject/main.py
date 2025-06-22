#!/usr/bin/env python3
"""
README 自动生成工具
一个用于自动生成和更新项目 README 文件的 Python 工具
"""

import logging
from pathlib import Path

import click
from rich.console import Console
from rich.logging import RichHandler

from readme_generator.config import Config
from readme_generator.core import ReadmeGenerator
from readme_generator.utils import setup_logging

console = Console()


@click.command()
@click.option('--config',
              '-c',
              type=click.Path(exists=True),
              help='配置文件路径 (默认: config.yaml)')
@click.option('--output',
              '-o',
              type=click.Path(),
              help='输出 README 文件路径 (默认: README.md)')
@click.option('--template',
              '-t',
              type=click.Path(exists=True),
              help='自定义模板文件路径')
@click.option('--verbose', '-v', is_flag=True, help='详细输出模式')
@click.option('--dry-run', is_flag=True, help='仅预览，不实际生成文件')
def main(config, output, template, verbose, dry_run):
  """README 自动生成工具"""

  # 设置日志
  setup_logging(verbose)
  logger = logging.getLogger(__name__)

  try:
    # 加载配置
    config_path = config or 'config.yaml'
    app_config = Config.load(config_path)

    # 设置输出路径
    if output:
      app_config.output_path = Path(output)

    # 设置自定义模板
    if template:
      app_config.template_path = Path(template)

    # 创建生成器
    generator = ReadmeGenerator(app_config)

    if dry_run:
      console.print("[yellow]🔍 预览模式 - 不会生成实际文件[/yellow]")
      preview = generator.preview()
      console.print(preview)
    else:
      # 生成 README
      output_file = generator.generate()
      console.print(f"[green]✅ README 已成功生成: {output_file}[/green]")

  except Exception as e:
    logger.error(f"生成失败: {e}")
    console.print(f"[red]❌ 错误: {e}[/red]")
    raise click.Abort()


if __name__ == '__main__':
  main()
  main()
