# syntax=docker/dockerfile:1

# Ferrite 后台管理前端（apps/admin-web，Dioxus 0.7 web）。
# 构建：dx build --release --platform web。
#   注意 dx 输出在 target/dx/admin-web/release/web/public（不是 dist/），
#   且 target 是 cache mount 不进镜像层，必须 cp 出来再 COPY。
# 运行：caddy 服务静态资源并把 /api 反代到 api 容器（web 端用相对 /api 路径，
#       同源契约，见 apps/admin-web/src/lib.rs）。端口 8080。

# ---- builder ----
FROM rust:1-bookworm AS builder
RUN cargo install dioxus-cli --locked --version 0.7.9
WORKDIR /app
# 工作区成员必须完整，否则根 Cargo.toml 的 members 解析失败。
COPY Cargo.toml Cargo.lock ./
COPY apps ./apps
COPY crates ./crates
COPY tests ./tests
COPY db ./db
RUN rustup target add wasm32-unknown-unknown
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cd apps/admin-web && dx build --release --platform web \
     && cp /app/target/dx/admin-web/release/web/public /out

# ---- runtime ----
FROM caddy:2-alpine
COPY deploy/admin-web/Caddyfile /etc/caddy/Caddyfile
COPY --from=builder /out /srv
EXPOSE 8080
