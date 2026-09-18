//! 设置 Tab 页面模块

pub mod account_section;
pub mod panel;
pub mod preferences_section;

pub use panel::SettingsPanel;
pub use account_section::AccountSection;
pub use preferences_section::PreferencesSection;