//! 带行号的配置错误。
//!
//! 校验失败必须**拒绝启动**并指出具体哪一行错，不要带着半个坏配置跑起来。
//! 见 docs/08-mvp.md §1.1。

use std::path::PathBuf;

/// 配置加载与校验的失败原因。
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("读取配置文件 {path} 失败: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// TOML 语法错误。`toml` crate 自带位置信息，直接透传给用户。
    #[error("配置语法错误: {0}")]
    Syntax(#[from] toml::de::Error),

    /// 语义校验失败。`at` 用于定位到具体条目，如 `channel[2].base_url`。
    #[error("配置校验失败于 {at}: {reason}")]
    Invalid { at: String, reason: String },

    /// `api_key = "env:FOO"` 但环境变量 FOO 不存在。
    #[error("配置 {at} 引用了未设置的环境变量 {var}")]
    MissingEnv { at: String, var: String },

    /// 渠道名重复 —— 名字是日志与 `gateway test` 的标识，必须唯一。
    #[error("渠道名重复: {name}")]
    DuplicateChannel { name: String },

    /// 一个渠道都没有可用的，起来也没意义。
    #[error("没有任何启用的渠道")]
    NoEnabledChannel,
}

impl ConfigError {
    /// 构造语义错误的便捷方法。
    pub fn invalid(at: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Invalid {
            at: at.into(),
            reason: reason.into(),
        }
    }
}
