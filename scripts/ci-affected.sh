#!/usr/bin/env bash
# ==============================================================================
# Ferrite 动态 CI 测试调度脚本 (Pure Bash，零临时产物，零环境依赖)
#
# 依据 git diff 分析变更范围，动态调度受影响模块的编译检查与测试：
# 1. 改变 crates/web/* 或前端 apps -> 运行 wasm32 编译及前端测试
# 2. 改变 crates/api/*、gateway、harness 或 apps/api -> 运行 native 检查与单测
# 3. 纯文档/配置改变 -> 秒级放行跳过
# 4. 共享契约 (crates/contract) 或依赖配置变动 -> 全量运行
#
# 用法:
#   ./scripts/ci-affected.sh [--base <ref>] [--dry-run]
# ==============================================================================

set -euo pipefail

BASE_REF=""
DRY_RUN=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base)
      BASE_REF="$2"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

# ------------------------------------------------------------------------------
# 1. 确定变更文件列表
# ------------------------------------------------------------------------------
DIFF_TARGET=""
CHANGED_FILES=""

if [[ -n "$BASE_REF" ]]; then
  if git rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
    DIFF_TARGET="$BASE_REF...HEAD"
    CHANGED_FILES=$(git diff --name-only "$BASE_REF...HEAD" 2>/dev/null || git diff --name-only "$BASE_REF" 2>/dev/null || true)
  fi
fi

if [[ -z "$CHANGED_FILES" ]]; then
  for candidate in "origin/main" "remotes/newxapi/main" "newxapi/main" "main" "HEAD~1"; do
    if git rev-parse --verify "$candidate" >/dev/null 2>&1; then
      DIFF_TARGET="$candidate...HEAD"
      CHANGED_FILES=$(git diff --name-only "$candidate...HEAD" 2>/dev/null || true)
      if [[ -n "$CHANGED_FILES" ]]; then
        break
      fi
    fi
  done
fi

# 补充工作区未提交与未跟踪的文件
UNCOMMITTED=$(git diff --name-only HEAD 2>/dev/null || true)
UNTRACKED=$(git status --porcelain 2>/dev/null | awk '/^\?\?/ {print $2}' || true)

ALL_CHANGED=$(printf "%s\n%s\n%s\n" "$CHANGED_FILES" "$UNCOMMITTED" "$UNTRACKED" | sed '/^$/d' | sort -u || true)
CHANGED_COUNT=$(echo "$ALL_CHANGED" | grep -c . || true)

echo "================================================================="
echo "  FERRITE DYNAMIC CI - AFFECTED CRATES RUNNER (Shell)"
echo "================================================================="
echo "Diff target : ${DIFF_TARGET:-working-tree} (${CHANGED_COUNT} changed files)"

if [[ "$CHANGED_COUNT" -eq 0 ]]; then
  echo "No changed files detected. Exiting cleanly."
  exit 0
fi

# ------------------------------------------------------------------------------
# 2. 全量触发规则检查
# ------------------------------------------------------------------------------
IS_GLOBAL=false
GLOBAL_TRIGGER=""

