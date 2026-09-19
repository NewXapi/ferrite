//! Account page: user's keys & profile, usage & logs, invite rewards & wallet,
//! plus login sessions and user settings.
//! 数据全部经 `api` 取用,面板不直接持有 mock。
//!
//! 一个 tab = 一个目录（目录名带 `tab-page-` 前缀，Rust 模块名不带连字符，
//! 故用 `#[path]` 声明）；目录内文件不带前缀，见各 `mod.rs` 的拆分约定。

pub mod api;
pub mod usage_support;

#[path = "tab-page-keys/mod.rs"]
pub mod tab_page_keys;
#[path = "tab-page-rewards/mod.rs"]
pub mod tab_page_rewards;
#[path = "tab-page-sessions/mod.rs"]
pub mod tab_page_sessions;
#[path = "tab-page-settings/mod.rs"]
pub mod tab_page_settings;
#[path = "tab-page-usage-logs/mod.rs"]
pub mod tab_page_usage_logs;

pub use tab_page_keys::KeysPanel;
pub use tab_page_rewards::RewardsPanel;
pub use tab_page_sessions::SessionsPanel;
pub use tab_page_settings::SettingsPanel;
pub use tab_page_usage_logs::UsageLogsPanel;
