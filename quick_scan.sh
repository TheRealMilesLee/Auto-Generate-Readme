#!/bin/bash

# 快速检测 README 杂质内容的脚本
# 用于快速扫描和预览哪些项目需要重新生成

set -euo pipefail

# 默认扫描目录
SCAN_DIR="${1:-/Users/silverhand/Developer/SourceRepo}"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# 统计变量
TOTAL=0
CONTAMINATED=0

echo -e "${BLUE}快速扫描 README 杂质内容${NC}"
echo -e "${BLUE}扫描目录: $SCAN_DIR${NC}"
echo

# 检测杂质内容的简化函数
check_contamination() {
  local readme_file="$1"
  local project_name="$2"

  # 常见的杂质模式
  local patterns=(
    "Thinking\.\.\."
    "\.\.\.done thinking\."
    "Let me analyze"
    "I'll analyze"
    "Here's.*README"
    '\[90m|\[0m|\[1m|\[33m|\[32m|\[31m'
    '^```$'
    "Based on.*analysis"
  )

  for pattern in "${patterns[@]}"; do
    if grep -qE "$pattern" "$readme_file" 2>/dev/null; then
      return 0
    fi
  done

  return 1
}

# 扫描所有项目
while IFS= read -r -d '' readme_file; do
  project_dir="$(dirname "$readme_file")"
  project_name="$(basename "$project_dir")"

  # 跳过当前目录
  if [[ "$(basename "$(pwd)")" == "Auto-Generate-Readme" && "$project_dir" == *"Auto-Generate-Readme"* ]]; then
    continue
  fi

  ((TOTAL++))

  if check_contamination "$readme_file" "$project_name"; then
    ((CONTAMINATED++))
    echo -e "${RED}❌ $project_name${NC} - ${YELLOW}$project_dir${NC}"
  else
    echo -e "${GREEN}✅ $project_name${NC}"
  fi

done < <(find "$SCAN_DIR" -name "README.md" -type f -print0 2>/dev/null)

echo
echo -e "${BLUE}========== 扫描结果 ==========${NC}"
echo -e "${BLUE}总项目数: $TOTAL${NC}"
echo -e "${RED}包含杂质: $CONTAMINATED${NC}"
echo -e "${GREEN}干净项目: $((TOTAL - CONTAMINATED))${NC}"

if [[ $CONTAMINATED -gt 0 ]]; then
  echo
  echo -e "${YELLOW}运行以下命令进行批量重新生成:${NC}"
  echo -e "${BLUE}./batch_regenerate.sh --regenerate${NC}"
fi
