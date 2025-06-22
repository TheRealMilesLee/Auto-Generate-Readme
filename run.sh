#!/bin/bash

# Auto-Generate-Readme Script
# 分析目标文件夹并使用 Ollama 生成 README 文件

set -e
set -o pipefail

# 变量初始化
TARGET_DIR=""
OLLAMA_MODEL="qwen3:8b"
OUTPUT_FILE=""
LANGUAGE="english" # 默认英文在前
FORCE=false        # 默认不强制重新生成

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
  echo -e "${BLUE}[INFO]${NC} $1"
}

log_warn() {
  echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
  echo -e "${RED}[ERROR]${NC} $1"
}

log_success() {
  echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# 错误处理函数
handle_error() {
  local exit_code=$?
  local line_number=$1
  log_error "脚本在第 $line_number 行出错，退出码: $exit_code"
  cleanup
  exit $exit_code
}

# 清理函数
cleanup() {
  if [[ -n "${TEMP_ANALYSIS_FILE:-}" && -f "$TEMP_ANALYSIS_FILE" ]]; then
    rm -f "$TEMP_ANALYSIS_FILE"
  fi
}

# 设置错误处理
trap 'handle_error $LINENO' ERR
trap 'cleanup' EXIT

# 帮助信息
show_help() {
  echo -e "${BLUE}Auto-Generate-Readme Tool${NC}"
  echo ""
  echo "用法: $0 <目标文件夹路径> [选项]"
  echo ""
  echo "选项:"
  echo "  -h, --help     显示帮助信息"
  echo "  -m, --model    指定 Ollama 模型 (默认: qwen3:8b)"
  echo "  -o, --output   指定输出文件路径 (默认: <目标文件夹>/README.md)"
  echo "  -l, --lang     指定默认显示语言 (english/chinese, 默认: english)"
  echo "  -f, --force    强制重新生成，忽略现有 README 文件"
  echo ""
  echo "注意: 无论选择哪种语言，都会生成包含中英文双语版本的 README 文件"
  echo "      指定的语言将作为默认显示在前面的版本"
  echo ""
  echo "示例:"
  echo "  $0 /path/to/project"
  echo "  $0 /path/to/project -m llama3:8b -l chinese"
  echo "  $0 /path/to/project -o /custom/path/README.md"
  echo "  $0 /path/to/project -f  # 强制重新生成"
}

# 验证语言参数
validate_language() {
  local lang="$1"
  if [[ "$lang" != "english" && "$lang" != "chinese" ]]; then
    log_error "语言参数只能是 'english' 或 'chinese'"
    exit 1
  fi
}

# 验证模型名称
validate_model() {
  local model="$1"
  if [[ -z "$model" ]]; then
    log_error "模型名称不能为空"
    exit 1
  fi
  # 基本格式检查（模型名通常包含字母、数字、冒号、连字符）
  if ! [[ "$model" =~ ^[a-zA-Z0-9:._-]+$ ]]; then
    log_error "模型名称格式无效: $model"
    exit 1
  fi
}

# 解析命令行参数
parse_arguments() {
  while [[ $# -gt 0 ]]; do
    case $1 in
    -h | --help)
      show_help
      exit 0
      ;;
    -m | --model)
      if [[ -z "$2" ]]; then
        log_error "选项 -m/--model 需要参数"
        exit 1
      fi
      validate_model "$2"
      OLLAMA_MODEL="$2"
      shift 2
      ;;
    -o | --output)
      if [[ -z "$2" ]]; then
        log_error "选项 -o/--output 需要参数"
        exit 1
      fi
      OUTPUT_FILE="$2"
      shift 2
      ;;
    -l | --lang)
      if [[ -z "$2" ]]; then
        log_error "选项 -l/--lang 需要参数"
        exit 1
      fi
      validate_language "$2"
      LANGUAGE="$2"
      shift 2
      ;;
    -f | --force)
      FORCE=true
      shift
      ;;
    -*)
      log_error "未知选项: $1"
      show_help
      exit 1
      ;;
    *)
      if [[ -z "$TARGET_DIR" ]]; then
        TARGET_DIR="$1"
      else
        log_error "只能指定一个目标文件夹"
        exit 1
      fi
      shift
      ;;
    esac
  done
}

