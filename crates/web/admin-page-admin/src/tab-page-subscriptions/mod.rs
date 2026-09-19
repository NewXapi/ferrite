//! 订阅套餐 tab。拆分约定:
//! - `page`:页面入口(`SubscriptionsPage`),持有弹窗开关与 21 个 `f_*` 表单
//!   signal,承载 `open_edit` / `open_new` / `commit` 三个写回闭包
//! - `card`:单张套餐卡(`PlanCard`),纯展示 + 三个回调出口
//! - `modal`:编辑/新建弹窗(`SubscriptionFormModal`)与三个 Tab 体
//! - `shared`:页面骨架件(`GridShell` / `Panel`)、通用按钮与 `ToggleSwitch`,
//!   以及该 tab 的全部用户可见文案常量
//! - `parse_url_key`:URL/Key 文本解析纯函数(无 UI 依赖)
//!
//! 边界:本 tab 数据全走本地 `EntityStore` 演示态(订阅套餐后端暂未实现),
//! 不含任何网络调用。

#[path = "card.rs"]
pub mod card;
#[path = "modal.rs"]
pub mod modal;
#[path = "page.rs"]
pub mod page;
#[path = "parse-url-key.rs"]
pub mod parse_url_key;
#[path = "shared.rs"]
pub mod shared;

pub use card::PlanCard;
pub use modal::SubscriptionFormModal;
pub use page::SubscriptionsPage;
pub use parse_url_key::parse_url_key;
