#!/usr/bin/env bash
# 共享 dev 后端生命周期管理 — 所有 .wt 会话的前端代理都指向 127.0.0.1:3211。
#
# 用法:
#   scripts/dev-backend.sh start    # 起共享后端 (二进制缺失时先构建)
#   scripts/dev-backend.sh update   # 重建二进制并重启 (~3s, 登录态不丢)
#   scripts/dev-backend.sh stop     # 停止
#   scripts/dev-backend.sh status   # 健康检查
#
# 更新协议: 重启对前端是透明的 — JWT secret 与 PG 会话都持久, 已登录用户
# 无需重登; 正在飞行中的请求会失败一次, 页面上的"重试"按钮即可恢复。
# 必须从主检出 (跟踪 newxapi/main) 运行, 不从 .wt 工作树启动共享实例。
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PIDFILE="/tmp/ferrite-dev-backend.pid"
LOGFILE="/tmp/ferrite-dev-backend.log"
LISTEN="${FERRITE_DEV_LISTEN:-127.0.0.1:3211}"
# dev 默认拉长 access token 有效期到 12h (代码默认 15min, 见 auth/src/service.rs)
JWT_TTL="${FERRITE_JWT_ACCESS_TTL_SECS:-43200}"
JWT_SECRET="${FERRITE_JWT_SECRET:-ferrite-local-development-secret-2026}"

is_running() {
    [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null
}

stop_backend() {
    if is_running; then
        kill "$(cat "$PIDFILE")" 2>/dev/null || true
        sleep 1
        echo "stopped pid $(cat "$PIDFILE")"
    fi
    rm -f "$PIDFILE"
}

start_backend() {
    cd "$REPO_ROOT"
    if [ ! -x target/debug/ferrite ]; then
        echo "binary missing, building (cpulimit 70%)..."
        cpulimit -l 70 -i -- cargo build -p api
    fi
    FERRITE_JWT_SECRET="$JWT_SECRET" FERRITE_JWT_ACCESS_TTL_SECS="$JWT_TTL" \
        nohup ./target/debug/ferrite >>"$LOGFILE" 2>&1 &
    echo $! >"$PIDFILE"
    for _ in $(seq 1 20); do
        if curl -sf -o /dev/null "http://$LISTEN/api/dashboard" 2>/dev/null ||
            curl -s -o /dev/null "http://$LISTEN/api/dashboard" 2>/dev/null; then
            echo "dev backend up on $LISTEN (jwt ttl ${JWT_TTL}s), pid $(cat "$PIDFILE")"
            return 0
        fi
        sleep 1
    done
    echo "backend failed to start, see $LOGFILE" >&2
    exit 1
}

case "${1:-status}" in
    start)
        if is_running; then
            echo "already running, pid $(cat "$PIDFILE")"
        else
            start_backend
        fi
        ;;
    update)
        stop_backend
        cd "$REPO_ROOT"
        echo "rebuilding (cpulimit 70%)..."
        cpulimit -l 70 -i -- cargo build -p api
        start_backend
        ;;
    stop)
        stop_backend
        ;;
    status)
        if is_running && curl -s -o /dev/null "http://$LISTEN/api/dashboard"; then
            echo "running, pid $(cat "$PIDFILE"), $LISTEN"
        else
            echo "not running"
            exit 1
        fi
        ;;
    *)
        echo "usage: $0 {start|update|stop|status}" >&2
        exit 1
        ;;
esac
