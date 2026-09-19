//! Account page: user's keys & profile, usage & logs, invite rewards & wallet,
//! plus login sessions and user settings.
//! 数据全部经 `api` 取用,面板不直接持有 mock。

pub mod api;
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
pub mod usage_support;

pub use tab_page_keys::KeysPanel;
pub use tab_page_rewards::RewardsPanel;
pub use tab_page_sessions::SessionsPanel;
pub use tab_page_settings::SettingsPanel;
pub use tab_page_usage_logs::UsageLogsPanel;
