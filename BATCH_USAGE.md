# README 批量检测和重新生成工具使用说明

## 概述

这套工具用于批量检测和修复包含 Ollama 输出杂质的 README 文件。主要解决的问题包括：

- 去除 "Thinking..." 和 "...done thinking." 等思考过程
- 清理 ANSI 控制字符（颜色代码）
- 移除 AI 元评论和分析性语言
- 清理孤立的代码块标记
- 保持 README 的 Markdown 结构和实际代码块

## 文件说明

### 主要脚本

1. **`run.sh`** - 核心的 README 生成脚本
   - 已修复临时文件清理问题
   - 已优化 Ollama 输出过滤逻辑
   - 支持干净的 README 生成

2. **`batch_regenerate.sh`** - 批量检测和重新生成脚本
   - 扫描指定目录下的所有项目
   - 检测 README 是否包含杂质内容
   - 批量重新生成污染的 README
   - 提供详细的日志和报告

3. **`quick_scan.sh`** - 快速扫描脚本
   - 快速预览哪些项目需要重新生成
   - 简化的输出格式
   - 适合快速检查

### 测试脚本

- `test_filter.sh`, `test_simple_filter.sh`, `test_new_filter.sh`, `test_final_filter.sh` - 各种过滤逻辑的测试脚本

## 使用方法

### 1. 快速扫描（推荐先运行）

```bash
# 扫描默认目录 /Users/silverhand/Developer/SourceRepo
./quick_scan.sh

# 扫描指定目录
./quick_scan.sh /path/to/your/repos
```

输出示例：
```
快速扫描 README 杂质内容
扫描目录: /Users/silverhand/Developer/SourceRepo

❌ ProjectA - /Users/silverhand/Developer/SourceRepo/ProjectA
✅ ProjectB
❌ ProjectC - /Users/silverhand/Developer/SourceRepo/ProjectC
✅ ProjectD

========== 扫描结果 ==========
总项目数: 4
包含杂质: 2
干净项目: 2
```

### 2. 批量重新生成

#### 仅扫描模式（不修改文件）
```bash
# 仅扫描，生成详细报告
./batch_regenerate.sh --scan-only

# 扫描指定目录
./batch_regenerate.sh --scan-only /path/to/your/repos
```

#### 完整重新生成模式
```bash
# 扫描并重新生成（会询问确认）
./batch_regenerate.sh --regenerate

# 扫描指定目录并重新生成
./batch_regenerate.sh --regenerate /path/to/your/repos

# 使用默认目录
./batch_regenerate.sh
```

### 3. 单个项目重新生成

```bash
# 对单个项目重新生成 README
./run.sh /path/to/project
```

## 安全特性

### 备份机制
- 重新生成前会自动备份原始 README 文件
- 备份文件格式：`README.md.backup.YYYYMMDD_HHMMSS`
- 如果重新生成失败，会自动恢复备份

### 验证机制
- 重新生成后会验证新的 README 是否仍包含杂质
- 如果验证失败，会在日志中记录警告

### 日志记录
- 所有操作都会记录在 `batch_regenerate.log` 中
- 详细报告保存在 `batch_report.txt` 中

## 杂质检测模式

脚本会检测以下类型的杂质内容：

1. **思考过程**
   - `Thinking...`
   - `...done thinking.`

2. **AI 元评论**
   - `Let me analyze`
   - `I'll analyze`
   - `Looking at`
   - `Based on the`
   - `Here's.*README`
   - `I'll create`
   - `Let me create`

3. **ANSI 控制字符**
   - 各种颜色代码如 `[90m`, `[0m`, `[1m` 等

4. **格式问题**
   - 孤立的 ```` 行
   - 不当的代码块包装

5. **日志和分析性语言**
   - `analyze.*project`
   - `examination.*code`
   - `This project.*appears to be`

## 输出文件

运行批量处理后会生成：

1. **`batch_regenerate.log`** - 详细的操作日志
2. **`batch_report.txt`** - 项目状态报告，格式：
   ```
   CLEAN: /path/to/clean/project
   CONTAMINATED: /path/to/contaminated/project
   ```

## 故障排除

### 常见问题

1. **权限问题**
   ```bash
   chmod +x batch_regenerate.sh quick_scan.sh run.sh
   ```

2. **Ollama 未运行**
   ```bash
   # 确保 Ollama 服务正在运行
   ollama serve
   ```

3. **脚本路径问题**
   - 确保在 Auto-Generate-Readme 目录中运行脚本
   - 或使用绝对路径

### 查看日志
```bash
# 查看最近的操作日志
tail -f batch_regenerate.log

# 查看报告摘要
cat batch_report.txt
```

## 建议的工作流程

1. **首次使用**
   ```bash
   # 1. 快速扫描，了解整体情况
   ./quick_scan.sh

   # 2. 仅扫描模式，生成详细报告
   ./batch_regenerate.sh --scan-only

   # 3. 检查报告和日志
   cat batch_report.txt
   ```

2. **批量修复**
   ```bash
   # 执行批量重新生成
   ./batch_regenerate.sh --regenerate
   ```

3. **验证结果**
   ```bash
   # 再次快速扫描，确认修复效果
   ./quick_scan.sh
   ```

## 注意事项

- 重新生成过程可能需要较长时间，取决于项目数量和 Ollama 响应速度
- 建议在非高峰时间运行批量操作
- 重新生成前请确保项目代码状态是你想要分析的版本
- 如有重要的手动编辑的 README，请在操作前进行额外备份
