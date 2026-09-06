#!/usr/bin/env bash
# scripts/deploy_bench_key.sh — 一键部署公钥到服务器（不需要 sshpass）
# 用法：./deploy_bench_key.sh

set -euo pipefail

KEY_PATH="$HOME/.ssh/id_bench"
PUB_KEY_PATH="$KEY_PATH.pub"

log() { echo "[✓] $*"; }
error() { echo "[✗] $*" >&2; exit 1; }

command -v ssh >/dev/null 2>&1 || error "缺少 ssh"

# 生成密钥（如果不存在）
if [ ! -f "$KEY_PATH" ]; then
    log "生成新密钥..."
    ssh-keygen -t ed25519 -C "bench@ferrite" -f "$KEY_PATH" -N ""
fi
log "密钥就绪: $KEY_PATH"

# 输入服务器信息
echo ""
read -rp "服务器 IP 或域名: " SERVER_HOST
read -rp "SSH 用户名 [root]: " SERVER_USER
SERVER_USER="${SERVER_USER:-root}"

[ -z "$SERVER_HOST" ] && error "服务器地址不能为空"

# 部署公钥（直接 ssh，交互式输密码，不重定向 stdin）
log "部署公钥到 ${SERVER_USER}@${SERVER_HOST}..."
PUB_KEY=$(cat "$PUB_KEY_PATH")

ssh -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null \
    "${SERVER_USER}@${SERVER_HOST}" \
    "mkdir -p ~/.ssh && chmod 700 ~/.ssh && echo '$PUB_KEY' >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys"

log "公钥部署成功"

# 测试连接
log "测试 SSH 连接..."
ssh -i "$KEY_PATH" -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null \
    -o ConnectTimeout=10 \
    "${SERVER_USER}@${SERVER_HOST}" "echo '连接成功!'; uname -a" 2>&1 | head -3

# 配置 SSH config
CONFIG="$HOME/.ssh/config"
if ! grep -q "Host bench-server" "$CONFIG" 2>/dev/null; then
    cat >> "$CONFIG" << EOF

Host bench-server
    HostName ${SERVER_HOST}
    User ${SERVER_USER}
    IdentityFile ${KEY_PATH}
    IdentitiesOnly yes
    StrictHostKeyChecking no
    UserKnownHostsFile /dev/null
EOF
    chmod 600 "$CONFIG"
    log "SSH config 已添加: ssh bench-server"
fi

echo ""
echo "========================================"
echo "  部署完成！"
echo "  服务器: ${SERVER_USER}@${SERVER_HOST}"
echo "  测试: ssh bench-server"
echo "========================================"
