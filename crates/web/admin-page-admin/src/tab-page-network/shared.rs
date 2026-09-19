//! 网络拓扑页共享文案常量。
//!
//! 本文件同时承载该 tab 的全部用户可见文案常量（i18n 第 1 层）：
//! 常量值即原字面量，逐字符保持一致以保证零渲染变化。
//!
//! 边界:不放类型与纯函数(在 `data.rs`)、不放组件(`#[component]` 在
//! `ui.rs` / `drawer.rs` / `inspector.rs` 里)、不放网络调用(在 `data.rs`
//! 与 `crate::drawer_write`)。这里只有文案常量。

// ---- LBL_* : 标签 / 表头 / 字段旁的说明 ----

/// 节点层名:分组。
pub const LBL_GROUP: &str = "分组";
/// 节点层名:模型别名。
pub const LBL_ALIAS: &str = "模型别名";
/// 节点层名:调度模型。
pub const LBL_DISPATCH: &str = "调度模型";
/// 抽屉页签:节点。
pub const LBL_NODES: &str = "节点";
/// 设置抽屉滚动导航里的实体卡名:渠道。
pub const LBL_CHANNELS: &str = "渠道";
/// 导入抽屉头副标题。
pub const LBL_IMPORT_SUBTITLE: &str = "把 JSON 包进来，一个渠道一个";
/// 导入表单:渠道名输入框标签。
pub const LBL_CHANNEL_NAME_OPT: &str = "渠道名（可选）";
/// 导入表单:Base URL 输入框标签。
pub const LBL_BASE_URL: &str = "Base URL";
/// 导入表单:API Key 输入框标签。
pub const LBL_API_KEY_MULTI: &str = "API Key（多 key 换行）";

// ---- BTN_* : 按钮文案 ----

/// HUD / 抽屉页签:导入。
pub const BTN_IMPORT: &str = "导入";
/// HUD / 抽屉页签:设置。
pub const BTN_SETTINGS: &str = "设置";
/// HUD:适配(缩放平移以框住全部可见节点)。
pub const BTN_FIT: &str = "适配";
/// 抽屉头关闭按钮的悬停提示。
pub const BTN_CLOSE_TITLE: &str = "关闭";
/// 检视器底部删除按钮。
pub const BTN_DELETE: &str = "删除";
/// 检视器底部保存按钮。
pub const BTN_SAVE: &str = "保存";
/// 分组检视:保存展示名按钮。
pub const BTN_SAVE_DISPLAY: &str = "保存展示名";
/// 渠道检视:保存渠道按钮。
pub const BTN_SAVE_CHANNEL: &str = "保存渠道";
/// 导入表单:提交按钮。
pub const BTN_IMPORT_CHANNEL: &str = "导入渠道";
/// 检视器删除按钮的悬停提示:分组。
pub const BTN_DELETE_GROUP_TITLE: &str = "删除分组";
/// 检视器删除按钮的悬停提示:渠道。
pub const BTN_DELETE_CHANNEL_TITLE: &str = "删除渠道";

// ---- SEC_* : 区段标题 / 说明条 ----

/// 设置抽屉:调度数据拉取失败时的顶部提示。
pub const SEC_SETTINGS_STALE: &str = "调度数据拉取失败,设置页显示的是本地缓存";
/// 焦点空间模式下的操作提示。
pub const SEC_HINT_FOCUS: &str = "焦点空间 · 左键节点切换 · 右键空白或再点同节点返回";
/// 连线拖拽中的操作提示。
pub const SEC_HINT_WIRING: &str = "拖到相邻层节点松开连线";
/// 节点拖拽中的操作提示。
pub const SEC_HINT_MOVING: &str = "松开落位";
/// 常态操作提示。
pub const SEC_HINT_IDLE: &str =
    "滚轮缩放 · 拖空白平移 · Shift拖空白框选 · Ctrl点选多个 · 拖节点摆位 · 拖圆点连线";

// ---- FIELD_* : 表单字段标签 ----

/// 分组检视:展示名输入框标签。
pub const FIELD_DISPLAY: &str = "展示名";
/// 导入表单/渠道检视:渠道名输入框占位示例。
pub const EXAMPLE_CHANNEL: &str = "OpenAI 官方";

// ---- MSG_* : 提示 / 错误 / 空态 / 占位 ----

