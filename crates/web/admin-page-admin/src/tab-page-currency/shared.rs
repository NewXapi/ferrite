//! 货币管理共享类型与文案。page / list / form 三处复用。

/// `kind` 语义对齐 0014 换算层：
/// - `Points` = 可扣费余额货币
/// - `Fiat` = 仅计价展示（不进余额）
#[derive(Clone, PartialEq)]
pub enum Kind {
    Points,
    Fiat,
}

impl Kind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Points => "points",
            Kind::Fiat => "fiat",
        }
    }
    pub fn parse(s: &str) -> Self {
        if s == "fiat" {
            Kind::Fiat
        } else {
            Kind::Points
        }
    }
}

pub const SEC_LIST: &str = "货币列表";
pub const SEC_FORM: &str = "新增 / 编辑货币";
