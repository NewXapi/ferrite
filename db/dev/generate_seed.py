#!/usr/bin/env python3
"""db/dev 种子生成器 — 真实数据纹理 + 可复现随机分布。

数据来源 (两层):
1. 真实层: db/dev/texture.json.gz — new-api 运行库 consume 日志的压缩纹理
   (真实模型名/token 量级/quota/耗时/流式比/昼夜节奏; 由外部库一次性提炼,
   重新提炼方法见 db/dev/README.md)。
2. 随机层: 种子化 RNG (固定 seed=42, 可复现) 做时间重映射与用户分配。

输出: stdout 打印幂等 seed.sql (不落盘, 管道直通 psql)。
- 时间戳全部用 now() - interval 相对表达式 → 任何时间执行, 今天/本周/
  本月/今年四个窗口都有数据, 永不过期。
- 清理标记: usage_logs.request_id LIKE 'seed-%'; 账号/渠道/令牌用固定 UUID。
"""

import gzip
import json
import random
import sys
from pathlib import Path

TEXTURE = Path(__file__).parent / "texture.json.gz"

SEED_MARK = "seed-"
YEAR_ROWS = 30_000
WEEK_ROWS = 8_000
DAY_ROWS = 6_000

# 固定 UUID — reset 与幂等重灌都靠它们定位 (纯 hex, UUID 只收 0-9a-f)
ADMIN_KEY = "00000000-0000-0000-0000-00000000aa01"
USER_KEYS = [f"00000000-0000-0000-0000-0000000000{i:02x}" for i in range(1, 11)]
GROUP_KEYS = {"default": "00000000-0000-0000-0000-00000000bb01", "vip": "00000000-0000-0000-0000-00000000bb02"}
CHANNEL_KEYS = [f"00000000-0000-0000-0000-00000000cc{i:02d}" for i in range(1, 5)]
TOKEN_KEYS = [f"00000000-0000-0000-0000-00000000dd{i:02d}" for i in range(1, 4)]

# dev 账号 (admin_dev / DevPassw0rd!12345) 的 argon2 PHC 哈希 — 仅 dev 种子
ADMIN_HASH = (
    "$argon2id$v=19$m=19456,t=2,p=1$99/iRJmcawny+IGRHpyd/Q$"
    "LClUaU5RAXfhsmdhYGuK6kEJPyMrUeqN3wAHZ8WXWoQ"
)
DEV_PASSWORD_NOTE = "admin_dev / DevPassw0rd!12345 (role=100 root)"

# 真实渠道名 (new-api.db channels 表, status=1 的前三个) + 一个演示上游
CHANNEL_NAMES = ["wt-52mxw", "wt-cao", "wt-factory", "OneAPI 上游"]
CHANNEL_BASES = [
    "https://api.openai.com/v1",
    "https://oneapi.example.internal/v1",
    "https://factory.example.internal/v1",
    "https://relay.example.internal/v1",
]
# 渠道探活可用率画像 (与真实探活抖动一致的梯度)
CHANNEL_AVAIL = [0.997, 0.972, 0.930, 0.985]
CHANNEL_LATENCY = [190, 410, 820, 350]

USERS = [
    ("hathaway", 24.0),
    ("alice_dev", 16.0),
    ("bob_ng", 12.0),
    ("charlie_q", 9.0),
    ("diana_w", 7.0),
    ("eve_gpt", 5.0),
    ("frank_lin", 3.0),
    ("grace_lee", 2.0),
    ("henry_gao", 1.2),
    ("ivan_p", 0.8),
]
USER_NAMES = [u for u, _ in USERS]
USER_WEIGHTS = [w for _, w in USERS]


def q(s: str) -> str:
    return "'" + s.replace("'", "''") + "'"


def load_texture(path: Path):
    """压缩纹理行: 与提炼时同构的 dict 列表 (模型/prompt/completion/quota/
    use_time_s/stream/channel_id/frac/hour)。"""
    with gzip.open(path, "rt", encoding="utf-8") as f:
        return json.load(f)


def ts_abs(day_int: int, hour: int, minute: int, sec: int) -> str:
    """绝对钟点时间戳: 今天 0 点回退 day_int 天, 再加当天钟点。
    与执行时刻解耦 — 同一钟点永远落在同一小时桶, 昼夜节奏可复现。"""
    return (
        f"date_trunc('day', now()) - interval '{day_int} days'"
        f" + interval '{hour} hours {minute} minutes {sec} seconds'"
    )


def pick_clock(r, rng):
    """真实小时 + 高斯抖动 (±~1h) → 保留昼夜节奏又填平稀疏桶。"""
    hour = int(round(r["hour"] + rng.gauss(0, 1.2))) % 24
    return hour, rng.randrange(60), rng.randrange(60)


