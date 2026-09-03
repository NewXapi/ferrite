//! Admin page: topology/network management plus entity settings.

pub mod entities;
pub mod groups;
pub mod network;
pub mod pages;
pub mod state;
pub use groups::GroupsPage;
pub use network::NetworkPanel;
pub use pages::{AliasesPage, ChannelsPage, RedemptionsPage, SubscriptionsPage, SystemPage, parse_url_key};

