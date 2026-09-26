//! Admin page: topology/network management plus entity settings.

pub mod api;
pub mod components;
pub mod drawer_write;
pub mod parse_url_key;
pub mod shared;
pub mod state;

#[path = "tab-page-entities/mod.rs"]
pub mod tab_page_entities;
#[path = "tab-page-groups/mod.rs"]
pub mod tab_page_groups;
#[path = "tab-page-network/mod.rs"]
pub mod tab_page_network;

pub use tab_page::aliases::AliasesPage;
pub use tab_page::channels::ChannelsPage;
pub use tab_page::currency::CurrencyPage;
#[path = "tab-page/mod.rs"]
pub mod tab_page;

pub use parse_url_key::parse_url_key;
pub use tab_page::gateway::GatewayHealthPanel;
pub use tab_page::redemptions::RedemptionsPage;
pub use tab_page::subscriptions::SubscriptionsPage;
pub use tab_page::system::SystemPage;
pub use tab_page_groups::GroupsPage;
pub use tab_page_network::{
    GraphView, NetworkPanel, NodeKey, bump_topo_refresh, channel_models, edges_of,
    topo_refresh_version,
};