def build_usage_layer(real, rng, count, day_base, day_span, idx0):
    """一层用量行: 真实纹理 × 窗口内均匀的天位置 × 真实昼夜钟点。

    天位置用均匀随机而非真实 frac — 真实窗口只有两周且高度成簇 (3 天占了
    全部流量), 直接映射会把簇砸进个别桶 (单桶撑爆/中间全空); 真实感保留在
    模型配比、token 量级与小时分布上。
    """
    out = []
    n_real = len(real)
    for i in range(count):
        r = real[(i * 7919 + idx0) % n_real]  # 质数步进, 避免按顺序循环
        day = int(day_base + rng.uniform(0, day_span))
        hour, minute, sec = pick_clock(r, rng)
        user = rng.choices(USER_NAMES, weights=USER_WEIGHTS, k=1)[0]
        ch_idx = (r["channel_id"] - 1) % len(CHANNEL_NAMES)
        prompt = max(r["prompt"] + rng.randint(-50, 50), 1)
        completion = max(r["completion"] + rng.randint(-80, 80), 1)
        use_ms = max(int(r["use_time_s"] * 1000 * rng.uniform(0.7, 1.4)), 100)
        quota = max(int(r["quota"] * rng.uniform(0.85, 1.15)), 0)
        out.append(
            "(2, {uk}, {u}, {tk}, {tn}, {ck}, {cn}, {m}, {p}, {c}, {q}, {ms}, {st}, '', {rid}, '', {ts})".format(
                uk=q(USER_KEYS[0] if user == "hathaway" else USER_KEYS[min(USER_NAMES.index(user), len(USER_KEYS) - 1)]),
                u=q(user),
                tk=q(TOKEN_KEYS[rng.randrange(len(TOKEN_KEYS))]),
                tn=q(rng.choice(["默认密钥", "测试密钥", "生产环境"])),
                ck=q(CHANNEL_KEYS[ch_idx]),
                cn=q(CHANNEL_NAMES[ch_idx]),
                m=q(r["model"]),
                p=prompt,
                c=completion,
                q=quota,
                ms=use_ms,
                st="true" if (r["stream"] if rng.random() > 0.1 else not r["stream"]) else "false",
                rid=q(f"{SEED_MARK}{idx0 + i}"),
                ts=ts_abs(day, hour, minute, sec),
            )
        )
    return out


