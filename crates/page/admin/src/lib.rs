//! Admin page: topology/network management plus per-entity tabs.

pub mod api;
pub mod billing;
pub mod channels;
pub mod entities;
pub mod groups;
pub mod network;
pub mod security;
pub mod showcase;
pub mod state;
pub mod system;
pub mod ui;

pub use billing::BillingPanel;
pub use channels::ChannelsPanel;
pub use groups::GroupsPanel;
pub use network::NetworkPanel;
pub use security::SecurityPanel;
pub use showcase::ShowcasePanel;
pub use system::SystemPanel;

