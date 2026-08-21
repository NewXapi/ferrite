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