# 验证目标目录
validate_target_directory() {
  if [[ -z "$TARGET_DIR" ]]; then
    log_error "请提供目标文件夹路径"
    show_help
    exit 1
  fi

  if [[ ! -d "$TARGET_DIR" ]]; then
    log_error "目标文件夹 '$TARGET_DIR' 不存在"
    exit 1
  fi

  if [[ ! -r "$TARGET_DIR" ]]; then
    log_error "目标文件夹 '$TARGET_DIR' 无读取权限"
    exit 1
  fi
}

# 验证输出文件
validate_output_file() {
  if [[ -z "$OUTPUT_FILE" ]]; then
    OUTPUT_FILE="$TARGET_DIR/README.md"
  fi

  # 检查输出目录权限
  local output_dir
  output_dir=$(dirname "$OUTPUT_FILE")

  if [[ ! -d "$output_dir" ]]; then
    log_error "输出目录 '$output_dir' 不存在"
    exit 1
  fi

  if [[ ! -w "$output_dir" ]]; then
    log_error "输出目录 '$output_dir' 无写入权限"
    exit 1
  fi

  # 如果输出文件已存在，检查是否可写
  if [[ -f "$OUTPUT_FILE" && ! -w "$OUTPUT_FILE" ]]; then
    log_error "输出文件 '$OUTPUT_FILE' 无写入权限"
    exit 1
  fi
}

# 检查 Ollama 是否可用
check_ollama() {
  log_info "检查 Ollama 服务..."

  # 检查 ollama 命令是否存在
  if ! type ollama >/dev/null 2>&1; then
    log_error "未找到 ollama 命令，请确保已安装 Ollama"
    exit 1
  fi

  # 检查 Ollama 服务是否运行
  if ! ollama list >/dev/null 2>&1; then
    log_error "Ollama 服务未运行，请启动 Ollama 服务"
    exit 1
  fi

  # 检查模型是否存在（使用更精确的匹配）
  if ! ollama list | awk '{print $1}' | grep -Fxq "$OLLAMA_MODEL"; then
    log_warn "模型 '$OLLAMA_MODEL' 未找到，正在下载..."
    if ! ollama pull "$OLLAMA_MODEL"; then
      log_error "下载模型 '$OLLAMA_MODEL' 失败"
      exit 1
    fi
  fi

  log_success "Ollama 服务正常，模型 '$OLLAMA_MODEL' 可用"
}

# 获取文件类型统计
get_file_types() {
  local dir="$1"
  echo "文件类型统计:"

  # 使用更安全的方式处理文件名
  local temp_list
  temp_list=$(mktemp)

  find "$dir" -type f \( -name ".*" -prune \) -o -type f -print0 |
    while IFS= read -r -d '' file; do
      basename "$file"
    done >"$temp_list"

  if [[ -s "$temp_list" ]]; then
    sed 's/.*\.//' "$temp_list" |
      sort | uniq -c | sort -nr |
      head -20 |
      while read -r count ext; do
        if [[ -n "$ext" && "$ext" != "$(basename "$file")" ]]; then
          echo "  .$ext: $count 个文件"
        else
          echo "  无扩展名: $count 个文件"
        fi
      done
  else
    echo "  未找到文件"
  fi

  rm -f "$temp_list"
}

# 获取目录结构
get_directory_structure() {
  local dir="$1"
  echo "目录结构:"

  if command -v tree >/dev/null 2>&1; then
    if ! tree "$dir" -L 3 -I '.git|node_modules|__pycache__|.DS_Store|*.pyc|.env' --dirsfirst 2>/dev/null; then
      log_warn "tree 命令执行失败，使用备用方法"
      get_directory_structure_fallback "$dir"
    fi
  else
    get_directory_structure_fallback "$dir"
  fi
}

