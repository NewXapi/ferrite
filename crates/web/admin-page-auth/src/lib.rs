//! Auth page library: provides state context provider plus public entry.

pub mod api;
pub mod components;
#[path = "tab-page/mod.rs"]
pub mod tab_page;

pub use tab_page::auth::AuthPageRoot;
