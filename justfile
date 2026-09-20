# Ferrite — justfile

# 默认：check
default: check

# 编译检查
check:
    cargo check

# 格式化
fmt:
    cargo fmt --all

# 格式化检查（不改文件）
fmt-check:
    cargo fmt --all --check

# clippy
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# 构建二进制
build:
    cargo build

# 运行
run:
    ./target/debug/ferrite

# 测试
test:
    cargo test --all

# 全套检查
verify: fmt-check clippy check

# ---------- dev 数据与共享后端 (详见 db/dev/README.md) ----------
#
# 使用场景速查：
#   首次搭本地环境        : just db-seed && just dev-backend start
#   改了 crates/api 代码   : just dev-backend update   (重建+重启, 登录态不丢, ~3s)
#   前端联调起不来/报 500 : just dev-check            (查 3211/8090 监听 + 后端状态)
#   重置脏数据            : just db-reset && just db-seed
#   起前端 web            : just dev-web 8090          (dx serve --platform web, --watch false 防 watch 卡死)
#   免登录调试前端        : just dev-web 8090 debug    (debug-auto-login feature, 自动登录 dev 种子 admin_dev;
#                                          打开 #login/#signup/#auth 仍可手动调试登录页)

# PG 连接参数 (容器名/库可按环境覆盖)
PG_CONTAINER := "uf-local-postgres"
PG_USER := "ferrite"
PG_DB := "ferrite_smoke"

# 灌入 dev 种子数据 (幂等, 可重复执行; 生成器现场生成, 管道直通 psql)
# 场景: 首次建库 / db-reset 后重灌 / 需要真实纹理的用量与渠道数据做前端联调
db-seed:
    python3 db/dev/generate_seed.py | docker exec -i {{PG_CONTAINER}} psql -U {{PG_USER}} -d {{PG_DB}} -v ON_ERROR_STOP=1

# 只清理种子行, 不碰真实数据
# 场景: 种子行被手动改乱时清一遍再 db-seed (db-seed 本身幂等, 多数情况直接重灌即可)
db-reset:
    docker exec -i {{PG_CONTAINER}} psql -U {{PG_USER}} -d {{PG_DB}} -v ON_ERROR_STOP=1 < db/dev/reset.sql


# ---------- 多会话 worktree 堆积清理 (scripts/wt-clean.sh) ----------
#   just wt-candidates       只列可清项(PR 已合并 + 无在跑进程), 不动文件
#   just wt-clean            回收可清项(gio trash 进回收站) + 删已合并分支
#   just wt-targets-clean    清所有 .wt/*/target 与根 target(编译产物, 可重建)
# 场景: 多会话并行开发后 .wt/ 膨胀(每个 worktree 一份独立 cargo target)
wt-candidates:
    bash scripts/wt-clean.sh candidates

wt-clean:
    bash scripts/wt-clean.sh clean

wt-clean-dry:
    bash scripts/wt-clean.sh clean --dry-run

