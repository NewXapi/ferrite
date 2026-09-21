
# Ferrite 酒馆前端（apps/tavern-web，Dioxus 0.7 web）。
# 构建：dx build --release --platform web。
#   注意 dx 输出在 target/dx/tavern-web-app/release/web/public（不是 dist/，
#   且 target 是 cache mount 不进镜像层，必须 cp 出来再 COPY）。
#   应用名按 Cargo.toml 的 package name（tavern-web-app），不是目录名。
# 运行：caddy 服务静态资源并把 /tavern、/v1 反代到 api 容器
#       （web 端用相对路径，同源契约，见 apps/tavern-web/Dioxus.toml 的 proxy 段）。
# 端口 8081。仅在全功能形态下使用（个人形态无 tavern 路由）。

# ---- builder ----
FROM rust:1-bookworm AS builder
RUN cargo install dioxus-cli --locked --version 0.7.9
WORKDIR /app
RUN rustup target add wasm32-unknown-unknown
COPY Cargo.toml Cargo.lock ./
COPY apps ./apps
COPY crates ./crates
COPY tests ./tests
COPY db ./db
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cd apps/tavern-web && dx build --release --platform web \
      && cp -r /app/target/dx/tavern-web-app/release/web/public /out

# ---- runtime ----
FROM caddy:2-alpine
COPY deploy/tavern-web/Caddyfile /etc/caddy/Caddyfile
COPY --from=builder /out /srv
EXPOSE 8081
