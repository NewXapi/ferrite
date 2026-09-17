-- =====================================================================
-- 0018 api_models 定价列 — 管理台别名页的每千 token 价格与倍率
-- ---------------------------------------------------------------------
-- 动因：管理台模型别名页的定价编辑（输入/输出单价、倍率）此前只是前端
-- 本地状态——contract 的 AliasUpsertRequest 已带这三字段，但后端
-- api_models 无对应列，PUT /api/models/{key} 请求体里它们被
-- UpdateModelRequest（未声明 + serde 未知字段默认忽略）读成 None 丢弃，
-- 保存不落库、新建按钮因此 blocked。本迁移把三列加上，models 域才能
-- 真的读写它们。
--
-- 设计取舍（不要与 model_prices 合并）：
-- - model_prices（0003）是**网关结算层**单价表：按 model 名关联、
--   $/M tokens 口径、给计费扣款用（启动快照读一次）；写由结算侧维护。
-- - 本迁移三列是**管理台展示/配置层**：随 api_models 行一起走 CRUD，
--   由运营在别名页直接编辑，只用于管理台展示与配置，不进结算链路。
--   两表口径不同（结算 vs 展示配置）、维护方与生命周期都不同：合并会让
--   结算单价被管理台误改，故分开，本列独立演化。
--
-- 幂等与存量数据：
-- - ADD COLUMN IF NOT EXISTS + NOT NULL DEFAULT：PG 对带默认值的
--   ADD COLUMN 不重写存量行（只在元数据层填默认），大表也瞬时完成；
--   api_models 行数十级，零风险。重跑迁移对已加过列的库安全。
-- - 默认值语义：input/output = 0（未定价）、multiplier = 1.0（无加价）。
--   应用层用 Option<f64> 表达「未提供」：create 缺席 resolve 到默认值；
--   update 缺席（None）走 COALESCE 保持现值，绝不置零。
--
-- 工作负载：随现有 api_models CRUD（读多写少，行数十级），不引入新索引
-- （不按定价列查询/排序，管理台列表按 created_at 排序）。
-- =====================================================================

ALTER TABLE api_models ADD COLUMN IF NOT EXISTS input_per_1k  DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE api_models ADD COLUMN IF NOT EXISTS output_per_1k DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE api_models ADD COLUMN IF NOT EXISTS multiplier   DOUBLE PRECISION NOT NULL DEFAULT 1.0;

-- 三列语义统一记录在表级注释（口径与上限约束见文件头设计取舍块）：
--   前两列 = 管理台展示用每 1k tokens 输入/输出单价，0 = 未定价；
--   第三列 = 展示用价格倍率，1.0 = 无加价，应用层上限 100.0 防运营手滑。
-- 不为每列单独 COMMENT ON COLUMN：三列同组同语义，表级注释 + 文件头说明更清晰。
COMMENT ON TABLE api_models IS '对外售卖的模型条目（管理台 CRUD）；0018 加三列展示用定价（每 1k tokens 输入/输出单价 + 倍率），与 model_prices 结算口径分离，不进结算链路';
