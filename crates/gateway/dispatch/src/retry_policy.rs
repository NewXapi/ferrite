//! P3 每模型重试档：api-hub `[retry."<model>"]` 配置面（_retry_policy :232-255
//! 的 merge 语义：default 先、model 覆盖后）。默认值健康化（max 3 非 100）。

use std::collections::HashMap;

use crate::retry::RetryPolicy;

/// 默认档 + 按模型精确覆盖档。
#[derive(Debug, Clone)]
pub struct ModelRetryPolicies {
    default_policy: RetryPolicy,
    per_model: HashMap<String, RetryPolicy>,
}

impl ModelRetryPolicies {
    /// 构造器。
    pub fn new(default_policy: RetryPolicy, per_model: HashMap<String, RetryPolicy>) -> Self {
        Self {
            default_policy,
            per_model,
        }
    }

    /// 精确命中 per_model，否则回 default。
    pub fn policy_for(&self, model: &str) -> &RetryPolicy {
        self.per_model.get(model).unwrap_or(&self.default_policy)
    }
}