for f in $ALL_CHANGED; do
  case "$f" in
    Cargo.lock|Cargo.toml|rust-toolchain.toml|.github/*|scripts/*|crates/contract/*)
      IS_GLOBAL=true
      GLOBAL_TRIGGER="$f"
      break
      ;;
  esac
done

# ------------------------------------------------------------------------------
# 3. 获取所有 Package 的映射目录与 Package Name
# ------------------------------------------------------------------------------
# 输出格式: dir_path|pkg_name|has_tests|is_web
get_workspace_meta() {
  cargo metadata --format-version 1 --no-deps | jq -r '
    .workspace_root as $root |
    .packages[] |
    ( .manifest_path | sub("/Cargo.toml$"; "") | sub("^" + $root + "/"; "") ) as $dir |
    .name as $name |
    ($dir | startswith("crates/web") or . == "apps/admin-web" or . == "apps/tavern-web") as $is_web |
    "\($dir)|\($name)|\($is_web)"
  '
}

META_LIST=$(get_workspace_meta)

NATIVE_CHECK_PKGS=()
WASM_CHECK_PKGS=()
TEST_PKGS=()

has_test_suite() {
  local dir="$1"
  if [[ -d "$dir/tests" ]] && compgen -G "$dir/tests/*.rs" >/dev/null; then
    return 0
  fi
  return 1
}

if [[ "$IS_GLOBAL" = true ]]; then
  echo "Scope       : FULL WORKSPACE (triggered by: ${GLOBAL_TRIGGER})"
  while IFS='|' read -r dir name is_web; do
    if [[ "$is_web" = "true" ]]; then
      WASM_CHECK_PKGS+=("$name")
    else
      NATIVE_CHECK_PKGS+=("$name")
    fi
    if has_test_suite "$dir"; then
      TEST_PKGS+=("$name")
    fi
  done <<< "$META_LIST"
else
  echo "Scope       : DYNAMIC SELECTIVE"
  while IFS='|' read -r dir name is_web; do
    # 检查是否有变更命中该目录
    for f in $ALL_CHANGED; do
      if [[ "$f" == "$dir" ]] || [[ "$f" == "$dir"/* ]]; then
        if [[ "$is_web" = "true" ]]; then
          WASM_CHECK_PKGS+=("$name")
        else
          NATIVE_CHECK_PKGS+=("$name")
        fi
        if has_test_suite "$dir"; then
          TEST_PKGS+=("$name")
        fi
        break
      fi
    done
  done <<< "$META_LIST"
fi

# 去重并排序
sort_unique() {
  if [[ $# -eq 0 ]]; then
    echo ""
  else
    printf "%s\n" "$@" | sort -u | tr '\n' ' ' | sed 's/ $//'
  fi
}

NATIVE_CHECK_UNIQ=$(sort_unique "${NATIVE_CHECK_PKGS[@]:-}")
WASM_CHECK_UNIQ=$(sort_unique "${WASM_CHECK_PKGS[@]:-}")
TEST_UNIQ=$(sort_unique "${TEST_PKGS[@]:-}")

NATIVE_COUNT=$(wc -w <<< "$NATIVE_CHECK_UNIQ")
WASM_COUNT=$(wc -w <<< "$WASM_CHECK_UNIQ")
TEST_COUNT=$(wc -w <<< "$TEST_UNIQ")

echo "Native check: ${NATIVE_COUNT} package(s)"
[[ -n "$NATIVE_CHECK_UNIQ" ]] && echo "              ${NATIVE_CHECK_UNIQ}"
echo "WASM check  : ${WASM_COUNT} package(s)"
[[ -n "$WASM_CHECK_UNIQ" ]] && echo "              ${WASM_CHECK_UNIQ}"
echo "Cargo test  : ${TEST_COUNT} package(s)"
[[ -n "$TEST_UNIQ" ]] && echo "              ${TEST_UNIQ}"
echo "================================================================="

if [[ "$NATIVE_COUNT" -eq 0 && "$WASM_COUNT" -eq 0 && "$TEST_COUNT" -eq 0 ]]; then
  echo "No code crates affected by these changes (docs/markdown only). Execution skipped."
  exit 0
fi

if [[ "$DRY_RUN" = true ]]; then
  echo "[DRY-RUN] Execution skipped."
  exit 0
fi

# ------------------------------------------------------------------------------
# 4. 执行按需编译检查与测试
# ------------------------------------------------------------------------------
if [[ -n "$NATIVE_CHECK_UNIQ" ]]; then
  echo -e "\n>>> Running cargo check (native)..."
  CARGO_ARGS=()
  for p in $NATIVE_CHECK_UNIQ; do
    CARGO_ARGS+=("-p" "$p")
  done
  cargo check "${CARGO_ARGS[@]}"
fi

if [[ -n "$WASM_CHECK_UNIQ" ]]; then
  echo -e "\n>>> Running cargo check --target wasm32-unknown-unknown..."
  CARGO_ARGS=()
  for p in $WASM_CHECK_UNIQ; do
    CARGO_ARGS+=("-p" "$p")
  done
  cargo check --target wasm32-unknown-unknown "${CARGO_ARGS[@]}"
fi

if [[ -n "$TEST_UNIQ" ]]; then
  echo -e "\n>>> Running cargo test on affected crates..."
  CARGO_ARGS=()
  for p in $TEST_UNIQ; do
    CARGO_ARGS+=("-p" "$p")
  done
  cargo test "${CARGO_ARGS[@]}"
fi

echo -e "\n================================================================="
echo "  ALL AFFECTED CHECKS AND TESTS PASSED SUCCESSFULLY! "
echo "================================================================="
