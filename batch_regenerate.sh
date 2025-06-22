#!/bin/bash

# 批量检测和重新生成 README 文件脚本
# 用于处理所有包含 Ollama 输出杂质的 README 文件

set -euo pipefail

# 默认扫描目录
SCAN_DIR="/mnt/e/SourceRepo/"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
RUN_SCRIPT="${SCRIPT_DIR}/run.sh"

# 日志文件
LOG_FILE="${SCRIPT_DIR}/batch_regenerate.log"
REPORT_FILE="${SCRIPT_DIR}/batch_report.txt"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 统计变量
TOTAL_PROJECTS=0
CONTAMINATED_PROJECTS=0
REGENERATED_PROJECTS=0
FAILED_PROJECTS=0

# 检测 README 是否包含杂质内容的函数
is_readme_contaminated() {
  local readme_file="$1"

  if [[ ! -f "$readme_file" ]]; then
    return 1
  fi

  # 检测各种杂质内容的模式
  local contamination_patterns=(
    "Thinking\.\.\."
    "\.\.\.done thinking\."
    "Let me analyze"
    "I'll analyze"
    "Looking at"
    "Based on the"
    "Here's.*README"
    "I'll create"
    "Let me create"
    '\[90m'
    '\[0m'
    '\[1m'
    '\[33m'
    '\[32m'
    '\[31m'
    '\[34m'
    '\[35m'
    '\[36m'
    '\[37m'
    '.*\x1b\[[0-9;]*m'
    '^```$'
    '^Here'\''s the.*:'
    '^I'\''ll.*:'
    '^Let me.*:'
    '^Based on.*:'
    '^This project.*appears to be'
    '^Looking at.*:'
    "analyze.*project"
    "examination.*code"
  )

  for pattern in "${contamination_patterns[@]}"; do
    if grep -qE "$pattern" "$readme_file" 2>/dev/null; then
      echo "Found contamination pattern: $pattern" >&2
      return 0
    fi
  done

  return 1
}

# 记录日志
log() {
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"
}

# 输出带颜色的消息
print_status() {
  local color=$1
  local message=$2
  echo -e "${color}${message}${NC}"
}

# 检查必要的工具和文件
check_prerequisites() {
  if [[ ! -f "$RUN_SCRIPT" ]]; then
    print_status "$RED" "错误: 找不到 run.sh 脚本: $RUN_SCRIPT"
    exit 1
  fi

  if [[ ! -x "$RUN_SCRIPT" ]]; then
    print_status "$YELLOW" "设置 run.sh 脚本为可执行..."
    chmod +x "$RUN_SCRIPT"
  fi

  if [[ ! -d "$SCAN_DIR" ]]; then
    print_status "$RED" "错误: 扫描目录不存在: $SCAN_DIR"
    exit 1
  fi

  # 清空日志和报告文件
  >"$LOG_FILE"
  >"$REPORT_FILE"
}

# 扫描项目并生成报告
scan_projects() {
  local scan_dir="$1"

  print_status "$BLUE" "开始扫描目录: $scan_dir"
  log "开始扫描目录: $scan_dir"

  # 查找所有包含 README.md 的项目目录
  while IFS= read -r -d '' readme_file; do
    local project_dir
    project_dir="$(dirname "$readme_file")"
    local project_name
    project_name="$(basename "$project_dir")"

    # 跳过当前脚本所在的目录
    if [[ "$project_dir" == "$SCRIPT_DIR" ]]; then
      continue
    fi

    ((TOTAL_PROJECTS++))

    print_status "$BLUE" "检查项目: $project_name ($project_dir)"

    if is_readme_contaminated "$readme_file"; then
      ((CONTAMINATED_PROJECTS++))
      print_status "$YELLOW" "发现污染的 README: $project_name"
      log "发现污染的 README: $project_dir"
      echo "CONTAMINATED: $project_dir" >>"$REPORT_FILE"
    else
      print_status "$GREEN" "README 干净: $project_name"
      echo "CLEAN: $project_dir" >>"$REPORT_FILE"
    fi

  done < <(find "$scan_dir" -name "README.md" -type f -print0 2>/dev/null)
}

