
# Ferrite 后端镜像（apps/api，bin 名 ferrite）。
# 双入口由 cargo feature 决定形态（构建上下文是仓库根，-f 指向本文件）：
#   docker build -f deploy/api/Dockerfile .                                         # 全功能（default = tavern + billing）
#   docker build -f deploy/api/Dockerfile --build-arg FEATURES=--no-default-features .   # 个人形态
#   docker build -f deploy/api/Dockerfile --build-arg FEATURES="--no-default-features --features tavern" .
# 运行时必须提供 /app/config/config.toml（示例见 config/config.toml.example，
# 真实文件 gitignore，挂卷或 COPY 进去）与 FERRITE_JWT_SECRET 环境变量；
# database_url 也写在配置里，指向可达的 Postgres。

ARG FEATURES=""

# ---- builder ----
FROM rust:1-bookworm AS builder
ARG FEATURES
WORKDIR /app
# db/migrations 由 db-bootstrap 的 sqlx::migrate! 在编译期嵌入
# （路径 ../../../db/migrations 相对该 crate），必须进构建上下文。
COPY Cargo.toml Cargo.lock ./
COPY apps ./apps
COPY crates ./crates
COPY tests ./tests
COPY db ./db
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release -p api ${FEATURES} && cp target/release/ferrite /ferrite

# ---- runtime ----
# debian:bookworm-slim 提供 glibc/libgcc（与 builder 同代 glibc，避免 ABI 不匹配）；全链路 rustls（reqwest rustls-tls、sqlx 无 TLS
# feature），无 openssl 动态依赖，无需额外运行时库。
FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /ferrite /app/ferrite
COPY config/config.toml.example /app/config/config.toml.example
EXPOSE 3000
ENTRYPOINT ["/app/ferrite"]
