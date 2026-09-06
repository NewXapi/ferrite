#!/usr/bin/env bash
# scripts/setup_bench_ssh.sh — 一键部署压测 SSH 密钥到服务器 + GitHub secrets
# 用法：./scripts/setup_bench_ssh.sh
# 需要：sshpass, ssh-copy-id, gh (GitHub CLI)

set -euo pipefail

KEY_PATH="$HOME/.ssh/id_bench"
PUB_KEY_PATH="$KEY_PATH.pub"
SECRET_NAME="BENCH_SSH_PRIVATE_KEY"
GITHUB_REPO="NewXapi/ferrite"

# 颜色
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() { echo -e "${GREEN}[✓]${NC} $*"; }
warn() { echo -e "${YELLOW}[!]${NC} $*"; }
error() { echo -e "${RED}[✗]${NC} $*" >&2; exit 1; }

# 检查依赖
check_deps() {
    command -v sshpass >/dev/null 2>&1 || error "缺少 sshpass，先安装：sudo apt install sshpass"
    command -v ssh-copy-id >/dev/null 2>&1 || error "缺少 ssh-copy-id，先安装：sudo apt install openssh-client"
    command -v gh >/dev/null 2>&1 || error "缺少 gh (GitHub CLI)，先安装：https://cli.github.com"
    gh auth status >/dev/null 2>&1 || error "gh 未登录，先运行：gh auth login"
}

# 生成密钥（如果不存在）
ensure_key() {
    if [ -f "$KEY_PATH" ]; then
        log "密钥已存在: $KEY_PATH"
    else
        log "生成新密钥..."
        ssh-keygen -t ed25519 -C "bench@ferrite" -f "$KEY_PATH" -N ""
        log "密钥已生成: $KEY_PATH"
    fi
}

# 获取服务器信息
get_server_info() {
    echo ""
    echo "=== 服务器信息 ==="
    read -rp "服务器 IP 或域名: " SERVER_HOST
    read -rp "SSH 用户名 [root]: " SERVER_USER
    SERVER_USER="${SERVER_USER:-root}"
    read -rp "SSH 端口 [22]: " SERVER_PORT
    SERVER_PORT="${SERVER_PORT:-22}"
    read -rsp "SSH 密码: " SERVER_PASS
    echo ""

    [ -z "$SERVER_HOST" ] && error "服务器地址不能为空"
    [ -z "$SERVER_PASS" ] && error "密码不能为空"
}

# 部署公钥到服务器
deploy_key() {
    echo ""
    echo "=== 部署公钥到服务器 ==="

    # 用 sshpass + ssh-copy-id 复制公钥
    log "复制公钥到 ${SERVER_USER}@${SERVER_HOST}:${SERVER_PORT}..."
    sshpass -p "$SERVER_PASS" ssh-copy-id \
        -i "$PUB_KEY_PATH" \
        -p "$SERVER_PORT" \
        -o StrictHostKeyChecking=no \
        -o UserKnownHostsFile=/dev/null \
        "${SERVER_USER}@${SERVER_HOST}" 2>&1 | tail -5

    if [ ${PIPESTATUS[0]} -eq 0 ]; then
        log "公钥部署成功"
    else
        error "公钥部署失败，请检查账号/密码/端口"
    fi
}

# 测试 SSH 连接
test_connection() {
    echo ""
    echo "=== 测试 SSH 连接 ==="
    ssh -i "$KEY_PATH" -p "$SERVER_PORT" -o StrictHostKeyChecking=no \
        -o UserKnownHostsFile=/dev/null \
        -o ConnectTimeout=10 \
        "${SERVER_USER}@${SERVER_HOST}" "echo 'SSH 连接成功!'; uname -a" 2>&1 | head -5
}

# 配置本地 SSH config
setup_ssh_config() {
    echo ""
    echo "=== 配置本地 SSH config ==="
    local config_file="$HOME/.ssh/config"

    # 检查是否已存在
    if grep -q "Host bench-server" "$config_file" 2>/dev/null; then
        warn "bench-server 配置已存在，跳过"
        return
    fi

    cat >> "$config_file" << EOF

# ferrite bench 服务器
Host bench-server
    HostName ${SERVER_HOST}
    User ${SERVER_USER}
    Port ${SERVER_PORT}
    IdentityFile ${KEY_PATH}
    IdentitiesOnly yes
    StrictHostKeyChecking no
    UserKnownHostsFile /dev/null
EOF

    chmod 600 "$config_file"
    log "SSH config 已更新: $config_file"
    log "以后可以用: ssh bench-server"
}

# 添加 GitHub secret
setup_github_secret() {
    echo ""
    echo "=== 添加 GitHub Secret ==="

    # 检查是否已存在
    if gh secret list -R "$GITHUB_REPO" 2>/dev/null | grep -q "$SECRET_NAME"; then
        warn "Secret $SECRET_NAME 已存在，是否覆盖？"
        read -rp "覆盖? [y/N]: " overwrite
        if [[ ! "$overwrite" =~ ^[Yy]$ ]]; then
            log "跳过 GitHub secret 设置"
            return
        fi
    fi

    # 添加私钥到 GitHub secrets
    gh secret set "$SECRET_NAME" -R "$GITHUB_REPO" < "$KEY_PATH"
    log "GitHub Secret 已设置: $SECRET_NAME"

    # 同时设置服务器信息
    gh secret set "BENCH_SERVER_HOST" -R "$GITHUB_REPO" <<< "$SERVER_HOST"
    gh secret set "BENCH_SERVER_USER" -R "$GITHUB_REPO" <<< "$SERVER_USER"
    gh secret set "BENCH_SERVER_PORT" -R "$GITHUB_REPO" <<< "$SERVER_PORT"
    log "GitHub Secrets 已设置: BENCH_SERVER_HOST, BENCH_SERVER_USER, BENCH_SERVER_PORT"
}

# 输出摘要
summary() {
    echo ""
    echo "========================================"
    echo "  部署完成！"
    echo "========================================"
    echo ""
    echo "  私钥: $KEY_PATH"
    echo "  公钥: $PUB_KEY_PATH"
    echo "  服务器: ${SERVER_USER}@${SERVER_HOST}:${SERVER_PORT}"
    echo ""
    echo "  GitHub Secrets:"
    echo "    - $SECRET_NAME"
    echo "    - BENCH_SERVER_HOST"
    echo "    - BENCH_SERVER_USER"
    echo "    - BENCH_SERVER_PORT"
    echo ""
    echo "  测试连接: ssh bench-server"
    echo "========================================"
}

# 主流程
main() {
    echo "=== Ferrite Bench SSH 密钥部署 ==="
    echo ""

    check_deps
    ensure_key
    get_server_info
    deploy_key
    test_connection
    setup_ssh_config
    setup_github_secret
    summary
}

main "$@"
