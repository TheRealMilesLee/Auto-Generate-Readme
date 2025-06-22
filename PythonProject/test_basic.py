#!/usr/bin/env python3
import os
import sys

# 添加当前目录到 Python 路径
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

try:
  from readme_generator.config import Config
  from readme_generator.core import ReadmeGenerator
  print("✅ 所有模块导入成功")

  # 测试配置加载
  config = Config.load('config.yaml')
  print(f"✅ 配置加载成功: {config.project_name or '(未设置)'}")

  # 测试生成器创建
  generator = ReadmeGenerator(config)
  print("✅ 生成器创建成功")

  # 测试预览
  preview = generator.preview()
  print("✅ 预览生成成功")
  print(f"预览长度: {len(preview)} 字符")

  print("\n" + "=" * 50)
  print("🎉 测试完成 - 项目工作正常!")

except Exception as e:
  print(f"❌ 错误: {e}")
  import traceback
  traceback.print_exc()
