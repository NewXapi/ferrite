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

# PG 连接参数 (容器名/库可按环境覆盖)
PG_CONTAINER := "uf-local-postgres"
PG_USER := "ferrite"
PG_DB := "ferrite_smoke"

# 灌入 dev 种子数据 (幂等, 可重复执行; 生成器现场生成, 管道直通 psql)
db-seed:
    python3 db/dev/generate_seed.py | docker exec -i {{PG_CONTAINER}} psql -U {{PG_USER}} -d {{PG_DB}} -v ON_ERROR_STOP=1

# 只清理种子行, 不碰真实数据
db-reset:
    docker exec -i {{PG_CONTAINER}} psql -U {{PG_USER}} -d {{PG_DB}} -v ON_ERROR_STOP=1 < db/dev/reset.sql


# 共享 dev 后端: start | update | stop | status
dev-backend *args:
    bash scripts/dev-backend.sh {{args}}

# dev 环境体检：查共享后端(3211)/前端 serve(8090) 监听 + 打印进程卫生提醒
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

