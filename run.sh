#!/bin/bash

# Auto-Generate-Readme Script
# 分析目标文件夹并使用 Ollama 生成 README 文件

set -e
set -o pipefail

# 设置 UTF-8 编码环境
export LC_ALL=C.UTF-8
export LANG=C.UTF-8

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
  # 只在脚本完全结束时清理临时文件
  if [[ "${CLEANUP_ENABLED:-true}" == "true" ]]; then
    # 清理分析报告临时文件
    if [[ -n "${TEMP_ANALYSIS_FILE:-}" && -f "$TEMP_ANALYSIS_FILE" ]]; then
      rm -f "$TEMP_ANALYSIS_FILE"
    fi

    # 清理其他可能的临时文件
    if [[ -n "${TEMP_FILES:-}" ]]; then
      for temp_file in "${TEMP_FILES[@]}"; do
        [[ -f "$temp_file" ]] && rm -f "$temp_file"
      done
    fi

    # 清理写入过程中的临时文件
    if [[ -n "${OUTPUT_FILE:-}" ]]; then
      rm -f "${OUTPUT_FILE}".tmp.*
    fi

    # 清理所有项目分析临时文件
    rm -f /tmp/project_analysis_* 2>/dev/null || true
    rm -f /tmp/file_list_* 2>/dev/null || true
    rm -f /tmp/entry_files_* 2>/dev/null || true
    rm -f /tmp/lang_files_* 2>/dev/null || true
  fi
}

# 设置错误处理
trap 'handle_error $LINENO' ERR
trap 'cleanup' EXIT

