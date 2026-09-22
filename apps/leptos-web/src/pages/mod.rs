//! dioxus admin 页的 leptos 移植。每个文件一个页面组件。

pub mod aliases;
pub mod channels;
pub mod currency;
pub mod gateway;
pub mod groups;
pub mod keys;
pub mod leaderboard;
pub mod models;
pub mod network;
pub mod overview;
pub mod redemptions;
pub mod rewards;
pub mod sessions;
pub mod settings;
pub mod subscriptions;
pub mod system;
pub mod usage;

pub use aliases::AliasesPage;
pub use channels::ChannelsPage;
pub use currency::CurrencyPage;
pub use gateway::GatewayPage;
pub use groups::GroupsPage;
pub use keys::KeysPage;
pub use leaderboard::LeaderboardPage;
pub use models::ModelsPage;
pub use network::NetworkPage;
pub use overview::OverviewPage;
pub use redemptions::RedemptionsPage;
pub use rewards::RewardsPage;
pub use sessions::SessionsPage;
pub use settings::SettingsPage;
pub use subscriptions::SubscriptionsPage;
pub use system::SystemPage;
pub use usage::UsagePage;
