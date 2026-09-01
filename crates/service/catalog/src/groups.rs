//! 分组管理 — 白名单与倍率。

use store::StoreError;

/// 分组校验:
/// - id 非空且唯一 (创建时); "default" 组不可删除 (wildtoken 规则, 防孤儿);
/// - rate_multiplier > 0;
/// - allowed_models 里每个模型建议在 model_meta 有条目 (软校验, 告警不阻断)。
pub fn validate_group(_g: &contract::records::GroupRecord) -> Result<(), StoreError> {
    todo!("TODO(#426): 校验 + default 组保护")
}
