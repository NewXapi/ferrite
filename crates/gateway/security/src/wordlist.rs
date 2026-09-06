//! `wordlist` —— 过滤词配置 (从 security.toml 的 [security] 段反序列化)
//!
//! 包含词列表、替换文本以及请求/响应侧开关。

use serde::{Deserialize, Serialize};

/// 过滤词配置 (从 config.toml 的 [security] 段反序列化)
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FilterConfig {
    /// 过滤词列表；空 = 不过滤
    #[serde(default)]
    pub words: Vec<String>,
    /// 命中时的替换文本，默认 "***"
    #[serde(default = "default_replacement")]
    pub replacement: String,
    /// 是否过滤请求（输入）
    #[serde(default = "default_true")]
    pub filter_request: bool,
    /// 是否过滤响应（输出，含流式）
    #[serde(default = "default_true")]
    pub filter_response: bool,
}

fn default_replacement() -> String {
    "***".to_string()
}

fn default_true() -> bool {
    true
}

impl FilterConfig {
    /// 验证配置；如果 words 为空则跳过构建过滤器
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}