# 目录结构获取的备用方法
get_directory_structure_fallback() {
  local dir="$1"
  find "$dir" -type d -name ".git" -prune -o -type d -print 2>/dev/null |
    grep -v ".git" |
    sort |
    sed "s|^$dir||" |
    sed 's|^/||' |
    sed 's|/| |g' |
    awk '{
        indent = ""
        for(i=1; i<NF; i++) indent = indent "  "
        if(NF > 0) print indent $NF
        else print "."
    }'
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

  local found_files=0
  for file in "${important_files[@]}"; do
    if [[ -f "$dir/$file" ]]; then
      echo "  - $file"
      ((found_files++))
    fi
  done

  if [[ $found_files -eq 0 ]]; then
    echo "  - 未找到标准配置文件"
  fi

  # 查找其他可能的入口文件
  echo "  其他可能的入口文件:"
  local temp_files
  temp_files=$(mktemp)

  find "$dir" -maxdepth 2 -type f \( \
    -name "*.py" -o -name "*.js" -o -name "*.ts" -o \
    -name "*.go" -o -name "*.java" -o -name "*.cpp" -o -name "*.c" \
    \) 2>/dev/null | head -10 >"$temp_files"

  if [[ -s "$temp_files" ]]; then
    while IFS= read -r file; do
      echo "    - $(basename "$file")"
    done <"$temp_files"
  else
    echo "    - 未找到明显的入口文件"
  fi

  rm -f "$temp_files"
}

# 分析代码语言
analyze_languages() {
  local dir="$1"
  echo "主要编程语言:"

  local temp_files
  temp_files=$(mktemp)

  find "$dir" -type f -name ".*" -prune -o -type f -print 2>/dev/null |
    grep -E '\.(py|js|ts|java|cpp|c|go|php|rb|rs|swift|kt|scala|sh|ps1)$' >"$temp_files"

  if [[ -s "$temp_files" ]]; then
    sed 's/.*\.//' "$temp_files" |
      sort | uniq -c | sort -nr |
      head -5 |
      while read -r count ext; do
        local lang
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
  else
    echo "  - 未检测到编程语言文件"
  fi

  rm -f "$temp_files"
}

# 分析项目信息
analyze_project() {
  local dir="$1"
  local project_name
  project_name=$(basename "$dir")

  log_info "分析项目: $project_name"
  log_info "路径: $dir"

  # 创建分析报告（使用安全的临时文件）
  TEMP_ANALYSIS_FILE=$(mktemp /tmp/project_analysis_XXXXXX.txt)

  cat >"$TEMP_ANALYSIS_FILE" <<EOF
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

  echo "$TEMP_ANALYSIS_FILE"
}

# 生成英文 README 内容
generate_english_readme() {
  local analysis_file="$1"
  log_info "生成英文版 README..."

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
  if readme_content=$(ollama run "$OLLAMA_MODEL" "$prompt" 2>/dev/null); then
    if [[ -n "$readme_content" ]]; then
      echo "$readme_content"
    else
      log_error "生成的英文 README 内容为空"
      return 1
    fi
  else
    log_error "生成英文 README 失败"
    return 1
  fi
}

# 生成中文 README 内容
generate_chinese_readme() {
  local analysis_file="$1"
  log_info "生成中文版 README..."

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
  if readme_content=$(ollama run "$OLLAMA_MODEL" "$prompt" 2>/dev/null); then
    if [[ -n "$readme_content" ]]; then
      echo "$readme_content"
    else
      log_error "生成的中文 README 内容为空"
      return 1
    fi
  else
    log_error "生成中文 README 失败"
    return 1
  fi
}

