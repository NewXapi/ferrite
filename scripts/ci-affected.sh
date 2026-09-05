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
# 显式 --base 时只认 merge-base..HEAD，不做任何 fallback：
# 静默退化成 `git diff <base>`（工作树 vs base 全量对比）会把 worktree 里的
# untracked 产物全部算成改动，直接把 scope 顶成 FULL WORKSPACE。
DIFF_TARGET=""
CHANGED_FILES=""
BASE_RESOLVED=""

resolve_diff() {
  local base="$1"
  local mb
  mb=$(git merge-base "$base" HEAD 2>/dev/null) || return 1
  DIFF_TARGET="${base}...HEAD"
  BASE_RESOLVED="$mb"
  CHANGED_FILES=$(git diff --name-only "$mb" HEAD 2>/dev/null) || return 1
  return 0
}

if [[ -n "$BASE_REF" ]]; then
  if ! git rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
    echo "ERROR: base ref '$BASE_REF' 不存在" >&2
    exit 1
  fi
  if ! resolve_diff "$BASE_REF"; then
    echo "ERROR: 无法计算 '$BASE_REF' 与 HEAD 的 merge-base diff" >&2
    exit 1
  fi
else
  for candidate in "newxapi/main" "origin/main" "remotes/newxapi/main" "main" "HEAD~1"; do
    if git rev-parse --verify "$candidate" >/dev/null 2>&1 && resolve_diff "$candidate"; then
      break
    fi
  done
fi

# 补充工作区改动：CI 上工作树本就干净，这里只为本地 dry-run 方便。
# 只取已跟踪文件的改动，不含 untracked —— untracked 多是构建产物（dx.log、
# tailwind.out.css、target/），算进来会误把 scope 顶成全量。
UNCOMMITTED=$(git diff --name-only HEAD 2>/dev/null || true)
STAGED=$(git diff --name-only --cached HEAD 2>/dev/null || true)

ALL_CHANGED=$(printf "%s\n%s\n%s\n" "$CHANGED_FILES" "$UNCOMMITTED" "$STAGED" | sed '/^$/d' | sort -u || true)
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
# 只有"影响面无法从依赖图推导"的改动才升级为全量：依赖版本、toolchain、CI 自身。
# crates/contract 不在此列 —— 它是普通 workspace 成员，反向依赖闭包能精确算出
# 受影响的下游包（实测 33 个，比全量 53 个更准）。
IS_GLOBAL=false
GLOBAL_TRIGGER=""

for f in $ALL_CHANGED; do
  case "$f" in
    Cargo.lock|Cargo.toml|rust-toolchain.toml|.github/*|scripts/*)
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

# workspace 内部依赖边，每行 "依赖者 被依赖者"。
# 只取带 path 的依赖（本地路径依赖 = workspace 成员），忽略 crates.io 依赖。
get_dep_edges() {
  cargo metadata --format-version 1 | jq -r '
    [.workspace_members[]] as $ws |
    .packages[] | select(.id as $id | $ws | index($id)) |
    .name as $me |
    .dependencies[]? | select(.path != null) | "\($me) \(.name)"
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

# 反向依赖闭包：给定直接命中的包（seed），沿"谁依赖我"方向 BFS，
# 补齐所有间接受影响的下游包。
#
# 为什么必须做：改 tavern-storage 只跑 tavern-storage 会漏掉 api 与 tests-e2e
# —— 它们依赖它，编译能过但测试断言可能已经破了。
expand_rdeps() {
  local seeds="$1"
  [[ -z "${seeds// /}" ]] && return 0
  printf '%s\n' "$DEP_EDGES" | awk -v seeds="$seeds" '
    NF == 2 { rev[$2] = rev[$2] " " $1 }
    END {
      n = split(seeds, s, " ")
      for (i = 1; i <= n; i++) if (s[i] != "") { seen[s[i]] = 1; q[++tail] = s[i] }
      head = 1
      while (head <= tail) {
        cur = q[head++]
        m = split(rev[cur], parents, " ")
        for (j = 1; j <= m; j++) {
          p = parents[j]
          if (p != "" && !(p in seen)) { seen[p] = 1; q[++tail] = p }
        }
      }
      for (k in seen) print k
    }
  ' | sort -u
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
  # 第一步：路径命中，得到直接改动的包（seed）
  SEED_PKGS=()
  while IFS='|' read -r dir name is_web; do
    for f in $ALL_CHANGED; do
      if [[ "$f" == "$dir" ]] || [[ "$f" == "$dir"/* ]]; then
        SEED_PKGS+=("$name")
        break
      fi
    done
  done <<< "$META_LIST"

  SEED_LIST=$(printf '%s\n' "${SEED_PKGS[@]:-}" | sed '/^$/d' | sort -u | tr '\n' ' ' | sed 's/ $//')
  SEED_COUNT=$(wc -w <<< "$SEED_LIST")

  # 第二步：沿反向依赖图展开，补齐下游受影响的包
  DEP_EDGES=$(get_dep_edges)
  AFFECTED_LIST=$(expand_rdeps "$SEED_LIST" | tr '\n' ' ' | sed 's/ $//')
  AFFECTED_COUNT=$(wc -w <<< "$AFFECTED_LIST")

  echo "Scope       : DYNAMIC SELECTIVE (+rdeps)"
  echo "Seed        : ${SEED_COUNT} package(s) 直接命中"
  [[ -n "$SEED_LIST" ]] && echo "              ${SEED_LIST}"
  if [[ "$AFFECTED_COUNT" -gt "$SEED_COUNT" ]]; then
    echo "Closure     : ${AFFECTED_COUNT} package(s) (+$((AFFECTED_COUNT - SEED_COUNT)) 经反向依赖补齐)"
  fi

  # 第三步：按闭包结果分类到 native / wasm / test
  while IFS='|' read -r dir name is_web; do
    for pkg in $AFFECTED_LIST; do
      if [[ "$pkg" == "$name" ]]; then
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
// test
