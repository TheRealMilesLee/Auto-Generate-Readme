#!/bin/bash

# Auto-Generate-Readme Script
# 分析目标文件夹并使用 Ollama 生成 README 文件

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 帮助信息
show_help() {
  echo -e "${BLUE}Auto-Generate-Readme Tool${NC}"
  echo ""
  echo "用法: $0 <目标文件夹路径> [选项]"
  echo ""
  echo "选项:"
  echo "  -h, --help     显示帮助信息"
  echo "  -m, --model    指定 Ollama 模型 (默认: qwen2.5:7b)"
  echo "  -o, --output   指定输出文件路径 (默认: <目标文件夹>/README.md)"
  echo "  -l, --lang     指定默认显示语言 (english/chinese, 默认: english)"
  echo ""
  echo "注意: 无论选择哪种语言，都会生成包含中英文双语版本的 README 文件"
  echo "      指定的语言将作为默认显示在前面的版本"
  echo ""
  echo "示例:"
  echo "  $0 /path/to/project"
  echo "  $0 /path/to/project -m llama3:8b -l chinese"
  echo "  $0 /path/to/project -o /custom/path/README.md"
}

# 默认参数
OLLAMA_MODEL="qwen2.5:7b"
OUTPUT_FILE=""
LANGUAGE="english" # 默认英文在前

# 解析命令行参数
while [[ $# -gt 0 ]]; do
  case $1 in
  -h | --help)
    show_help
    exit 0
    ;;
  -m | --model)
    OLLAMA_MODEL="$2"
    shift 2
    ;;
  -o | --output)
    OUTPUT_FILE="$2"
    shift 2
    ;;
  -l | --lang)
    LANGUAGE="$2"
    shift 2
    ;;
  -*)
    echo -e "${RED}错误: 未知选项 $1${NC}"
    show_help
    exit 1
    ;;
  *)
    if [[ -z "$TARGET_DIR" ]]; then
      TARGET_DIR="$1"
    else
      echo -e "${RED}错误: 只能指定一个目标文件夹${NC}"
      exit 1
    fi
    shift
    ;;
  esac
done

# 检查是否提供了目标文件夹
if [[ -z "$TARGET_DIR" ]]; then
  echo -e "${RED}错误: 请提供目标文件夹路径${NC}"
  show_help
  exit 1
fi

# 检查目标文件夹是否存在
if [[ ! -d "$TARGET_DIR" ]]; then
  echo -e "${RED}错误: 目标文件夹 '$TARGET_DIR' 不存在${NC}"
  exit 1
fi

# 设置默认输出文件
if [[ -z "$OUTPUT_FILE" ]]; then
  OUTPUT_FILE="$TARGET_DIR/README.md"
fi

# 检查 Ollama 是否可用
check_ollama() {
  echo -e "${BLUE}检查 Ollama 服务...${NC}"
  if ! command -v ollama &>/dev/null; then
    echo -e "${RED}错误: 未找到 ollama 命令，请确保已安装 Ollama${NC}"
    exit 1
  fi

  if ! ollama list &>/dev/null; then
    echo -e "${RED}错误: Ollama 服务未运行，请启动 Ollama 服务${NC}"
    exit 1
  fi

  if ! ollama list | grep -q "$OLLAMA_MODEL"; then
    echo -e "${YELLOW}警告: 模型 '$OLLAMA_MODEL' 未找到，正在下载...${NC}"
    ollama pull "$OLLAMA_MODEL"
  fi

  echo -e "${GREEN}✓ Ollama 服务正常${NC}"
}

# 获取文件类型统计
get_file_types() {
  local dir="$1"
  echo "文件类型统计:"
  find "$dir" -type f -name ".*" -prune -o -type f -print |
    sed 's/.*\.//' |
    sort | uniq -c | sort -nr |
    head -20 |
    while read count ext; do
      if [[ -n "$ext" ]]; then
        echo "  .$ext: $count 个文件"
      else
        echo "  无扩展名: $count 个文件"
      fi
    done
}

