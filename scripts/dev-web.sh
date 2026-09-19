#!/usr/bin/env bash
# dev-web 一站式启动：后端(复用/新起) + Ainotation 标注栈(默认开) + dx serve 前端。
#
# 用法:
#   scripts/dev-web.sh --port 8092 [选项]
#     --backend shared|fresh   复用共享后端(3211, 默认) | 起隔离后端(端口 = web端口+1000)
#     --login auto|manual      免登录(debug-auto-login, 默认) | 需要登录
#     --aino on|off            接入 Ainotation 标注栈(默认 on) | 不接
#
# 长跑约定: 本脚本整体作为运行时的持久后台任务启动(omp hub start / 等价物)，
# dx serve 前台运行；service/bridge/隔离后端以 nohup+pidfile 守护(同 dev-backend.sh 模式)，
# 生命周期跟随开发会话，just aino-check 可体检。
#
# 共享后端红线: shared 模式只复用不代起 —— 共享实例必须从主检出启动
# (见 .agent/rules/dev-env.md「两条红线」)，未运行时本脚本仅提示。
set -euo pipefail

PORT="8090"
BACKEND="shared"
LOGIN="auto"
AINO="on"

usage() { sed -n '2,12p' "${BASH_SOURCE[0]}"; exit 1; }
die() { echo "dev-web: $*" >&2; exit 1; }

while [ $# -gt 0 ]; do
    case "$1" in
        --port) PORT="$2"; shift 2 ;;
        --backend) BACKEND="$2"; shift 2 ;;
        --login) LOGIN="$2"; shift 2 ;;
        --aino) AINO="$2"; shift 2 ;;
        --help|-h) usage ;;
        *) die "未知参数: $1 (--help 查看用法)" ;;
    esac
done
case "$BACKEND" in shared|fresh) ;; *) die "--backend 只能是 shared|fresh" ;; esac
case "$LOGIN" in auto|manual) ;; *) die "--login 只能是 auto|manual" ;; esac
case "$AINO" in on|off) ;; *) die "--aino 只能是 on|off" ;; esac
[[ "$PORT" =~ ^[0-9]+$ ]] || die "--port 需为数字"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ADMIN_WEB="$REPO_ROOT/apps/admin-web"