wt-targets-clean:
    #!/usr/bin/env bash
    set -u; total=0
    for t in .wt/*/target target; do
      [ -d "$t" ] || continue
      sz=$(du -sh "$t" | cut -f1); n=0
      find "$t" -mindepth 1 -maxdepth 1 -exec sh -c 'gio trash "$1" 2>/dev/null && true' _ {} \; 2>/dev/null
      gio trash "$t" 2>/dev/null && { echo "  ✓ trashed $t ($sz)"; total=$((total+1)); }
    done
    [ "$total" -eq 0 ] && echo "无可清 target"

# 共享 dev 后端: start | update | stop | status
#   start   首次拉起 (二进制缺失会先 cargo build -p api)
#   update  改了 crates/api / apps/api 后重建并重启 (对前端透明, JWT/会话持久)
#   stop    停掉 (一般不用手动停, 多会话共享同一实例)
#   status  健康检查, 报 pid + 端口
# 注意: 必须从跟踪 newxapi/main 的主检出运行, 不要在 .wt 工作树里起共享实例
dev-backend *args:
    bash scripts/dev-backend.sh {{args}}

# 起 admin 前端 web dev server（务必用会话的持久后台任务起, 起后 ss 验证端口, 见文末疑难）
#   普通前端    : just dev-web 8090
#   免登录调试  : just dev-web 8090 debug
#     debug 档启用 debug-auto-login feature: 无 token 且不在登录页时自动登录 dev 种子
#     账号 admin_dev (401 清会话后也会先自动重登); 打开 #login/#signup/#auth 仍可
#     手动调试登录页, 主动「退出登录」不会被自动重登顶掉。彻底关闭用普通档重新起。
#   全部 --watch false (仓库已知 dx watch 重建卡死)。
#   ⚠️ 改了依赖 crate（ui-components 等）后页面没变：dx 不会自动重编 wasm，
#      跑 `just dev-web-rebuild <port>`（或 `just dev-web-rebuild <port> debug`）一键重编+重启，
#      浏览器再强刷一次。
dev-web port="8090" mode="":
    #!/usr/bin/env bash
    # 锚定 justfile 所在目录（= 仓库根），使配方可从任意 cwd 调用
    # ponytail: just 无 justfile_directory 变量（1.58 实测），用内置 justfile() + shell dirname
    cd "$(dirname "{{ justfile() }}" )/apps/admin-web"
    if [ "{{mode}}" = "debug" ]; then
      dx serve --platform web --port {{port}} --watch false --features debug-auto-login
    else
      dx serve --platform web --port {{port}} --watch false
    fi

# 改完代码一键重编 wasm + 重启 dx（替代手动的 kill+restart 仪式）
#   普通: just dev-web-rebuild 8090     |  免登录调试档: just dev-web-rebuild 8090 debug
#   档位（debug 与否）要和当前跑着的 dx 一致；重启后浏览器强刷一次。
#   ponytail: 配交互 dev 循环（人等自己的构建），这里不套 cpulimit；agent 会话发起的构建仍按 AGENTS.md 套
dev-web-rebuild port="8090" mode="":
    #!/usr/bin/env bash
    set -e
    cd "$(dirname "{{ justfile() }}" )/apps/admin-web"
    echo "== rebuild wasm (dev profile) =="
    cargo build --target wasm32-unknown-unknown
    echo "== restart dx (port {{port}}, mode {{mode}}) =="
    pid="$(ss -ltnp 2>/dev/null | grep ":{{port}} " | grep -oP 'pid=\K[0-9]+' | head -1 || true)"
    if [ -n "$pid" ]; then kill "$pid"; sleep 1; fi
    if [ "{{mode}}" = "debug" ]; then
      dx serve --platform web --port {{port}} --watch false --features debug-auto-login
    else
      dx serve --platform web --port {{port}} --watch false
    fi
    echo "== done: hard-refresh the browser (wasm/js are cached) =="

# ---------- Ainotation 视觉标注反馈栈 (用法详见 .agent/skills/ainotation-web/SKILL.md) ----------
#   重打 SDK bundle : just aino-bundle   (改了 ainotation-entry.ts / 升级 SDK 后; 改完需重启 dx)
#   起 service      : just aino-service  (用运行时后台任务启动; 就绪判据 ~/.ainotation/service/connection.json)
#   起同步桥        : just aino-bridge   (同上; 就绪判据日志 "project ready" + :44090 端点)
#   体检            : just aino-check    (service 注册表 + 桥端点 + 前端连通)
#   顺序硬约束: service 必须先于 agent 的 ainotation MCP 可用, 否则 MCP 工具调用会挂起
#   端口不一致时: AINO_ORIGIN=http://127.0.0.1:<端口> just aino-bridge
aino-bundle:
    #!/usr/bin/env bash
    cd "$(dirname "{{ justfile() }}" )/apps/admin-web"
    bun install
    bun run aino
    echo "✓ bundle 已重打: assets/ainotation/ainotation.iife.js (内容变化会改指纹, 记得重启 dx)"

aino-service:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -f "$HOME/.ainotation/service/connection.json" ]; then
      url=$(jq -r .url "$HOME/.ainotation/service/connection.json")
      token=$(jq -r .token "$HOME/.ainotation/service/connection.json")
      if curl -sf -m 3 "$url/control/health" -H "Authorization: Bearer $token" >/dev/null 2>&1; then
        echo "service 已在运行: $url"; exit 0
      fi
    fi
    CLI=$(ls "$HOME"/.npm/_npx/*/node_modules/@ainotation/mcp/dist/cli.mjs 2>/dev/null | head -1)
    if [ -n "$CLI" ]; then exec node "$CLI" service; else exec npx --yes @ainotation/mcp@beta service; fi