# 检查是否已存在合适的 README 文件
check_existing_readme() {
  local dir="$1"
  local readme_files=("README.md" "readme.md" "README.txt" "readme.txt" "README.rst" "readme.rst" "Readme.md" "ReadMe.MD" "README.MD")

  log_info "检查现有 README 文件..."

  for readme_file in "${readme_files[@]}"; do
    local readme_path="$dir/$readme_file"
    if [[ -f "$readme_path" && -r "$readme_path" ]]; then
      log_warn "发现现有 README 文件: $readme_file"

      # 检查文件大小（字节数）
      local file_size
      file_size=$(wc -c <"$readme_path" 2>/dev/null || echo 0)

      # 检查行数
      local line_count
      line_count=$(wc -l <"$readme_path" 2>/dev/null || echo 0)

      # 检查非空行数（排除只有空白字符的行）
      local non_empty_lines
      non_empty_lines=$(grep -c '[^[:space:]]' "$readme_path" 2>/dev/null || echo 0)

      # 检查字符数（排除空白字符）
      local char_count
      char_count=$(tr -d '[:space:]' <"$readme_path" 2>/dev/null | wc -c || echo 0)

      echo -e "${BLUE}文件分析:${NC}"
      echo -e "  文件大小: $file_size 字节"
      echo -e "  总行数: $line_count 行"
      echo -e "  非空行数: $non_empty_lines 行"
      echo -e "  有效字符数: $char_count 个"

      # 判断是否为简单的 README
      if [[ $file_size -lt 200 || $non_empty_lines -lt 5 || $char_count -lt 100 ]]; then
        log_warn "README 内容过于简单，将重新生成"
        return 1 # 需要重新生成
      fi

      # 检查是否只有标题行
      local title_lines
      title_lines=$(grep -c '^#' "$readme_path" 2>/dev/null || echo 0)
      if [[ $title_lines -eq $non_empty_lines && $non_empty_lines -le 2 ]]; then
        log_warn "README 只包含标题，将重新生成"
        return 1 # 需要重新生成
      fi

      # 显示 README 前几行内容供用户参考
      echo -e "${BLUE}现有 README 内容预览:${NC}"
      echo -e "${GREEN}----------------------------------------${NC}"
      head -n 10 "$readme_path" | sed 's/^/  /'
      if [[ $line_count -gt 10 ]]; then
        echo -e "  ..."
        echo -e "  (还有 $((line_count - 10)) 行)"
      fi
      echo -e "${GREEN}----------------------------------------${NC}"

      log_success "发现完整的 README 文件，跳过生成"
      return 0 # 不需要重新生成
    fi
  done

  log_warn "未发现 README 文件，将生成新的 README"
  return 1 # 需要生成
}

# 生成双语 README 内容
generate_readme() {
  local analysis_file="$1"
  local project_name
  project_name=$(basename "$TARGET_DIR")

  log_info "使用 Ollama 生成双语 README..."

  local chinese_content english_content

  if [[ "$LANGUAGE" == "chinese" ]]; then
    # 中文优先模式：先生成中文，再生成英文
    if ! chinese_content=$(generate_chinese_readme "$analysis_file"); then
      log_error "生成中文版本失败"
      return 1
    fi

    if ! english_content=$(generate_english_readme "$analysis_file"); then
      log_error "生成英文版本失败"
      return 1
    fi

    # 创建双语 README，中文在前
    cat >"$OUTPUT_FILE" <<EOF
$chinese_content

---

## English Version

$english_content
EOF
  else
    # 英文优先模式：先生成英文，再生成中文
    if ! english_content=$(generate_english_readme "$analysis_file"); then
      log_error "生成英文版本失败"
      return 1
    fi

    if ! chinese_content=$(generate_chinese_readme "$analysis_file"); then
      log_error "生成中文版本失败"
      return 1
    fi

    # 创建双语 README，英文在前
    cat >"$OUTPUT_FILE" <<EOF
$english_content

---

## 中文版本

$chinese_content
EOF
  fi

  log_success "双语 README 文件已生成: $OUTPUT_FILE"
}

# 主函数
main() {
  echo -e "${GREEN}=== Auto-Generate-Readme Tool ===${NC}"
  echo ""

  # 解析命令行参数
  parse_arguments "$@"

  # 验证输入
  validate_target_directory
  validate_output_file

  # 如果不是强制模式，检查是否已存在合适的 README 文件
  if [[ "$FORCE" != "true" ]]; then
    if check_existing_readme "$TARGET_DIR"; then
      log_success "已存在合适的 README 文件，跳过生成"
      log_info "💡 提示: 使用 -f 或 --force 参数可强制重新生成"
      exit 0
    fi
  else
    log_warn "🔄 强制模式：将重新生成 README 文件"
  fi

  # 检查 Ollama（只有在需要生成时才检查）
  check_ollama

  # 分析项目
  local analysis_file
  if ! analysis_file=$(analyze_project "$TARGET_DIR"); then
    log_error "项目分析失败"
    exit 1
  fi

  # 生成 README
  if ! generate_readme "$analysis_file"; then
    log_error "README 生成失败"
    exit 1
  fi

  echo ""
  log_success "🎉 双语 README 生成完成！"
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
