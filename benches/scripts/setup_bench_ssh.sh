#!/usr/bin/env bash
# benches/scripts/setup_bench_ssh.sh — 压测服务器 SSH 密钥一键配置
# 纯系统原生：使用 openssh 自带工具，不需要额外安装 sshpass 或任何依赖。

set -euo pipefail

KEY_PATH="$HOME/.ssh/id_bench"
PUB_KEY_PATH="$KEY_PATH.pub"
GITHUB_REPO="NewXapi/ferrite"

# 颜色
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log() { echo -e "${GREEN}[✓]${NC} $*"; }
warn() { echo -e "${YELLOW}[!]${NC} $*"; }
error() { echo -e "${RED}[✗]${NC} $*" >&2; exit 1; }

echo "========================================"
echo "   Ferrite Bench 服务器 SSH 一键配置"
echo "========================================"
echo ""

# 1. 检查基础工具
command -v ssh >/dev/null 2>&1 || error "系统未检测到 ssh 命令，请确保已安装 openssh"
command -v ssh-copy-id >/dev/null 2>&1 || error "系统未检测到 ssh-copy-id 命令"

# 2. 检查或生成专用密钥
if [ -f "$KEY_PATH" ]; then
    log "检测到已有压测专用密钥: $KEY_PATH"
else
    log "生成专用压测密钥 (ED25519)..."
    mkdir -p "$HOME/.ssh"
    chmod 700 "$HOME/.ssh"
    ssh-keygen -t ed25519 -C "bench@ferrite" -f "$KEY_PATH" -N ""
    log "密钥已生成: $KEY_PATH"
fi

PUB_KEY=$(cat "$PUB_KEY_PATH")

# 3. 收集服务器信息
echo ""
read -rp "请输入新服务器 IP 或域名: " SERVER_HOST
[ -z "$SERVER_HOST" ] && error "服务器地址不能为空"

read -rp "请输入 SSH 用户名 [默认: root]: " SERVER_USER
SERVER_USER="${SERVER_USER:-root}"

read -rp "请输入 SSH 端口 [默认: 22]: " SERVER_PORT
SERVER_PORT="${SERVER_PORT:-22}"

echo ""
log "目标服务器: ${SERVER_USER}@${SERVER_HOST}:${SERVER_PORT}"
log "接下来系统会提示你输入服务器的登录密码..."
echo ""

# 4. 使用官方标准的 ssh-copy-id 部署公钥
# 不经过任何 stdin 重定向，让 OpenSSH 正常读取终端输入密码
ssh-copy-id -i "$PUB_KEY_PATH" -p "$SERVER_PORT" -o StrictHostKeyChecking=no "${SERVER_USER}@${SERVER_HOST}" || {
    echo ""
    error "公钥部署未成功完成。请核对输入的密码是否正确，或者服务器是否开启了密码认证。"
}

# 5. 校验免密登录
echo ""
log "测试使用新私钥免密登录..."
ssh -i "$KEY_PATH" -p "$SERVER_PORT" -o StrictHostKeyChecking=no -o ConnectTimeout=10 \
    "${SERVER_USER}@${SERVER_HOST}" "echo -n '远程内核: ' && uname -sr && echo -n '系统架构: ' && uname -m" || {
    warn "免密测试未通过，请检查 authorized_keys 权限。"
}

# 6. 写入本地 ~/.ssh/config 别名
CONFIG_FILE="$HOME/.ssh/config"
touch "$CONFIG_FILE"
chmod 600 "$CONFIG_FILE"

# 如果已有旧配置先清理，保持别名最新
if grep -q "Host bench-server" "$CONFIG_FILE" 2>/dev/null; then
    # 临时文件处理
    sed -i '/# ferrite bench server/,+7d' "$CONFIG_FILE" 2>/dev/null || true
fi

cat >> "$CONFIG_FILE" << EOF

# ferrite bench server
Host bench-server
    HostName ${SERVER_HOST}
    User ${SERVER_USER}
    Port ${SERVER_PORT}
    IdentityFile ${KEY_PATH}
    IdentitiesOnly yes
    StrictHostKeyChecking no
EOF

log "本地快捷别名配置完成！以后你在终端直接输入: ssh bench-server 即可登录。"

# 7. 处理 GitHub Secrets（可选增强，绝不因未登录而阻塞）
echo ""
echo "=== 同步 GitHub Secrets（用于云端压测）==="
HAS_GH=false
if command -v gh >/dev/null 2>&1; then
    if gh auth status >/dev/null 2>&1; then
        HAS_GH=true
    fi
fi

if [ "$HAS_GH" = true ]; then
    log "检测到 gh 已登录，自动为你更新 GitHub Secrets..."
    gh secret set BENCH_SERVER_HOST -R "$GITHUB_REPO" <<< "$SERVER_HOST" >/dev/null 2>&1 || true
    gh secret set BENCH_SERVER_USER -R "$GITHUB_REPO" <<< "$SERVER_USER" >/dev/null 2>&1 || true
    gh secret set BENCH_SSH_PRIVATE_KEY -R "$GITHUB_REPO" < "$KEY_PATH" >/dev/null 2>&1 || true
    log "GitHub Secrets (HOST, USER, SSH_PRIVATE_KEY) 更新完毕！"
else
    warn "当前环境未登录 gh CLI，已自动跳过 Secrets 同步。"
    echo "  如果你之后需要云端 Actions 访问，可在 GitHub 仓库手动更新这几个 Secrets："
    echo "  - BENCH_SERVER_HOST: $SERVER_HOST"
    echo "  - BENCH_SERVER_USER: $SERVER_USER"
    echo "  - BENCH_SSH_PRIVATE_KEY: (内容为 ~/.ssh/id_bench)"
fi

echo ""
echo "========================================"
echo -e "${GREEN}  全部配置顺利完成！${NC}"
echo "  测试连接命令: ssh bench-server"
echo "========================================"
