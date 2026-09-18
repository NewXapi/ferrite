#[path = "data.rs"]
pub mod data;
#[path = "physics.rs"]
pub mod physics;
#[path = "drawer.rs"]
pub mod drawer;
#[path = "inspector.rs"]
pub mod inspector;
#[path = "ui.rs"]
pub mod ui;

pub use data::*;
pub use ui::NetworkPanel;