# 重新生成污染的 README 文件
regenerate_contaminated_readmes() {
  print_status "$BLUE" "开始重新生成污染的 README 文件..."
  log "开始重新生成污染的 README 文件"

  while IFS= read -r line; do
    if [[ "$line" =~ ^CONTAMINATED:\ (.+)$ ]]; then
      local project_dir="${BASH_REMATCH[1]}"
      local project_name
      project_name="$(basename "$project_dir")"

      print_status "$YELLOW" "重新生成 README: $project_name"
      log "重新生成 README: $project_dir"

      # 备份原始 README
      local backup_file="${project_dir}/README.md.backup.$(date +%Y%m%d_%H%M%S)"
      if cp "${project_dir}/README.md" "$backup_file" 2>/dev/null; then
        log "已备份原始 README: $backup_file"
      fi

      # 运行 run.sh 重新生成 README
      if (cd "$project_dir" && "$RUN_SCRIPT" "$project_dir" >/dev/null 2>&1); then
        ((REGENERATED_PROJECTS++))
        print_status "$GREEN" "成功重新生成: $project_name"
        log "成功重新生成: $project_dir"

        # 验证新生成的 README 是否干净
        if is_readme_contaminated "${project_dir}/README.md"; then
          print_status "$RED" "警告: 重新生成的 README 仍然包含杂质: $project_name"
          log "警告: 重新生成的 README 仍然包含杂质: $project_dir"
        else
          print_status "$GREEN" "验证通过: 新 README 干净: $project_name"
          log "验证通过: 新 README 干净: $project_dir"
        fi
      else
        ((FAILED_PROJECTS++))
        print_status "$RED" "重新生成失败: $project_name"
        log "重新生成失败: $project_dir"

        # 恢复备份
        if [[ -f "$backup_file" ]]; then
          mv "$backup_file" "${project_dir}/README.md"
          log "已恢复备份: $project_dir"
        fi
      fi
    fi
  done <"$REPORT_FILE"
}

# 显示统计信息
show_summary() {
  print_status "$BLUE" "================= 批量处理总结 ================="
  print_status "$BLUE" "扫描的项目总数: $TOTAL_PROJECTS"
  print_status "$YELLOW" "发现污染的项目: $CONTAMINATED_PROJECTS"
  print_status "$GREEN" "成功重新生成: $REGENERATED_PROJECTS"
  print_status "$RED" "重新生成失败: $FAILED_PROJECTS"
  print_status "$BLUE" "=============================================="

  log "批量处理完成 - 总数: $TOTAL_PROJECTS, 污染: $CONTAMINATED_PROJECTS, 成功: $REGENERATED_PROJECTS, 失败: $FAILED_PROJECTS"

  if [[ $FAILED_PROJECTS -gt 0 ]]; then
    print_status "$RED" "存在失败的项目，请查看日志文件: $LOG_FILE"
  fi

  print_status "$BLUE" "详细报告保存在: $REPORT_FILE"
  print_status "$BLUE" "日志文件保存在: $LOG_FILE"
}

# 显示使用帮助
show_help() {
  cat <<EOF
批量检测和重新生成 README 文件脚本

用法: $0 [选项] [扫描目录]

选项:
  --scan-only     只扫描和报告，不进行重新生成
  --regenerate    执行完整的扫描和重新生成流程
  --help          显示此帮助信息

参数:
  扫描目录        要扫描的根目录路径 (默认: /Users/silverhand/Developer/SourceRepo)

示例:
  $0 --scan-only                    # 只扫描当前默认目录
  $0 --regenerate ~/projects        # 扫描并重新生成 ~/projects 目录
  $0 /path/to/repos                 # 扫描并重新生成指定目录

EOF
}

# 主函数
main() {
  local scan_only=false
  local regenerate=false
  local target_dir=""

  # 解析命令行参数
  while [[ $# -gt 0 ]]; do
    case $1 in
    --scan-only)
      scan_only=true
      shift
      ;;
    --regenerate)
      regenerate=true
      shift
      ;;
    --help)
      show_help
      exit 0
      ;;
    -*)
      print_status "$RED" "未知选项: $1"
      show_help
      exit 1
      ;;
    *)
      target_dir="$1"
      shift
      ;;
    esac
  done

  # 如果指定了目录，使用它；否则使用默认目录
  if [[ -n "$target_dir" ]]; then
    SCAN_DIR="$target_dir"
  fi

  # 如果没有指定模式，默认为完整的重新生成流程
  if [[ "$scan_only" == false && "$regenerate" == false ]]; then
    regenerate=true
  fi

  print_status "$BLUE" "批量 README 检测和重新生成工具"
  print_status "$BLUE" "扫描目录: $SCAN_DIR"
  print_status "$BLUE" "运行模式: $([ "$scan_only" == true ] && echo "仅扫描" || echo "扫描并重新生成")"

  check_prerequisites

  # 扫描项目
  scan_projects "$SCAN_DIR"

  # 如果只是扫描模式，显示结果并退出
  if [[ "$scan_only" == true ]]; then
    show_summary
    return 0
  fi

  # 询问用户是否继续重新生成
  if [[ $CONTAMINATED_PROJECTS -gt 0 ]]; then
    print_status "$YELLOW" "发现 $CONTAMINATED_PROJECTS 个包含杂质的项目。"
    read -p "是否继续重新生成这些项目的 README？ (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
      regenerate_contaminated_readmes
    else
      print_status "$YELLOW" "已取消重新生成操作。"
    fi
  else
    print_status "$GREEN" "没有发现包含杂质的 README 文件！"
  fi

  show_summary
}

# 如果脚本被直接执行
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  main "$@"
fi