# ---------- Ainotation 标注栈 (aino=on) ----------
AINO_URL=""
ensure_aino() {
    local cli
    cli=$(ls "$HOME"/.npm/_npx/*/node_modules/@ainotation/mcp/dist/cli.mjs 2>/dev/null | head -1)
    [ -n "$cli" ] || { echo "⚠️  ainotation CLI 未找到(先 npx --yes @ainotation/mcp@beta --help 预热)，本次不接标注"; return 1; }

    # service: 健康则复用，否则守护拉起(失败时按 SKILL.md 排障表 repair 后重试一次)
    start_service() {
        nohup node "$cli" service >>/tmp/aino-service.log 2>&1 &
        echo $! >/tmp/aino-service.pid
        for _ in $(seq 1 15); do
            [ -f "$HOME/.ainotation/service/connection.json" ] && break
            sleep 1
        done
        AINO_URL=$(jq -r .url "$HOME/.ainotation/service/connection.json" 2>/dev/null || true)
        [ -n "$AINO_URL" ] && curl -sf -m 3 "$AINO_URL/control/health" \
            -H "Authorization: Bearer $(jq -r .token "$HOME/.ainotation/service/connection.json")" >/dev/null 2>&1
    }
    if [ -f "$HOME/.ainotation/service/connection.json" ]; then
        AINO_URL=$(jq -r .url "$HOME/.ainotation/service/connection.json")
        if curl -sf -m 3 "$AINO_URL/control/health" \
            -H "Authorization: Bearer $(jq -r .token "$HOME/.ainotation/service/connection.json")" >/dev/null 2>&1; then
            echo "ainotation service ✓ $AINO_URL"
        else
            AINO_URL=""
        fi
    fi
    if [ -z "$AINO_URL" ]; then
        start_service || true
        if [ -z "$AINO_URL" ]; then
            # 典型因: 旧实例残留无主 coordinator socket, 报 already locked —— repair 后重试一次
            node "$cli" repair --json >/dev/null 2>&1 || true
            start_service || true
        fi
        [ -n "$AINO_URL" ] || { echo "⚠️  service 启动失败(见 /tmp/aino-service.log)，本次不接标注"; return 1; }
        echo "ainotation service ✓ $AINO_URL (pid $(cat /tmp/aino-service.pid))"
    fi

    # bridge: :44090 有 token 则复用，否则守护拉起(桥内部自带服务等待与续租)
    if curl -sf -m 3 http://127.0.0.1:44090/connection.json 2>/dev/null | jq -e .token >/dev/null 2>&1; then
        echo "ainotation bridge ✓ :44090"
    else
        (cd "$ADMIN_WEB" && nohup node scripts/ainotation-bridge.mjs >>/tmp/aino-bridge.log 2>&1 & echo $! >/tmp/aino-bridge.pid)
        for _ in $(seq 1 15); do
            curl -sf -m 2 http://127.0.0.1:44090/connection.json 2>/dev/null | jq -e .token >/dev/null 2>&1 && break
            sleep 1
        done
        curl -sf -m 3 http://127.0.0.1:44090/connection.json 2>/dev/null | jq -e .token >/dev/null 2>&1 \
            && echo "ainotation bridge ✓ :44090 (pid $(cat /tmp/aino-bridge.pid))" \
            || { echo "⚠️  bridge 启动失败(见 /tmp/aino-bridge.log)，本次不接标注"; return 1; }
    fi
}

# ---------- 后端 ----------
ensure_shared_backend() {
    # 只探测不代起: 共享实例必须从主检出启动(红线)
    if bash "$REPO_ROOT/scripts/dev-backend.sh" status 2>/dev/null | grep -qiE 'running|pid'; then
        echo "共享后端 ✓ 127.0.0.1:3211"
    else
        echo "⚠️  共享后端未运行。红线: 共享实例须从主检出启动:"
        echo "    cd <主检出> && scripts/dev-backend.sh start"
        echo "   (后端起来前, 页面 /api 会 500)"
    fi
}

FRESH_PIDFILE=""
FRESH_TOML_BAK=""
ensure_fresh_backend() {
    local bport=$((PORT + 1000))
    FRESH_PIDFILE="/tmp/ferrite-devweb-$PORT-backend.pid"
    if [ -f "$FRESH_PIDFILE" ] && kill -0 "$(cat "$FRESH_PIDFILE")" 2>/dev/null; then
        echo "隔离后端 ✓ 127.0.0.1:$bport (复用 pid $(cat "$FRESH_PIDFILE"))"
    else
        cd "$REPO_ROOT"
        [ -x target/debug/ferrite ] || { echo "构建后端..."; cpulimit -l 70 -i -- cargo build -p api; }
        FERRITE_DEV_LISTEN="127.0.0.1:$bport" FERRITE_JWT_SECRET="ferrite-local-development-secret-2026" \
            FERRITE_JWT_ACCESS_TTL_SECS=43200 \
            nohup ./target/debug/ferrite >>"/tmp/ferrite-devweb-$PORT-backend.log" 2>&1 &
        echo $! >"$FRESH_PIDFILE"
        for _ in $(seq 1 20); do
            curl -s -o /dev/null "http://127.0.0.1:$bport/api/dashboard" 2>/dev/null && break
            sleep 1
        done
        if curl -s -o /dev/null "http://127.0.0.1:$bport/api/dashboard" 2>/dev/null ||
            curl -s -o /dev/null "http://127.0.0.1:$bport/api/dashboard" 2>/dev/null; then
            echo "隔离后端 ✓ 127.0.0.1:$bport (pid $(cat "$FRESH_PIDFILE"))"
        else
            rm -f "$FRESH_PIDFILE"
            die "隔离后端启动失败, 见 /tmp/ferrite-devweb-$PORT-backend.log"
        fi
        echo "  ⚠️  数据库仍取本 worktree config/config.toml —— 要数据隔离请改指向另一库后重跑 (dev-env.md 3.2)"
    fi
    # 前端代理指向隔离后端 (dx 不支持代理参数, 临时改 Dioxus.toml, 退出还原)
    FRESH_TOML_BAK="$ADMIN_WEB/Dioxus.toml.devweb-bak"
    cp "$ADMIN_WEB/Dioxus.toml" "$FRESH_TOML_BAK"
    sed -i "s|http://127.0.0.1:3211|http://127.0.0.1:$bport|" "$ADMIN_WEB/Dioxus.toml"
    grep -q "127.0.0.1:$bport" "$ADMIN_WEB/Dioxus.toml" || echo "⚠️  Dioxus.toml 代理改写未生效, 前端仍指向共享后端"
    trap 'restore_fresh' EXIT INT TERM
}

restore_fresh() {
    [ -n "$FRESH_TOML_BAK" ] && [ -f "$FRESH_TOML_BAK" ] && mv "$FRESH_TOML_BAK" "$ADMIN_WEB/Dioxus.toml"
    echo "Dioxus.toml 代理已还原"
}

# ---------- 主流程 ----------
if [ "$AINO" = "on" ]; then ensure_aino || true; fi
case "$BACKEND" in
    shared) ensure_shared_backend ;;
    fresh) ensure_fresh_backend ;;
esac

cd "$ADMIN_WEB"
FEATURES=()
[ "$LOGIN" = "auto" ] && FEATURES=(--features debug-auto-login)
echo "== dx serve :$PORT (login=$LOGIN backend=$BACKEND aino=$AINO) =="
dx serve --platform web --port "$PORT" --watch false "${FEATURES[@]}"
