//! Admin page: topology/network management plus entity settings.

pub mod aliases;
pub mod api;
pub mod channels;
pub mod currency;
pub mod drawer_write;
pub mod entities;
pub mod gateway;
pub mod groups;
pub mod network;
pub mod pages;
pub mod redemptions;
pub mod state;
pub mod system;
pub use aliases::AliasesPage;
pub use channels::ChannelsPage;
pub use currency::CurrencyPage;
pub use gateway::GatewayHealthPanel;
pub use groups::GroupsPage;
pub use network::{
    GraphView, NetworkPanel, NodeKey, bump_topo_refresh, channel_models, edges_of,
    topo_refresh_version,
};
pub use pages::{SubscriptionsPage, parse_url_key};
pub use redemptions::RedemptionsPage;
pub use system::SystemPage;
