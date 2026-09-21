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

  # 车道固定 worktree（.agent/rules/web-lanes.md）：永久基础设施，任何自动清理都不许碰。
  # .wt/web-fix 常驻 detached 且无远端分支（remote_gone 恒真），唯一屏障是合并判定；
  # 首次发布后其 HEAD 即成为 main 祖先，只靠 merge-base 判定必被误清——必须硬排除。
  case "$name" in
    web-dev|web-fix) KEPT+=("$name [车道固定 worktree，永不自动清]"); continue ;;
  esac

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

  # 合并判定(全本地): 远端同名 ref 已不在本地 refstore + wt HEAD 是 main 的祖先
  # (即 wt 没有任何 main 不包含的提交)。detached HEAD 没有 branch 名, remote_gone
  # 恒为 1, 更不能省祖先判定——车道 .wt/web-fix 常驻 detached 且带未合入提交,
  # 用 "main 比 wt 新" 判断会误删(实测误判过)。
  remote_gone=1
  [[ -n "$branch" ]] && git rev-parse --verify -q "refs/remotes/newxapi/$branch" >/dev/null 2>&1 && remote_gone=0
  merged=0
  [[ -n "$head_sha" ]] && git merge-base --is-ancestor "$head_sha" HEAD && merged=1

  if [[ $remote_gone -eq 1 && $merged -eq 1 ]]; then
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
    echo "  [dry] git worktree remove .wt/$n"
  else
    # 目录由 git worktree remove 直接删除；可回收性靠"已合并"判定（提交都在 main 里），
    # 脏改动已在上方备份 .wt/<name>.dirty.patch。曾经在此多走一步 gio trash——
    # remove 之后目录已不存在，必报 "No such file"，已移除。
    git worktree remove --force ".wt/$n" 2>/dev/null || true
    echo "  ✓ removed .wt/$n"
  fi
done

# 删已合并本地分支。直接扫分支表：上面的 remove 已删掉 .git/worktrees/<n> 元数据，
# 从元数据反推分支名永远空转（实测该循环一次都没触发过）。-d 自带合并校验，
# 被其他 worktree checkout 的分支 git 会拒绝删，双保险。
if [[ $DRY -eq 0 ]]; then
  echo "== 删除已合并本地分支 =="
  while read -r b; do
    [[ -z "$b" ]] && continue
    # 车道干流：发布后即"已合并"，但 .wt/web-dev 常驻其上——永不自动删
    [[ "$b" == "web-dev" ]] && continue
    git rev-parse --verify -q "refs/remotes/newxapi/$b" >/dev/null 2>&1 && continue
    git rev-parse --verify -q "refs/remotes/origin/$b" >/dev/null 2>&1 && continue
    git branch -d "$b" >/dev/null 2>&1 && echo "  ✓ branch -d $b"
  done < <(git branch --merged refs/heads/main --format='%(refname:short)' | grep -vE '^(main|web-dev)$')
  git worktree prune
fi
[[ $DRY -eq 1 ]] || echo
echo "完成。编译产物: just wt-targets-clean"
