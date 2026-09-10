-- =====================================================================
-- 0003 模型单价：model_prices
-- ---------------------------------------------------------------------
-- 工作负载画像：读 = 启动快照一次；写 = 管理台编辑（低频）；行数几十。
-- 单位约定：input/output/cache 为 $/M tokens；内部结算 500_000 单位 = $1。
-- 设计取舍：不放 group_multiplier 列——分组倍率唯一来源是 api_groups.ratio，
--   结算时 cost × group.ratio；此前"配置文件 group_multiplier 恒 1.0、
--   api_groups.ratio 死字段"的双轨断裂由此收敛为单轨。
-- 对照：new-api 无价格表，单价塞 options KV 的 JSON 字符串，只能整段编辑；
--   本表按模型一行，管理台可做价格编辑页。
-- =====================================================================
CREATE TABLE IF NOT EXISTS model_prices (
    model       TEXT PRIMARY KEY,                    -- 公开别名，与 route unit 的 public_model 同口径
    input       DOUBLE PRECISION NOT NULL,           -- 输入单价 $/M tokens
    output      DOUBLE PRECISION NOT NULL,           -- 输出单价 $/M tokens
    cache       DOUBLE PRECISION NOT NULL DEFAULT 0, -- 缓存读单价 $/M tokens；0 = 不计缓存费
    remark      TEXT NOT NULL DEFAULT '',
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
