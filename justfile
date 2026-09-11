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
