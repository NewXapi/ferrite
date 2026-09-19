//! Admin page: topology/network management plus entity settings.

pub mod api;
pub mod drawer_write;
pub mod state;

#[path = "tab-page-aliases/mod.rs"]
pub mod tab_page_aliases;
#[path = "tab-page-channels/mod.rs"]
pub mod tab_page_channels;
#[path = "tab-page-currency/mod.rs"]
pub mod tab_page_currency;
#[path = "tab-page-gateway/mod.rs"]
pub mod tab_page_gateway;
#[path = "tab-page-redemptions/mod.rs"]
pub mod tab_page_redemptions;
#[path = "tab-page-system/mod.rs"]
pub mod tab_page_system;

#[path = "tab-page-entities/mod.rs"]
pub mod tab_page_entities;
#[path = "tab-page-groups/mod.rs"]
pub mod tab_page_groups;
#[path = "tab-page-network/mod.rs"]
pub mod tab_page_network;
#[path = "tab-page-subscriptions/mod.rs"]
pub mod tab_page_subscriptions;

pub use tab_page_aliases::AliasesPage;
pub use tab_page_channels::ChannelsPage;
pub use tab_page_currency::CurrencyPage;
pub use tab_page_gateway::GatewayHealthPanel;
pub use tab_page_groups::GroupsPage;
pub use tab_page_network::{
    GraphView, NetworkPanel, NodeKey, bump_topo_refresh, channel_models, edges_of,
    topo_refresh_version,
};
pub use tab_page_redemptions::RedemptionsPage;
pub use tab_page_subscriptions::{SubscriptionsPage, parse_url_key};
pub use tab_page_system::SystemPage;
