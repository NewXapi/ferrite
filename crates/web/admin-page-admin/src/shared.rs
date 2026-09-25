//! Admin page 共享文案常量与格式化助手。
//!
//! 根级 `shared.rs` 承载跨 tab 复用的用户可见文案 (i18n 第 1 层)。
//! 常量值即原字面量，逐字符保持一致以保证零渲染变化。
//! 只有各 tab 自己的文案放在对应组件文件顶部或 `components/` 内部;
//! 根级只放真正跨 tab 复用的常量。

// ============ 跨 Tab 复用常量 ============

/// 全部档位胶囊。
pub const OPT_ALL: &str = "全部";
/// 刷新按钮。
pub const BTN_REFRESH: &str = "刷新";
/// 重试按钮。
pub const BTN_RETRY: &str = "重试";
/// 取消按钮。
pub const BTN_CANCEL: &str = "取消";
/// 加载中徽标。
pub const OPT_BADGE_LOADING: &str = "加载中…";
/// 加载中占位。
pub const MSG_LOADING: &str = "加载中…";
/// 加载中 ARIA 文案。
pub const MSG_LOADING_ARIA: &str = "正在加载数据";
/// 加载失败标题。
pub const MSG_LOAD_FAILED: &str = "加载失败";
/// 空态占位。
pub const MSG_EMPTY: &str = "暂无数据";
/// 操作成功提示。
pub const MSG_OP_OK: &str = "操作成功";
/// 操作失败前缀。
pub const MSG_OP_FAILED: &str = "操作失败:";
/// 保存失败前缀。
pub const MSG_SAVE_FAILED: &str = "保存失败:";
/// 删除失败前缀。
pub const MSG_DELETE_FAILED: &str = "删除失败:";
/// 名称必填提示。
pub const MSG_NAME_REQUIRED: &str = "名称不能为空";
/// 数值无效提示。
pub const MSG_NUM_INVALID: &str = "数值无效,已保留原值";

// ============ Aliases tab 专用常量 (从 tab-page-aliases/shared.rs 迁移) ============

/// 别名概览统计区标题。
pub const SEC_STATS: &str = "别名概览";
/// 筛选与操作区标题。
pub const SEC_FILTER: &str = "筛选与操作";
/// 别名列表区标题。
pub const SEC_LIST: &str = "别名列表";
/// 筛选区说明。
pub const SEC_FILTER_NOTE: &str = "按倍率与资费规则快速筛选";
/// 数据来源说明条。
pub const SEC_DATA_NOTE: &str = "别名来自真实 /api/models;编辑与删除已接后端;定价模式与价格配置为 UI 层本地状态,后端扩展 pricing 列前保存不写库;新建暂未开放(后端需要 owner/api_key 字段)";
/// 定价模式补充说明。
pub const SEC_MODE_NOTE: &str = "切换到按量定价后,补充通道的启用开关在「按量定价」tab";
/// 按次定价说明。
pub const SEC_PER_CALL_NOTE: &str = "后端落地前按次价格暂存于倍率字段,仅 UI 层生效。";

/// 新建别名弹窗标题。
pub const TTL_NEW: &str = "新建模型别名";

/// 弹窗基本页签。
pub const TAB_BASIC: &str = "基本";
/// 弹窗按量定价页签。
pub const TAB_PER_TOKEN: &str = "按量定价";
/// 弹窗按次定价页签。
pub const TAB_PER_CALL: &str = "按次定价";

/// 别名标识字段标签。
pub const FIELD_ALIAS_ID: &str = "别名标识 (API 请求匹配名)";
/// 展示名称字段标签。
pub const FIELD_DISPLAY: &str = "展示名称 (可选)";
/// 计费倍率字段标签。
pub const FIELD_MULTIPLIER: &str = "计费倍率 (multiplier ≥ 0)";
/// 定价模式选择器标签。
pub const FIELD_PRICE_MODE: &str = "定价模式 (启用哪种定价)";
/// 输入价格字段标签。
pub const FIELD_INPUT_PRICE: &str = "输入价格";
/// 单次调用价格字段标签。
pub const FIELD_PER_CALL_PRICE: &str = "单次调用价格";

/// 概览卡:总别名数。
pub const LBL_STAT_TOTAL: &str = "总别名数";
/// 概览卡:标准倍率别名数。
pub const LBL_STAT_STANDARD: &str = "标准 1.0× 别名";
/// 概览卡:自定倍率别名数。
pub const LBL_STAT_CUSTOM: &str = "自定倍率别名";
/// 概览卡:免费别名数。
pub const LBL_STAT_FREE: &str = "免费别名 (0×)";
/// 概览卡:平均加价倍率。
pub const LBL_STAT_AVG: &str = "平均加价倍率";
/// 卡片网格 aria-label。
pub const LBL_ALIAS_LIST: &str = "别名列表";
/// 弹窗页签栏 aria-label。
pub const LBL_MODAL_TABLIST: &str = "别名编辑选项";
/// 输入价格说明。
pub const LBL_INPUT_PRICE_DESC: &str = "每 100 万输入 token 的价格。";
/// 单次调用价格说明。
pub const LBL_PER_CALL_DESC: &str = "每次调用(不论 token 数)固定扣费。";
/// 补充通道:输出价格。
pub const LBL_CH_OUTPUT: &str = "输出价格";
/// 补充通道:缓存读取价格。
pub const LBL_CH_CACHE_READ: &str = "缓存读取价格";
/// 补充通道:缓存写入价格。
pub const LBL_CH_CACHE_WRITE: &str = "缓存写入价格";
/// 补充通道:补全价格。
pub const LBL_CH_COMPLETION: &str = "补全价格";
/// 输出价格悬停说明。
pub const LBL_CH_OUTPUT_DESC: &str = "生成内容的输出 token 价格(悬停标题查看)";
/// 缓存读取悬停说明。
pub const LBL_CH_CACHE_READ_DESC: &str = "缓存读取 token 价格(悬停标题查看)";
/// 缓存写入悬停说明。
pub const LBL_CH_CACHE_WRITE_DESC: &str = "缓存写入 token 价格(悬停标题查看)";
/// 补全价格悬停说明。
pub const LBL_CH_COMPLETION_DESC: &str = "补全(输出)调用的 token 价格(悬停标题查看)";

/// 标准 1.0× 档位。
pub const OPT_STANDARD: &str = "标准 1.0×";
/// 自定倍率档位。
pub const OPT_CUSTOM: &str = "自定倍率";
/// 免费通道档位。
pub const OPT_FREE: &str = "免费通道";

/// 新建别名按钮。
pub const BTN_NEW_ALIAS: &str = "✚ 新建别名";
/// 创建别名按钮。
pub const BTN_CREATE_ALIAS: &str = "创建别名";

/// 别名标识占位。
pub const MSG_PH_ALIAS_ID: &str = "例如: gpt-4o, claude-3-5-sonnet";
/// 展示名称占位。
pub const MSG_PH_DISPLAY: &str = "例如: GPT-4o 旗舰模型";
/// 单次价格占位。
pub const MSG_PH_PER_CALL: &str = "例如: 0.05";
/// 搜索框占位。
pub const MSG_SEARCH_PLACEHOLDER: &str = "搜索别名 ID 或展示名称 (如 gpt-4o, claude-sonnet)...";
/// 删除成功提示。
pub const MSG_DELETED: &str = "已删除";
/// 新建被诚实拒绝提示。
pub const MSG_CREATE_REJECTED: &str =
    "新建未执行:后端创建模型需要 owner 与 api_key 字段,当前表单未提供";