# 创建安全的临时文件
create_temp_file() {
  local prefix="${1:-temp}"
  local suffix="${2:-}"

  # 在 WSL2 Ubuntu 环境下，优先使用 /tmp 目录
  local temp_dir="/tmp"

  # 创建临时文件
  if [[ -n "$suffix" ]]; then
    mktemp "${temp_dir}/${prefix}_XXXXXX${suffix}"
  else
    mktemp "${temp_dir}/${prefix}_XXXXXX"
  fi
}

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
  temp_list=$(create_temp_file "file_list")

  # 确保临时文件创建成功
  if [[ ! -f "$temp_list" ]]; then
    echo "  - 无法创建临时文件进行统计"
    return 1
  fi

  # 使用更安全的 find 命令，避免特殊字符问题
  if find "$dir" -type f \( -name ".*" -prune \) -o -type f -print0 2>/dev/null |
    while IFS= read -r -d '' file; do
      if [[ -f "$file" ]]; then
        basename "$file" 2>/dev/null || echo "unknown"
      fi
    done >"$temp_list"; then

    if [[ -s "$temp_list" ]]; then
      # 使用更安全的处理方式
      if sed 's/.*\.//' "$temp_list" 2>/dev/null |
        sort | uniq -c | sort -nr |
        head -20 |
        while read -r count ext; do
          if [[ -n "$ext" && "$ext" != "unknown" ]]; then
            echo "  .$ext: $count 个文件"
          else
            echo "  无扩展名: $count 个文件"
          fi
        done; then
        true # 成功处理
      else
        echo "  - 处理文件扩展名时出错"
      fi
    else
      echo "  - 未找到文件"
    fi
  else
    echo "  - 查找文件时出错"
  fi

  # 安全删除临时文件
  rm -f "$temp_list" 2>/dev/null || true
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
    "Podfile" "Podfile.lock" "Package.swift" "Cartfile" "Cartfile.resolved"
    "Info.plist" "AppDelegate.swift" "SceneDelegate.swift"
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

  # 检查 Xcode 项目文件
  local xcode_projects=0
  for xcodeproj in "$dir"/*.xcodeproj; do
    if [[ -d "$xcodeproj" && "$xcodeproj" != "$dir/*.xcodeproj" ]]; then
      echo "  - $(basename "$xcodeproj")"
      ((found_files++))
      ((xcode_projects++))
    fi
  done

  for xcworkspace in "$dir"/*.xcworkspace; do
    if [[ -d "$xcworkspace" && "$xcworkspace" != "$dir/*.xcworkspace" ]]; then
      echo "  - $(basename "$xcworkspace")"
      ((found_files++))
      ((xcode_projects++))
    fi
  done

  if [[ $xcode_projects -gt 0 ]]; then
    echo "  iOS/macOS 项目类型: Xcode 项目"
  fi

  if [[ $found_files -eq 0 ]]; then
    echo "  - 未找到标准配置文件"
  fi

  # 查找其他可能的入口文件
  echo "  其他可能的入口文件:"
  local temp_files
  temp_files=$(create_temp_file "entry_files")

  if [[ -f "$temp_files" ]]; then
    if find "$dir" -maxdepth 2 -type f \( \
      -name "*.py" -o -name "*.js" -o -name "*.ts" -o \
      -name "*.go" -o -name "*.java" -o -name "*.cpp" -o -name "*.c" -o \
      -name "*.swift" -o -name "*.m" -o -name "*.mm" \
      \) 2>/dev/null | head -10 >"$temp_files"; then

      if [[ -s "$temp_files" ]]; then
        while IFS= read -r file; do
          if [[ -n "$file" ]]; then
            echo "    - $(basename "$file")"
          fi
        done <"$temp_files"
      else
        echo "    - 未找到明显的入口文件"
      fi
    else
      echo "    - 查找入口文件时出错"
    fi

    # 安全删除临时文件
    rm -f "$temp_files" 2>/dev/null || true
  else
    echo "    - 无法创建临时文件"
  fi
}

# 分析代码语言
analyze_languages() {
  local dir="$1"
  echo "主要编程语言:"

  local temp_files
  temp_files=$(create_temp_file "lang_files")

  # 确保临时文件创建成功
  if [[ ! -f "$temp_files" ]]; then
    echo "  - 无法创建临时文件进行语言分析"
    return 1
  fi

  if find "$dir" -type f -name ".*" -prune -o -type f -print 2>/dev/null |
    grep -E '\.(py|js|ts|java|cpp|c|go|php|rb|rs|swift|kt|scala|sh|ps1|m|mm|h)$' >"$temp_files"; then

    if [[ -s "$temp_files" ]]; then
      if sed 's/.*\.//' "$temp_files" 2>/dev/null |
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
          m) lang="Objective-C" ;;
          mm) lang="Objective-C++" ;;
          h) lang="C/C++/Objective-C Header" ;;
          *) lang="$ext" ;;
          esac
          echo "  - $lang: $count 个文件"
        done; then
        true # 成功处理
      else
        echo "  - 处理语言统计时出错"
      fi
    else
      echo "  - 未检测到编程语言文件"
    fi
  else
    echo "  - 未检测到编程语言文件"
  fi

  # 安全删除临时文件
  rm -f "$temp_files" 2>/dev/null || true
}

# 分析项目信息
analyze_project() {
  local dir="$1"
  local project_name
  project_name=$(basename "$dir")

  # 将日志输出重定向到 stderr，确保只有文件路径输出到 stdout
  {
    log_info "分析项目: $project_name"
    log_info "路径: $dir"
  } >&2

  # 临时禁用自动清理，防止文件被过早删除
  CLEANUP_ENABLED=false

  # 创建分析报告（使用安全的临时文件）
  local temp_analysis_file
  temp_analysis_file=$(create_temp_file "project_analysis" ".txt")

  # 调试信息：显示临时文件路径
  {
    log_info "临时分析文件: $temp_analysis_file"
  } >&2

  # 确保临时文件创建成功
  if [[ ! -f "$temp_analysis_file" ]]; then
    {
      log_error "无法创建临时分析文件: $temp_analysis_file"
    } >&2
    return 1
  fi

  # 分别获取各部分内容，使用更安全的方式
  local directory_structure file_types important_files languages

  {
    log_info "开始收集项目信息..."

    # 重定向 stderr 以避免日志输出干扰，并处理可能的错误
    log_info "获取目录结构..."
  } >&2
  directory_structure=$(get_directory_structure "$dir" 2>/dev/null || echo "  - 无法获取目录结构")

  {
    log_info "获取文件类型统计..."
  } >&2
  file_types=$(get_file_types "$dir" 2>/dev/null || echo "  - 无法获取文件类型")

  {
    log_info "获取重要文件..."
  } >&2
  important_files=$(get_important_files "$dir" 2>/dev/null || echo "  - 无法获取重要文件")

  {
    log_info "分析编程语言..."
  } >&2
  languages=$(analyze_languages "$dir" 2>/dev/null || echo "  - 无法分析编程语言")

  {
    log_info "写入分析报告..."
  } >&2

  # 使用更安全的写入方式
  if ! {
    printf "项目分析报告\n"
    printf "=============\n\n"
    printf "项目名称: %s\n" "$project_name"
    printf "项目路径: %s\n" "$dir"
    printf "分析时间: %s\n\n" "$(date '+%Y-%m-%d %H:%M:%S')"
    printf "%s\n\n" "$directory_structure"
    printf "%s\n\n" "$file_types"
    printf "%s\n\n" "$important_files"
    printf "%s\n\n" "$languages"
  } >"$temp_analysis_file" 2>/dev/null; then
    {
      log_error "写入分析报告失败"
    } >&2
    rm -f "$temp_analysis_file" 2>/dev/null || true
    return 1
  fi

  # 验证文件是否成功创建和写入
  if [[ ! -f "$temp_analysis_file" ]]; then
    {
      log_error "分析报告文件创建失败: $temp_analysis_file"
    } >&2
    return 1
  fi

  if [[ ! -s "$temp_analysis_file" ]]; then
    {
      log_error "分析报告文件为空: $temp_analysis_file"
      # 显示文件权限和状态信息用于调试
      if ls -la "$temp_analysis_file" 2>/dev/null; then
        log_info "文件权限信息已显示"
      else
        log_error "无法获取文件权限信息"
      fi
    } >&2
    rm -f "$temp_analysis_file" 2>/dev/null || true
    return 1
  fi

  local file_size
  file_size=$(wc -c <"$temp_analysis_file" 2>/dev/null || echo 0)
  {
    log_info "分析报告创建成功，大小: $file_size 字节"
  } >&2

  # 将临时文件路径保存到全局变量，确保在主函数中可以访问
  TEMP_ANALYSIS_FILE="$temp_analysis_file"

  # 重新启用清理（但不会立即清理当前正在使用的文件）
  CLEANUP_ENABLED=true

  # 返回文件路径（仅文件路径，不包含日志输出）
  printf "%s" "$temp_analysis_file"
}

# 生成英文 README 内容
generate_english_readme() {
  local analysis_file="$1"

  # 将日志输出重定向到 stderr
  {
    log_info "生成英文版 README..."
  } >&2

  local prompt="You are a professional software documentation writer. Based on the following project analysis, please generate a comprehensive and well-structured README.md file.

CRITICAL REQUIREMENTS:
- Output ONLY raw Markdown content, no explanations, no thinking process, no additional text
- Start directly with the markdown content (e.g., # Project Title)
- Do not include any meta-commentary, explanations, or thinking process about the README
- Do not wrap the content in code blocks or any other formatting
- Generate ONLY the actual README.md file content that can be directly saved
- Do not include any ANSI color codes or control characters
- Do not include any thinking process like 'Thinking...' or '...done thinking.'

The README should include:
1. Project title and brief description
2. Features and functionality
3. Installation instructions (including Xcode setup for iOS/macOS projects)
4. Usage examples
5. Project structure explanation
6. Dependencies and requirements (including CocoaPods, SPM, Carthage for iOS/macOS)
7. Contributing guidelines
8. License information

For Xcode projects, please include:
- iOS/macOS deployment targets
- Xcode version requirements
- Swift version compatibility
- CocoaPods/Swift Package Manager setup instructions
- Build and run instructions

Use proper Markdown formatting. Make the README informative, professional, and easy to understand.

Project Analysis:
$(cat "$analysis_file")

Generate ONLY the complete README.md content (raw Markdown only, no explanations):"

  local readme_content
  # 确保 Ollama 输出使用 UTF-8 编码，并过滤掉所有可能的杂质
  if readme_content=$(
    LC_ALL=C.UTF-8 TERM=dumb ollama run "$OLLAMA_MODEL" "$prompt" 2>/dev/null |
      # 移除 ANSI 颜色代码和控制字符
      sed 's/\x1b\[[0-9;]*m//g' |
      # 移除思考过程相关的行
      grep -v 'Thinking\.\.\.' |
      grep -v '\.\.\.done thinking\.' |
      grep -v '^思考中\.\.\.' |
      grep -v '^\.\.\.思考完成\.' |
      # 移除常见的元评论开头
      grep -v '^Here\|^I\|^The following\|^Based on\|^This README' |
      grep -v '^Let me\|^I will\|^I have\|^Below is\|^Here is' |
      # 确保没有日志信息混入
      grep -v '\[INFO\]\|\[ERROR\]\|\[WARN\]\|\[SUCCESS\]' |
      # 移除包装的代码块，保留内容
      sed '/^```markdown$/,/^```$/{/^```markdown$/d; /^```$/d;}' |
      # 移除末尾孤立的 ``` 行，保留代码块中的
      awk 'BEGIN{buffer=""} {if($0=="```" && buffer!="" && buffer!~/```/) next; buffer=$0; print}' |
      # 移除可能的控制字符
      tr -d '\r'
  ); then

    # 最终清理：移除开头和结尾的空行
    readme_content=$(echo "$readme_content" | sed '/^$/d' | awk 'BEGIN{RS=""; ORS="\n\n"} {print}' | sed 's/\n\n$//')

    if [[ -n "$readme_content" && "${#readme_content}" -gt 50 ]]; then
      echo "$readme_content"
    else
      {
        log_error "生成的英文 README 内容为空或过短"
      } >&2
      return 1
    fi
  else
    {
      log_error "生成英文 README 失败"
    } >&2
    return 1
  fi
}

# 生成中文 README 内容
generate_chinese_readme() {
  local analysis_file="$1"

  # 将日志输出重定向到 stderr
  {
    log_info "生成中文版 README..."
  } >&2

  local prompt="直接生成README.md文件内容，不要任何解释文字。

必须严格遵守：
- 立即开始输出README内容，第一行就是项目标题（# 项目名称）
- 绝对不要输出"好的"、"我现在需要"、"让我来"等任何解释性文字
- 绝对不要输出思考过程或元评论
- 只输出可以直接保存为README.md的纯Markdown内容
- 不要包装在代码块中

README包含这些部分：
1. 项目标题和描述
2. 功能特性
3. 安装说明
4. 使用示例
5. 项目结构
6. 依赖要求
7. 贡献指南
8. 许可证信息

项目分析：
$(cat "$analysis_file")

现在直接开始输出README内容："

  local readme_content
  # 确保 Ollama 输出使用 UTF-8 编码，并过滤掉所有可能的杂质
  if readme_content=$(
    LC_ALL=C.UTF-8 TERM=dumb ollama run "$OLLAMA_MODEL" "$prompt" 2>/dev/null |
      # 移除 ANSI 颜色代码和控制字符
      sed 's/\x1b\[[0-9;]*m//g' |
      # 移除思考过程相关的行
      grep -v 'Thinking\.\.\.' |
      grep -v '\.\.\.done thinking\.' |
      grep -v '^思考中\.\.\.' |
      grep -v '^\.\.\.思考完成\.' |
      # 移除常见的中文解释性开头
      grep -v '^好的' |
      grep -v '^我现在需要' |
      grep -v '^让我来' |
      grep -v '^我将' |
      grep -v '^我会' |
      grep -v '^我需要' |
      grep -v '^根据您的要求' |
      grep -v '^根据项目分析' |
      grep -v '^基于以上分析' |
      grep -v '^现在我来' |
      grep -v '^首先' |
      grep -v '^接下来' |
      grep -v '^以下是' |
      grep -v '^下面是' |
      grep -v '^这里是' |
      grep -v '^这个README' |
      grep -v '处理用户的请求' |
      grep -v '生成一个符合要求' |
      grep -v '巴拉巴拉' |
      # 移除常见的元评论开头
      grep -v '^根据\|^我将\|^以下是\|^这个README' |
      grep -v '^让我\|^我会\|^我已经\|^下面是\|^这里是' |
      # 确保没有日志信息混入
      grep -v '\[INFO\]\|\[ERROR\]\|\[WARN\]\|\[SUCCESS\]' |
      # 移除包装的代码块，保留内容
      sed '/^```markdown$/,/^```$/{/^```markdown$/d; /^```$/d;}' |
      # 移除末尾孤立的 ``` 行
      sed '${/^```$/d;}' |
      # 移除包含"处理"、"需要"、"生成"等词的解释性句子开头
      grep -v '^.*处理.*请求' |
      grep -v '^.*需要.*生成' |
      grep -v '^.*符合要求' |
      # 只保留以 # 开头或者明显是 Markdown 内容的行，跳过第一个非 Markdown 行
      awk 'BEGIN{found_md=0} /^#/ {found_md=1; print; next} found_md==1 {print} found_md==0 && /^[^#]/ && !/^$/ {next} {print}' |
      # 移除可能的控制字符
      tr -d '\r'
  ); then

    # 最终清理：移除开头和结尾的空行
    readme_content=$(echo "$readme_content" | sed '/^$/d' | awk 'BEGIN{RS=""; ORS="\n\n"} {print}' | sed 's/\n\n$//')

    if [[ -n "$readme_content" && "${#readme_content}" -gt 50 ]]; then
      echo "$readme_content"
    else
      {
        log_error "生成的中文 README 内容为空或过短"
      } >&2
      return 1
    fi
  else
    {
      log_error "生成中文 README 失败"
    } >&2
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

# 安全写入文件（确保 UTF-8 编码）
write_readme_file() {
  local content="$1"
  local output_file="$2"

  # 创建临时文件以确保原子写入
  local temp_file
  temp_file=$(create_temp_file "$(basename "$output_file")" ".tmp")

  # 使用 printf 而不是 echo 来避免换行问题
  # 设置 LC_ALL=C.UTF-8 确保 UTF-8 编码
  if ! LC_ALL=C.UTF-8 printf '%s\n' "$content" >"$temp_file"; then
    log_error "写入临时文件失败: $temp_file"
    rm -f "$temp_file"
    return 1
  fi

  # 验证临时文件是否成功写入且非空
  if [[ ! -f "$temp_file" || ! -s "$temp_file" ]]; then
    log_error "临时文件为空或不存在: $temp_file"
    rm -f "$temp_file"
    return 1
  fi

  # 原子移动到目标文件
  if ! mv "$temp_file" "$output_file"; then
    log_error "移动文件失败: $temp_file -> $output_file"
    rm -f "$temp_file"
    return 1
  fi

  # 验证最终文件
  if [[ ! -f "$output_file" || ! -s "$output_file" ]]; then
    log_error "最终文件验证失败: $output_file"
    return 1
  fi

  # 检查文件编码（如果 file 命令可用）
  if command -v file >/dev/null 2>&1; then
    local file_type
    file_type=$(file "$output_file")
    if [[ "$file_type" == *"UTF-8"* ]]; then
      log_info "文件编码验证成功: UTF-8"
    else
      log_warn "文件编码可能不是 UTF-8: $file_type"
    fi
  fi

  log_info "文件已成功写入: $output_file ($(wc -c <"$output_file") 字节)"
}

# 生成双语 README 内容
generate_readme() {
  local analysis_file="$1"
  local project_name
  project_name=$(basename "$TARGET_DIR")

  log_info "使用 Ollama 生成双语 README..."

  local chinese_content english_content final_content

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
    final_content="${chinese_content}

---

## English Version

${english_content}"
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
    final_content="${english_content}

---

## 中文版本

${chinese_content}"
  fi

  # 使用安全的 UTF-8 写入方法
  if ! write_readme_file "$final_content" "$OUTPUT_FILE"; then
    log_error "写入 README 文件失败"
    return 1
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
  log_info "开始项目分析..."

  # 临时禁用 set -e 以防止子函数中的正常错误处理被中断
  set +e
  analysis_file=$(analyze_project "$TARGET_DIR")
  local analyze_result=$?
  set -e

  if [[ $analyze_result -ne 0 || -z "$analysis_file" ]]; then
    log_error "项目分析失败，返回码: $analyze_result"
    exit 1
  fi

  log_info "项目分析完成，分析文件: $analysis_file"

  # 验证分析文件的存在性和完整性
  if [[ ! -f "$analysis_file" ]]; then
    log_error "项目分析失败：分析文件不存在: $analysis_file"
    # 调试信息：检查 /tmp 目录
    log_info "检查 /tmp 目录中的临时文件："
    ls -la /tmp/project_analysis_* 2>/dev/null || log_info "未找到项目分析临时文件"
    exit 1
  fi

  # 验证文件可读性和内容
  if [[ ! -r "$analysis_file" ]]; then
    log_error "分析文件无法读取: $analysis_file"
    exit 1
  fi

  if [[ ! -s "$analysis_file" ]]; then
    log_error "分析文件为空: $analysis_file"
    exit 1
  fi

  local file_size
  file_size=$(wc -c <"$analysis_file" 2>/dev/null || echo 0)
  log_info "分析文件验证通过，大小: $file_size 字节"

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
