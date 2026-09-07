//! Admin page: topology/network management plus entity settings.

pub mod aliases;
pub mod api;
pub mod channels;
pub mod entities;
pub mod groups;
pub mod network;
pub mod pages;
pub mod redemptions;
pub mod state;
pub mod system;
pub use aliases::AliasesPage;
pub use channels::ChannelsPage;
pub use groups::GroupsPage;
pub use network::NetworkPanel;
pub use pages::{SubscriptionsPage, parse_url_key};
pub use redemptions::RedemptionsPage;
pub use system::SystemPage;
