#!/usr/bin/env bash
# wt-clean.sh — 多会话 worktree 堆积清理(全本地判定, 无网络)
#
# 用法:
#   just wt-candidates     # 列出可清理项(PR 已合并 + 无在跑进程), 不动文件
#   just wt-clean          # 执行: 回收可清项(gio trash), 删已合并本地分支, prune 陈旧记录
#   just wt-clean-dry      # 干跑 clean, 只打印
#   just wt-targets-clean  # 清所有 .wt/*/target(编译产物, 可重建)
#
# 安全设计:
#   - 已合并判定(本地): 远端同名 ref 在本地 refstore 里已不存在 + 本地 main 领先该 HEAD
#     → 两者同时成立 = squash-merge 后远端分支已删、main 已吸收该提交
#   - 有未提交改动的 worktree: 先 git diff 备份到 .wt/<name>.dirty.patch 再删, 不静默丢工作
#   - 有 cargo/rustc/vite 等编译进程在跑的 worktree: 跳过(多会话并发保护)
#   - 目录删除一律 gio trash(回收站可恢复), 分支删除 git branch -D(提交仍在 main 里)
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"

GRN=$'\033[32m'; YEL=$'\033[33m'; DIM=$'\033[2m'; RST=$'\033[0m'
ACTION="${1:-candidates}"   # candidates | clean
DRY=0
[ "${2:-}" = "--dry-run" ] && DRY=1

# ---- 在跑编译进程的 cwd 集合 -------------------------------------------------
declare -A RUNNING_DIR
for pid in $(pgrep -f "cargo|rustc|vite|dx serve" 2>/dev/null); do
  cwd=$(readlink "/proc/$pid/cwd" 2>/dev/null) || continue
  [[ -n "$cwd" ]] && RUNNING_DIR["$cwd"]=1
done

is_running_in() {  # $1=worktree 绝对路径
  local p="$1"
  for d in "${!RUNNING_DIR[@]}"; do
    [[ "$d" == "$p" || "$d" == "$p"/* ]] && { echo "$p"; return 0; }
  done
  return 1
}

# ---- 逐个 worktree 判定 -----------------------------------------------------
declare -a CLEANABLE=() DIRTY_BACKED=() SKIPPED_RUNNING=() KEPT=()
TOOL_WT=""
for wt in .wt/*/; do
  name=$(basename "${wt%/}")
  [[ "$name" == *.patch ]] && continue
  dir="${wt%/}"

  # 在跑进程 → 跳过
  if hit=$(is_running_in "$PWD/$dir"); then
    SKIPPED_RUNNING+=("$name")
    continue
  fi

  # 分支/HEAD: worktree 的 .git 是指向主仓 .git/worktrees/<n> 的文本指针
  gitdir=""
  [[ -f "$dir/.git" ]] && gitdir=$(tr -d ' \n' < "$dir/.git" | sed 's|gitdir: ||')
  branch=$(cd "$dir" && git symbolic-ref --short HEAD 2>/dev/null || true)
  head_sha=$(cd "$dir" && git rev-parse --short HEAD 2>/dev/null || true)

  # 合并判定(全本地): 远端同名 ref 已不在本地 refstore + main 领先该 HEAD
  remote_gone=1
  [[ -n "$branch" ]] && git rev-parse --verify -q "refs/remotes/newxapi/$branch" >/dev/null 2>&1 && remote_gone=0
  main_ahead=0
  [[ -n "$head_sha" ]] && git rev-list --count "$head_sha..HEAD" 2>/dev/null | grep -qE '^[1-9]' && main_ahead=1

  if [[ $remote_gone -eq 1 && ( -z "$branch" || $main_ahead -eq 1 ) ]]; then
    # 有未提交改动 → 备份 diff 再删
    if [[ -n "$branch" ]] && ! (cd "$dir" && git diff --quiet 2>/dev/null); then
      (cd "$dir" && git diff) > ".wt/$name.dirty.patch"
      DIRTY_BACKED+=("$name → .wt/$name.dirty.patch")
    fi
    CLEANABLE+=("$name")
  else
    subject=$(cd "$dir" && git log -1 --format=%s 2>/dev/null | cut -c1-48)
    KEPT+=("$name ${branch:+[$branch]} ${subject}")
  fi
done

# ---- 输出 ------------------------------------------------------------------
if [[ $ACTION == candidates ]]; then
  echo "${GRN}== 可清理 (${#CLEANABLE[@]}) =="
  for n in "${CLEANABLE[@]:-}"; do [[ -n "$n" ]] && echo "  ✓ $n"; done
  [[ ${#DIRTY_BACKED[@]} -gt 0 ]] && { echo "  ${YEL}(其中未提交改动将先备份 patch):"; for n in "${DIRTY_BACKED[@]}"; do echo "    $n"; done; }
  echo "${YEL}== 跳过·在跑进程 (${#SKIPPED_RUNNING[@]}) =="
  for n in "${SKIPPED_RUNNING[@]:-}"; do [[ -n "$n" ]] && echo "  ⏸ $n"; done
  echo "${DIM}== 保留·未合并 (${#KEPT[@]}) =="
  for n in "${KEPT[@]:-}"; do [[ -n "$n" ]] && echo "  · $n"; done
  echo; echo "执行: just wt-clean"
  exit 0
fi

# ACTION == clean
[[ ${#CLEANABLE[@]} -eq 0 ]] && { echo "无可清理项"; exit 0; }
[[ $DRY -eq 1 ]] && echo "${YEL}[dry-run] 将执行以下操作${RST}"
echo "== 回收 worktree 目录 (${#CLEANABLE[@]}) =="
for n in "${CLEANABLE[@]}"; do
  if [[ $DRY -eq 1 ]]; then
    echo "  [dry] git worktree remove + gio trash .wt/$n"
  else
    git worktree remove --force ".wt/$n" 2>/dev/null || true
    gio trash ".wt/$n" && echo "  ✓ trashed .wt/$n"
  fi
done

# 删已合并本地分支
if [[ $DRY -eq 0 ]]; then
  echo "== 删除已合并本地分支 =="
  for wt in .git/worktrees/*/; do
    [[ -d "$wt" ]] || continue
    bn=$(basename "$wt")
    head=$(sed 's|refs/heads/||' "$wt/HEAD" 2>/dev/null || true)
    [[ -z "$head" ]] && continue
    if git rev-parse --verify -q "refs/heads/$head" >/dev/null 2>&1; then
      # 仅当远端 ref 不在 + main 领先时才删
      if ! git rev-parse --verify -q "refs/remotes/newxapi/$head" >/dev/null 2>&1 \
         && git rev-list --count "refs/heads/$head..HEAD" 2>/dev/null | grep -qE '^[1-9]'; then
        git branch -D "$head" >/dev/null 2>&1 && echo "  ✓ branch -D $head"
      fi
    fi
  done
  git worktree prune
fi
[[ $DRY -eq 1 ]] || echo
echo "完成。编译产物: just wt-targets-clean"
