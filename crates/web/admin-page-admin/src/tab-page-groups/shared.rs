//! 分组管理共享类型与文案常量。page / toolbar / list / modal 四处复用。

/// 弹窗状态
#[derive(Clone, PartialEq)]
pub enum ModalState {
    Closed,
    New,
    Edit(String),
}

/// 写操作种类(目前仅删除;工厂保留扩展位)
#[derive(Clone, Copy)]
pub enum WriteOp {
    Delete,
    /// 启用/停用切换 (status: 1=启用, 2=停用)
    ToggleStatus(i16),
    /// 倍率滑条拖动写回 (ratio)
    SetRatio(f64),
}

pub const SEC_STATS: &str = "分组概览";
pub const SEC_FILTER: &str = "筛选与操作";
pub const SEC_LIST: &str = "分组列表";

/// 把用户输入的逗号/分号分隔白名单拆成模型名数组(去空、trim)。
/// 与后端 `validate_whitelist` 对齐:每项必须是非空字符串。
pub fn parse_whitelist_raw(raw: &str) -> Vec<String> {
    raw.split([',', '，', ';', '；'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

/// 从后端返回的 `model_whitelist` JSON(字符串数组或空)取回白名单。
pub fn parse_whitelist(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| m.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}