aino-bridge:
    #!/usr/bin/env bash
    set -euo pipefail
    cd "$(dirname "{{ justfile() }}" )/apps/admin-web"
    exec node scripts/ainotation-bridge.mjs

aino-check port="8090":
    #!/usr/bin/env bash
    echo "== service =="
    if [ -f "$HOME/.ainotation/service/connection.json" ]; then
      url=$(jq -r .url "$HOME/.ainotation/service/connection.json")
      token=$(jq -r .token "$HOME/.ainotation/service/connection.json")
      curl -sf -m 3 "$url/control/health" -H "Authorization: Bearer $token" >/dev/null 2>&1 \
        && echo "  ✓ $url" || echo "  ✗ $url 不健康 (just aino-service 重启)"
      curl -sf -m 3 "$url/control/projects" -H "Authorization: Bearer $token" 2>/dev/null \
        | jq -r '.projects[] | "  project: \(.name) (\(.projectId[0:8]))"' 2>/dev/null || true
    else
      echo "  ✗ 未运行 (just aino-service)"
    fi
    echo "== 同步桥 (:44090) =="
    curl -sf -m 3 http://127.0.0.1:44090/connection.json 2>/dev/null \
      | jq -e -r '"  ✓ 桥活跃 service=" + .url + " token=" + (.token[0:8])' \
      || echo "  ✗ 未运行 (just aino-bridge)"
    echo "== 前端 (:{{port}}) =="
    curl -s -o /dev/null -m 3 -w '  :{{port}} -> %{http_code}\n' http://127.0.0.1:{{port}}/ \
      || echo "  :{{port}} 未监听 (just dev-web {{port}} debug)"

# dev 环境体检：查共享后端(3211)/前端 serve(8090) 监听 + 打印进程卫生提醒
# 场景: 前端页面报 500/连不上, 或 agent 开工前确认环境活着
dev-check:
    #!/usr/bin/env bash
    echo "== 监听检查 =="
    ss -ltn 2>/dev/null | grep -E ':3211|:8090' || echo "  (无 3211/8090 监听)"
    echo "== 共享后端状态 =="
    bash scripts/dev-backend.sh status || echo "  ⚠️  后端未运行: just dev-backend start"
    echo "== 进程卫生提醒 (详见 AGENTS.md「本机 dev 服务与进程卫生」) =="
    echo "  1. 长跑服务勿用 'nohup &'(Bash 调用结束回收进程组→服务静默死); 用持久后台任务 + ss 验证监听"
    echo "  2. 勿 pkill -f cargo/rustc (T 态≠死进程, 会误杀并行会话构建); 清理前 readlink /proc/<pid>/cwd"
    echo "  3. 用户报错但实测 200 → 先查浏览器缓存重放 (dx 日志 grep 该路径无请求 = 实锤)"

# ---------- 疑难问题 → 推荐处理 ----------
# 症状: 前端一直 500 "Connection refused" / dx 日志消失
#   → 后端或 dx serve 被回收(常见于用 nohup 起)。just dev-check 看监听;
#     后端 just dev-backend start; dx serve 用持久后台任务重起, 起后 ss 验证端口。
# 症状: 某卡片报 HTTP 404, 但 curl / 无缓存浏览器实测 200
#   → 浏览器(IAB)缓存重放了旧错误响应(代理误配期毒化), 请求根本没出网。
#     诊断: dx 代理日志 grep 该路径查无请求 = 实锤。处理: 清 IAB 缓存/重启 webview;
#     后端 /api /tavern 已加 Cache-Control: no-store (#143) 防复发。
# 症状: 本地构建卡死不动 (rustc 长时间 0 进展 / cargo 锁等待)
#   → 多半是别的会话构建被 cpulimit 遗留的 SIGSTOP 僵进程持锁。
#     诊断: ps -eo pid,stat,args | awk '$2 ~ /^T/' 找 T 态; readlink /proc/<pid>/cwd 确认归属;
#     无主的才 kill, 活会话的用 kill -CONT 恢复。绝不无脑 pkill cargo/rustc。
# 症状: dx serve 改了代码（含依赖 crate）但页面没变
#   → dx 对改动不自动重建 wasm（--watch false）。跑 `just dev-web-rebuild <port> [debug]`
#     一键重编 + 重启，浏览器强刷一次。（旧的 cargo rustc --profile wasm-dev 命令已失效：
#     仓库没有 wasm-dev profile，是 dx 0.6 的遗留提示。）