# 获取目录结构
get_directory_structure() {
  local dir="$1"
  echo "目录结构:"
  if command -v tree &>/dev/null; then
    tree "$dir" -L 3 -I '.git|node_modules|__pycache__|.DS_Store|*.pyc|.env' --dirsfirst
  else
    find "$dir" -type d -name ".git" -prune -o -type d -print |
      grep -v ".git" |
      sort |
      sed "s|$dir||" |
      sed 's|/| |g' |
      awk '{for(i=1;i<=NF;i++) printf "  "; print $NF}'
  fi
}

# 获取重要文件列表
get_important_files() {
  local dir="$1"
  echo "重要文件:"
  local important_files=(
    "README.md" "readme.md" "README.txt"
    "package.json" "requirements.txt" "Pipfile" "poetry.lock"
    "Dockerfile" "docker-compose.yml" "docker-compose.yaml"
    "Makefile" "CMakeLists.txt" "build.gradle" "pom.xml"
    "tsconfig.json" "webpack.config.js" "vite.config.js"
    "go.mod" "Cargo.toml" "composer.json"
    ".gitignore" "LICENSE" "CHANGELOG.md"
    "main.py" "app.py" "index.js" "main.js" "index.html"
  )

  for file in "${important_files[@]}"; do
    if [[ -f "$dir/$file" ]]; then
      echo "  - $file"
    fi
  done

  # 查找其他可能的入口文件
  echo "  其他可能的入口文件:"
  find "$dir" -maxdepth 2 -type f \( -name "*.py" -o -name "*.js" -o -name "*.ts" -o -name "*.go" -o -name "*.java" -o -name "*.cpp" -o -name "*.c" \) |
    head -10 |
    sed "s|$dir/||" |
    sed 's/^/    - /'
}

# 分析代码语言
analyze_languages() {
  local dir="$1"
  echo "主要编程语言:"
  find "$dir" -type f -name ".*" -prune -o -type f -print |
    grep -E '\.(py|js|ts|java|cpp|c|go|php|rb|rs|swift|kt|scala|sh|ps1)$' |
    sed 's/.*\.//' |
    sort | uniq -c | sort -nr |
    head -5 |
    while read count ext; do
      case $ext in
      py) lang="Python" ;;
      js) lang="JavaScript" ;;
      ts) lang="TypeScript" ;;
      java) lang="Java" ;;
      cpp | cc | cxx) lang="C++" ;;
      c) lang="C" ;;
      go) lang="Go" ;;
      php) lang="PHP" ;;
      rb) lang="Ruby" ;;
      rs) lang="Rust" ;;
      swift) lang="Swift" ;;
      kt) lang="Kotlin" ;;
      scala) lang="Scala" ;;
      sh) lang="Shell Script" ;;
      ps1) lang="PowerShell" ;;
      *) lang="$ext" ;;
      esac
      echo "  - $lang: $count 个文件"
    done
}

# 分析项目信息
analyze_project() {
  local dir="$1"
  local project_name=$(basename "$dir")

  echo -e "${BLUE}分析项目: $project_name${NC}"
  echo -e "${BLUE}路径: $dir${NC}"
  echo ""

  # 创建分析报告
  local analysis_file="/tmp/project_analysis_$$.txt"

  cat >"$analysis_file" <<EOF
项目分析报告
=============

项目名称: $project_name
项目路径: $dir
分析时间: $(date '+%Y-%m-%d %H:%M:%S')

$(get_directory_structure "$dir")

$(get_file_types "$dir")

$(get_important_files "$dir")

$(analyze_languages "$dir")

EOF

  echo "$analysis_file"
}

