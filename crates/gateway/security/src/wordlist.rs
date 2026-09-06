//! `wordlist` —— 过滤词配置（`config.toml` 的 `[security]` 段）

use serde::Deserialize;

/// 过滤词配置。
///
/// `Default` 与 serde 默认值走同一组函数：两条路径给不同默认值会让
/// `FilterConfig::default()` 和空 `[security]` 段行为分叉。
#[derive(Debug, Clone, Deserialize)]
pub struct FilterConfig {
    /// 过滤词列表；空 = 不过滤。
    #[serde(default)]
    pub words: Vec<String>,
    /// 命中时的替换文本。
    #[serde(default = "default_replacement")]
    pub replacement: String,
    /// 是否过滤请求（输入）。
    #[serde(default = "default_true")]
    pub filter_request: bool,
    /// 是否过滤响应（输出，含流式）。
    #[serde(default = "default_true")]
    pub filter_response: bool,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            words: Vec::new(),
            replacement: default_replacement(),
            filter_request: default_true(),
            filter_response: default_true(),
        }
    }
}

fn default_replacement() -> String {
    "***".to_string()
}

fn default_true() -> bool {
    true
}
