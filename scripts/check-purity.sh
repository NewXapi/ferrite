#!/usr/bin/env bash
# crate 纯度守卫。见 docs/08-mvp.md §3.3
#
# protocol / provider 必须是纯函数：输入请求出请求，输入响应出 usage。
# 不碰 IO、不碰全局状态。今天靠人守很容易破，所以靠依赖图守。
#
# router 只回答「该试哪些渠道」，不负责发请求，所以也不该有 HTTP 客户端。
set -euo pipefail

cd "$(dirname "$0")/.."

fail=0

check() {
    local crate="$1"
    shift
    local tree
    tree="$(cargo tree -p "$crate" --edges normal -q 2>/dev/null)"
    local bad=0
    for dep in "$@"; do
        # cargo tree 的行形如 "├── reqwest v0.12.28"，前缀是多字节制表符，
        # 所以用「空白或行首 + 名字 + 空格 + v数字」匹配。
        if grep -qE "(^|[[:space:]])${dep} v[0-9]" <<<"$tree"; then
            echo "FAIL  $crate 不应依赖 $dep"
            bad=1
            fail=1
        fi
    done
    [ "$bad" -eq 0 ] && echo "ok    $crate"
    return 0
}

echo "crate 纯度检查"
check protocol reqwest axum tokio sqlx redis
check provider reqwest axum tokio sqlx redis
check router   reqwest axum sqlx redis

if [ "$fail" -ne 0 ]; then
    echo
    echo "纯度被破坏。要么把 IO 移到调用方，要么说明为什么必须破例。"
    exit 1
fi

echo "全部通过"