def main():
    real = load_texture(TEXTURE)
    if not real:
        sys.exit(f"empty texture: {TEXTURE}")
    rng = random.Random(42)

    emit = sys.stdout.write
    emit("-- 自动生成: python3 db/dev/generate_seed.py — 不要手改, 改生成器后重新生成。\n")
    emit(f"-- 真实纹理来源: texture.json.gz ({len(real)} 条 consume) + seed=42 随机分布。\n")
    emit(f"-- dev 账号: {DEV_PASSWORD_NOTE}\n")
    emit("\\set ON_ERROR_STOP on\nBEGIN;\n\n")

    # ---- 清理旧种子 (幂等): UUID 与唯一名双条件, 兼容历史手工创建的同名行 ----
    emit("DELETE FROM usage_logs WHERE request_id LIKE 'seed-%';\n")
    emit(f"DELETE FROM monitor_history WHERE channel_key IN ({','.join(q(k) for k in CHANNEL_KEYS)});\n")
    emit(f"DELETE FROM api_tokens WHERE key IN ({','.join(q(k) for k in TOKEN_KEYS)});\n")
    emit(f"DELETE FROM api_channels WHERE key IN ({','.join(q(k) for k in CHANNEL_KEYS)}) OR name IN ({','.join(q(n) for n in CHANNEL_NAMES)});\n")
    emit(f"DELETE FROM api_groups WHERE key IN ({','.join(q(k) for k in GROUP_KEYS.values())}) OR name IN ('default', 'vip');\n")
    emit(f"DELETE FROM auth_users WHERE key IN ({','.join(q(k) for k in [ADMIN_KEY, *USER_KEYS])}) OR username IN ('admin_dev', {','.join(q(n) for n in USER_NAMES)});\n\n")

    # ---- 账号 ----
    emit(f"INSERT INTO auth_users (key, username, display_name, email, password_hash, role, status, quota, group_id) VALUES\n")
    emit(f"  ({q(ADMIN_KEY)}, 'admin_dev', 'Dev Admin', 'admin@dev.local', {q(ADMIN_HASH)}, 100, 1, 5000000000, 'default')")
    for i, name in enumerate(USER_NAMES):
        quota = int(USERS[i][1] * 200_000_000)
        emit(f",\n  ({q(USER_KEYS[min(i, len(USER_KEYS) - 1)])}, {q(name)}, {q(name)}, {q(f'{name}@dev.local')}, {q(ADMIN_HASH)}, 1, 1, {quota}, {q('vip' if i < 3 else 'default')})")
    emit(";\n\n")

    # ---- 分组 / 渠道 / 令牌 ----
    emit("INSERT INTO api_groups (key, name, ratio, model_whitelist, remark, status) VALUES\n")
    emit(f"  ({q(GROUP_KEYS['default'])}, 'default', 1.0, '[]', 'dev 种子: 基准分组', 1),\n")
    emit(f"  ({q(GROUP_KEYS['vip'])}, 'vip', 0.8, '[]', 'dev 种子: 优惠分组', 1);\n\n")
    emit("INSERT INTO api_channels (key, name, channel_type, base_url, keys, models, groups, priority, weight, status, tags, settings, remark) VALUES\n")
    for i, (name, base) in enumerate(zip(CHANNEL_NAMES, CHANNEL_BASES)):
        models = '["gpt-5.6-sol","claude-fable-5","kimi-k3","glm-5.3-flash","deepseek-v4"]' if i < 3 else '["gpt-5.6-sol","kimi-k3"]'
        emit(f"  ({q(CHANNEL_KEYS[i])}, {q(name)}, 'openai', {q(base)}, '[]', {q(models)}, ARRAY['default'], {10 - i}, {10 - i}, 1, '[]', '{{}}', 'dev 种子'){',' if i < len(CHANNEL_NAMES) - 1 else ';\n'}\n")
    emit("\nINSERT INTO api_tokens (key, user_key, name, key_hash, key_preview, group_id, quota, unlimited_quota, used_quota, status) VALUES\n")
    tok_names = ["默认密钥", "测试密钥", "生产环境"]
    for i, (k, n) in enumerate(zip(TOKEN_KEYS, tok_names)):
        # key_hash: 64 位纯 hex 占位 — 快照加载会 hex 解码, 非法字符会炸启动
        h = f"{i + 1:064x}"
        emit(f"  ({q(k)}, {q(ADMIN_KEY)}, {q(n)}, {q(h)}, {q(f'sk-seed…{i}')}, 'default', 5000000000, false, 0, 1){',' if i < 2 else ';\n'}\n")

    # ---- 渠道探活 (近 7 天, 每渠道 ~320 条) ----
    mon_header = "INSERT INTO monitor_history (channel_key, channel_name, model, ok, status_code, latency_ms, error_kind, message, created_at) VALUES\n"
    emit("\n" + mon_header)
    mon_rows = []
    for ci, ck in enumerate(CHANNEL_KEYS):
        for j in range(320):
            day = int(rng.uniform(0, 7))
            ok = rng.random() < CHANNEL_AVAIL[ci]
            lat = int(rng.gauss(CHANNEL_LATENCY[ci], CHANNEL_LATENCY[ci] * 0.25))
            hour, minute, sec = rng.randrange(24), rng.randrange(60), rng.randrange(60)
            mon_rows.append(
                "({ck}, {cn}, 'gpt-5.6-sol', {ok}, {sc}, {lat}, {ek}, {msg}, {ts})".format(
                    ck=q(ck), cn=q(CHANNEL_NAMES[ci]),
                    ok="true" if ok else "false",
                    sc=200 if ok else rng.choice([429, 502, 503]),
                    lat=max(lat, 20),
                    ek=q("" if ok else rng.choice(["http", "timeout", "connect"])),
                    msg=q("probe" if ok else "seed failure"),
                    ts=ts_abs(day, hour, minute, sec),
                )
            )
    for i in range(0, len(mon_rows), 500):
        chunk = mon_rows[i : i + 500]
        emit(",\n".join(chunk) + ";\n")
        if i + 500 < len(mon_rows):
            emit(mon_header)
    emit("\n")

    # ---- 用量: 三层时间窗 (天位置均匀, 钟点用真实小时分布) ----
    layers = [
        (YEAR_ROWS, 7, 357.0, 0),        # 全年: 7~364 天前
        (WEEK_ROWS, 0, 6.4, YEAR_ROWS),  # 近 7 天加密
        (DAY_ROWS, 0, 0.0, YEAR_ROWS + WEEK_ROWS),  # 今天 (真实小时分布)
    ]
    emit("INSERT INTO usage_logs (log_type, user_key, username, token_key, token_name, channel_key, channel_name, model_name, prompt_tokens, completion_tokens, quota, use_time_ms, is_stream, ip, request_id, content, created_at) VALUES\n")
    all_rows = []
    for count, day_base, day_span, idx0 in layers:
        all_rows.extend(build_usage_layer(real, rng, count, day_base, day_span, idx0))
    for i in range(0, len(all_rows), 500):
        chunk = all_rows[i : i + 500]
        emit(",\n".join(chunk) + ";\n")
        if i + 500 < len(all_rows):
            emit("INSERT INTO usage_logs (log_type, user_key, username, token_key, token_name, channel_key, channel_name, model_name, prompt_tokens, completion_tokens, quota, use_time_ms, is_stream, ip, request_id, content, created_at) VALUES\n")

    emit("\nCOMMIT;\n")
    emit(f"-- 总行数: usage_logs={YEAR_ROWS + WEEK_ROWS + DAY_ROWS}, monitor={len(mon_rows)}\n")


if __name__ == "__main__":
    main()