# 生成英文 README 内容
generate_english_readme() {
  local analysis_file="$1"
  echo -e "${BLUE}生成英文版 README...${NC}"

  local prompt="You are a professional software documentation writer. Based on the following project analysis, please generate a comprehensive and well-structured README.md file. The README should include:

1. Project title and brief description
2. Features and functionality
3. Installation instructions
4. Usage examples
5. Project structure explanation
6. Dependencies and requirements
7. Contributing guidelines
8. License information

Please write in English and use proper Markdown formatting. Make the README informative, professional, and easy to understand.

Project Analysis:
$(cat "$analysis_file")

Please generate a complete README.md content:"

  local readme_content
  readme_content=$(ollama run "$OLLAMA_MODEL" "$prompt")

  if [[ $? -eq 0 && -n "$readme_content" ]]; then
    echo "$readme_content"
  else
    echo -e "${RED}错误: 生成英文 README 失败${NC}"
    exit 1
  fi
}

# 生成中文 README 内容
generate_chinese_readme() {
  local analysis_file="$1"
  echo -e "${BLUE}生成中文版 README...${NC}"

  local prompt="你是一个专业的软件文档编写专家。根据以下项目分析，请生成一个完整且结构良好的 README.md 文件。README 应该包含：

1. 项目标题和简要描述
2. 功能特性
3. 安装说明
4. 使用示例
5. 项目结构说明
6. 依赖要求
7. 贡献指南
8. 许可证信息

请使用中文编写，采用标准的 Markdown 格式。让 README 内容丰富、专业且易于理解。

项目分析：
$(cat "$analysis_file")

请生成完整的 README.md 内容："

  local readme_content
  readme_content=$(ollama run "$OLLAMA_MODEL" "$prompt")

  if [[ $? -eq 0 && -n "$readme_content" ]]; then
    echo "$readme_content"
  else
    echo -e "${RED}错误: 生成中文 README 失败${NC}"
    exit 1
  fi
}

# 生成双语 README 内容
generate_readme() {
  local analysis_file="$1"
  local project_name=$(basename "$TARGET_DIR")

  echo -e "${BLUE}使用 Ollama 生成双语 README...${NC}"

  if [[ "$LANGUAGE" == "chinese" ]]; then
    # 中文优先模式：先生成中文，再生成英文
    local chinese_content
    chinese_content=$(generate_chinese_readme "$analysis_file")

    local english_content
    english_content=$(generate_english_readme "$analysis_file")

    # 创建双语 README，中文在前
    cat >"$OUTPUT_FILE" <<EOF
$chinese_content

---

## English Version

$english_content
EOF
  else
    # 英文优先模式：先生成英文，再生成中文
    local english_content
    english_content=$(generate_english_readme "$analysis_file")

    local chinese_content
    chinese_content=$(generate_chinese_readme "$analysis_file")

    # 创建双语 README，英文在前
    cat >"$OUTPUT_FILE" <<EOF
$english_content

---

## 中文版本

$chinese_content
EOF
  fi

  echo -e "${GREEN}✓ 双语 README 文件已生成: $OUTPUT_FILE${NC}"
}

# 主函数
main() {
  echo -e "${GREEN}=== Auto-Generate-Readme Tool ===${NC}"
  echo ""

  # 检查 Ollama
  check_ollama

  # 分析项目
  local analysis_file
  analysis_file=$(analyze_project "$TARGET_DIR")

  # 生成 README
  generate_readme "$analysis_file"

  # 清理临时文件
  rm -f "$analysis_file"

  echo ""
  echo -e "${GREEN}🎉 双语 README 生成完成！${NC}"
  echo -e "${BLUE}文件位置: $OUTPUT_FILE${NC}"
  if [[ "$LANGUAGE" == "english" ]]; then
    echo -e "${YELLOW}📖 英文版本显示在前，中文版本在分割线后${NC}"
  else
    echo -e "${YELLOW}📖 中文版本显示在前，英文版本在分割线后${NC}"
  fi
  echo ""
}

# 执行主函数
main "$@"
