//! 管理页组件层:只放 `#[component]` 组件(spec 理念 2);跨页复用组件在
//! `ui-components`;文件按业务名/段落边界拆分,命名前缀区分归属。

// Aliases
pub mod aliases_card;
pub mod aliases_list_section;
pub mod aliases_modal;
pub mod aliases_stats_section;
pub mod aliases_toolbar_section;

// Channels
pub mod channels_card;
pub mod channels_list_section;
pub mod channels_modal;
pub mod channels_stats_section;
pub mod channels_toolbar_section;

// Currency
pub mod currency_form_section;
pub mod currency_list_section;

// Entities
pub mod entities_cards_section;
pub mod entities_channels_section;

// Gateway
pub mod gateway_row_section;

// Groups
pub mod groups_list_section;
pub mod groups_modal;
pub mod groups_stats_section;
pub mod groups_toolbar_section;

// Network
pub mod network_drawer;
pub mod network_inspector;
pub mod network_ui_section;

// Redemptions
pub mod redemptions_card;
pub mod redemptions_list_section;
pub mod redemptions_modal;
pub mod redemptions_stats_section;
pub mod redemptions_toolbar_section;

// Subscriptions
pub mod subscriptions_card;
pub mod subscriptions_list_section;
pub mod subscriptions_modal;
pub mod subscriptions_parse_url_key;

// System
pub mod system_options_section;
pub mod system_overview_section;
pub mod system_proxy_nodes_section;
pub mod system_proxy_runtime_section;

// Shared
pub mod badge;

pub use badge::Badge;