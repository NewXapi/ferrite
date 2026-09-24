//! 用户管理 tab 的共享层:该 tab 全部用户可见文案常量。
//!
//! 按 spec §3.2 前缀分层:`LBL_` 标签 / `BTN_` 按钮 / `SEC_` 区段标题 /
//! `FIELD_` 表单字段 / `MSG_` 提示错误空态 / `OPT_` 下拉选项 /
//! `STATUS_` 状态徽标 / `TTL_` 弹窗标题 / `TAB_` 页签。
//! 常量值即原字面量,一个字符不改(H1 零渲染变化)。
//!
//! 边界:不放网络调用(在 `page.rs`)、不放组件(在各业务文件)。

// ---- BTN_* : 按钮文案 ----

/// 弹窗底部取消按钮文案。
pub const BTN_CANCEL: &str = "取消";
/// 新建用户按钮。
pub const BTN_NEW_USER: &str = "✚ 新建用户";
/// 筛选区刷新按钮。
pub const BTN_REFRESH: &str = "刷新";
/// 加载失败重试按钮。
pub const BTN_RETRY: &str = "重试";
/// 用户表单保存(编辑态)。
pub const BTN_SAVE: &str = "保存修改";
/// 用户表单提交(新建态)。
pub const BTN_CREATE: &str = "创建用户";

// ---- STATUS_* : 状态徽标 ----

// ---- LBL_* : 标签 / 表头 / 字段名 ----

/// 邮箱字段标签(表单输入框与绑定页只读行共用)。
pub const LBL_EMAIL: &str = "邮箱";
/// 额度字段标签(卡片进度条 / 表单输入框同口径)。
pub const LBL_QUOTA: &str = "额度";
/// 统计卡:总用户数。
pub const LBL_TOTAL_USERS: &str = "总用户";
/// 统计卡:启用中用户数。
pub const LBL_ENABLED_USERS: &str = "启用中";
/// 统计卡:本月新增数。
pub const LBL_NEW_THIS_MONTH: &str = "本月新增";
/// 统计卡:累计发放额度。
pub const LBL_GRANTED_TOTAL: &str = "总发放额度";
/// 统计卡:累计消耗额度。
pub const LBL_CONSUMED_TOTAL: &str = "总消耗";
/// 卡片计数行:请求数标签。
pub const LBL_REQUESTS: &str = "请求数";
/// 卡片计数行:创建时间标签。
pub const LBL_CREATED: &str = "创建";
/// 列表条数:人数单位。
pub const LBL_PERSON: &str = "人";

// ---- SEC_* : 区段标题 / 说明条 ----

/// 页面顶部统计区标题。
pub const SEC_STATS: &str = "用户概览";
/// 用户列表区标题。
pub const SEC_LIST: &str = "用户列表";
/// 筛选区标题。
pub const SEC_FILTER: &str = "筛选用户";

// ---- FIELD_* : 表单字段标签 ----

/// 用户名字段。
pub const FIELD_USERNAME: &str = "用户名";
/// 用户名输入框占位。
pub const FIELD_USERNAME_HINT: &str = "例如: zhangna";
/// 初始密码字段。
pub const FIELD_INIT_PASSWORD: &str = "初始密码";
/// 初始密码输入框占位。
pub const FIELD_PASSWORD_HINT: &str = "至少 8 位";
/// 角色权限字段。
pub const FIELD_ROLE: &str = "角色权限";
/// 生效分组字段。
pub const FIELD_GROUPS: &str = "生效分组";
/// 生效分组旁说明。
pub const FIELD_GROUP_HINT: &str = "点击分组切换选中,可多选;首个分组为计费生效分组。";
/// 管理员备注字段。
pub const FIELD_NOTE: &str = "管理员备注(仅管理员可见)";
/// 管理员备注输入框占位。
pub const FIELD_NOTE_HINT: &str = "例如: 连续 30 天无登录,待清退";

// ---- MSG_* : 提示 / 错误 / 空态 ----

/// 第三方绑定区说明条。
pub const MSG_BINDING_READONLY: &str = "第三方账号绑定(后端暂未返回,只读)";
/// 额度输入框旁说明(与卡片进度条同口径)。
pub const MSG_QUOTA_HINT: &str = "折合";
/// 列表搜索框占位。
pub const MSG_SEARCH_HINT: &str = "搜索用户名或邮箱";
/// 列表加载中。
pub const MSG_LOADING: &str = "加载中…";
/// 列表拉取失败。
pub const MSG_LOAD_USERS_FAIL: &str = "加载用户失败";
/// 列表加载态文案。
pub const MSG_LOADING_USERS: &str = "正在加载用户…";
/// 列表空态。
pub const MSG_NO_MATCH: &str = "没有匹配的用户";
/// 行内操作成功提示。
pub const MSG_ACTION_OK: &str = "操作成功";
/// 行内操作失败提示前缀(`:e` 拼接错误详情)。
pub const MSG_ACTION_ERR: &str = "操作失败";
/// 用户创建成功提示。
pub const MSG_USER_CREATED: &str = "用户已创建";
/// 竖条操作(启停)进行中提示(带动作名)。
pub const MSG_MANAGING: &str = "正在执行";
/// 竖条操作(启停)成功提示。
pub const MSG_MANAGED: &str = "操作成功";
/// 竖条操作(启停)失败提示(带原因)。
pub const MSG_MANAGE_ERR: &str = "操作失败";
/// 用户创建失败提示前缀。
pub const MSG_CREATE_ERR: &str = "创建失败";
/// 分组选择器空态。
pub const MSG_NO_GROUPS: &str = "暂无分组(后端 /api/group 为空)";

// ---- OPT_* : 下拉选项 ----

/// 筛选下拉:全部用户。
pub const OPT_ALL: &str = "全部";

// ---- TTL_* : 弹窗标题 ----

/// 用户编辑弹窗标题。
pub const TTL_EDIT_USER: &str = "编辑用户";
/// 用户新建弹窗标题。
pub const TTL_NEW_USER: &str = "新建用户";

// ---- TAB_* : 弹窗内页签 ----

/// 用户表单页签:基本信息。
pub const TAB_BASIC: &str = "基本信息";
/// 用户表单页签:分组与备注。
pub const TAB_GROUP: &str = "分组与备注";
/// 用户表单页签:绑定。
pub const TAB_BINDING: &str = "绑定";
