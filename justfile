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
#   起前端 web            : cd apps/admin-web && dx serve --platform web --port 8090
#                           (务必用会话的持久后台任务起, 不要 nohup &, 见文末疑难)

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


# 共享 dev 后端: start | update | stop | status
#   start   首次拉起 (二进制缺失会先 cargo build -p api)
#   update  改了 crates/api / apps/api 后重建并重启 (对前端透明, JWT/会话持久)
#   stop    停掉 (一般不用手动停, 多会话共享同一实例)
#   status  健康检查, 报 pid + 端口
# 注意: 必须从跟踪 newxapi/main 的主检出运行, 不要在 .wt 工作树里起共享实例
dev-backend *args:
    bash scripts/dev-backend.sh {{args}}

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
# 症状: dx serve 改了依赖 crate 但页面没变
#   → dx 对依赖 crate 改动不自动重建 wasm。手动 cargo rustc --profile wasm-dev 重编 wasm,
#     再 touch 任一 src 文件让 dx 重跑 bindgen, 或直接重启 dx serve。

