//! 订阅套餐 tab。拆分约定:
//! - `page`:页面入口(`SubscriptionsPage`),持有列表 signal 与 8 个 `f_*`
//!   表单 signal,承载 `open_edit` / `open_new` / `commit` / 行内启停 / 行内删除
//!   五个写回闭包(均调真实 `/api/subscriptions` 端点,成功后 reload 重拉全表)
//! - `card`:单张套餐卡(`PlanCard`),纯展示 + 三个回调出口(下标回传)
//! - `modal`:编辑/新建弹窗(`SubscriptionFormModal`),两 Tab,表单 signal 由
//!   页面持有并传入
//! - `shared`:页面骨架件(`GridShell` / `Panel`)、通用按钮与 `ToggleSwitch`,
//!   以及该 tab 的全部用户可见文案常量
//! - `parse_url_key`:URL/Key 文本解析纯函数(无 UI 依赖)
//!
//! 数据走真实后端:列表 `use_effect` 拉 `list_subscriptions_api`,写回调
//! `upsert_subscription_api` / `delete_subscription_api`,后端按 name upsert、
//! 按 sort_order 排序,故写回成功后一律重拉全表而非本地插入。

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