/// 拓扑数据拉取:分组端点失败前缀(后接错误详情)。
pub const MSG_LOAD_GROUPS_FAILED: &str = "拉取分组失败: ";
/// 拓扑数据拉取:渠道端点失败前缀(后接错误详情)。
pub const MSG_LOAD_CHANNELS_FAILED: &str = "拉取渠道失败: ";
/// 拓扑数据拉取:模型端点失败前缀(后接错误详情)。
pub const MSG_LOAD_MODELS_FAILED: &str = "拉取模型失败: ";
/// 画布加载态 aria 标签。
pub const MSG_LOADING_ARIA: &str = "正在加载调度数据";
/// 画布加载态占位。
pub const MSG_LOADING: &str = "正在加载调度数据…";
/// 画布空态 aria 标签 / 占位。
pub const MSG_EMPTY: &str = "暂无调度数据";
/// 已提交连线的右键菜单项:删除连线。
pub const MSG_WIRE_DELETE_HINT: &str = "右键删除连线";
/// 导入表单:API Key 为空时的提示。
pub const MSG_IMPORT_KEY_REQUIRED: &str = "API Key 不能为空";
/// 导入表单:渠道名留空时的默认名。
pub const MSG_DEFAULT_CHANNEL_NAME: &str = "新渠道";
/// 导入表单:导入失败前缀(后接错误详情)。
pub const MSG_IMPORT_FAILED_PREFIX: &str = "导入失败：";
/// 写操作失败前缀(后接错误详情)。共享于删除/保存失败三类消息。
pub const MSG_WRITE_FAILED_PREFIX: &str = "删除失败：";
/// 保存操作失败前缀(后接错误详情)。
pub const MSG_SAVE_FAILED_PREFIX: &str = "保存失败：";
/// 分组定位失败提示前缀(后接分组名)。
pub const MSG_GROUP_MISSING_PREFIX: &str = "分组「";
/// 渠道定位失败提示前缀(后接渠道名)。
pub const MSG_CHANNEL_MISSING_PREFIX: &str = "渠道「";
/// 定位失败提示后缀(后接关闭引号前的说明)。
pub const MSG_MISSING_SUFFIX: &str = "」不存在于后端（可能已被删除）";
/// 分组检视:该分组不存在。
pub const MSG_GROUP_ABSENT: &str = "该分组不存在";
/// 别名检视:该别名不存在。
pub const MSG_ALIAS_ABSENT: &str = "该别名不存在";
/// 检视器删除按钮在不可删时的悬停提示。
pub const MSG_DELETE_DISABLED_TITLE: &str = "别名无删除端点（锁读）";
/// 检视器保存按钮在有权限时的悬停提示。
pub const MSG_SAVE_TITLE: &str = "保存按钮在各编辑卡片内（展示名/渠道）";
/// 检视器保存按钮在无权限时的悬停提示。
pub const MSG_SAVE_DISABLED_TITLE: &str = "别名锁读，无写路径";
/// 删除确认弹窗正文。
pub const MSG_DELETE_CONFIRM: &str = "确认删除？该操作直接生效于后端，不可撤销。";
/// 分组检视:分组名锁读标签。
pub const MSG_GROUP_NAME_LOCKED: &str = "分组名（锁读：后端无改名路径）";
/// 分组检视:展示名输入框占位。
pub const MSG_PH_DEFAULT_GROUP: &str = "默认分组";
/// 分组检视:包含的模型别名列表标题。
pub const MSG_INSPECT_ALIASES_TITLE: &str = "包含的模型别名";
/// 分组检视:别名列表空态。
pub const MSG_INSPECT_ALIASES_EMPTY: &str = "拖端口连线以加入别名";
/// 别名检视:别名锁读标签。
pub const MSG_ALIAS_LOCKED: &str = "别名（锁读：写路径按 UUID key，此处定位不到）";
/// 别名检视:展示名锁读标签。
pub const MSG_ALIAS_DISPLAY_LOCKED: &str = "展示名（锁读：后端 models 域无对应列）";
/// 别名检视:所属分组列表标题。
pub const MSG_INSPECT_GROUPS_TITLE: &str = "所属分组";
/// 别名检视:所属分组列表空态。
pub const MSG_INSPECT_GROUPS_EMPTY: &str = "未加入任何分组";
/// 别名检视:路由到的调度模型列表标题。
pub const MSG_INSPECT_DISPATCH_TITLE: &str = "路由到的调度模型";
/// 别名检视:调度模型列表空态。
pub const MSG_INSPECT_DISPATCH_EMPTY: &str = "未连接调度模型";
/// 调度检视:模型名只读标签。
pub const MSG_MODEL_NAME_READONLY: &str = "模型名（只读，来自上游）";
/// 调度检视:所属渠道区标题。
pub const MSG_OWNER_CHANNEL: &str = "所属渠道";
/// 调度检视:渠道名称输入框标签。
pub const MSG_CHANNEL_NAME: &str = "渠道名称";
/// 调度检视:API Key 输入框标签。
pub const MSG_API_KEY_EDIT: &str = "API Key（多 key 一行一个；留空 = 保留现有密钥）";
/// 调度检视:被哪些别名路由列表标题。
pub const MSG_INSPECT_ROUTED_TITLE: &str = "被哪些别名路由";
/// 调度检视:被哪些别名路由列表空态。
pub const MSG_INSPECT_ROUTED_EMPTY: &str = "未被任何别名引用";
