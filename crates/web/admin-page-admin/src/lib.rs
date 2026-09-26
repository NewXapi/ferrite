//! Admin page: topology/network management plus entity settings.

pub mod api;
pub mod components;
pub mod drawer_write;
pub mod network_data;
pub mod network_physics;
pub mod parse_url_key;
pub mod shared;
pub mod state;
#[path = "tab-page/mod.rs"]
pub mod tab_page;

pub use network_data::{GraphView, NodeKey, bump_topo_refresh, channel_models, edges_of};
pub use parse_url_key::parse_url_key;
pub use tab_page::gateway::GatewayHealthPanel;
pub use tab_page::groups::GroupsPage;
pub use tab_page::network::NetworkPanel;
pub use tab_page::redemptions::RedemptionsPage;
pub use tab_page::subscriptions::SubscriptionsPage;
pub use tab_page::system::SystemPage;
