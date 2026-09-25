//! 用户列表操作行:「编辑 / 充值 / 启停」三键,经 `ui::ActionButtonGroup` 复用渲染。
//!
//! 三颗按钮是同一枚「卡片操作按钮」的一对多实例(共享 `ActionButton` 条目组件
//! 与 tone 样式);本组件只做两件事:按用户状态推导按钮 spec 集合、按下下标
//! 分派到三个意图回调。

use contract::api::admin::AdminUserDto;
use dioxus::prelude::*;

use crate::shared::{BTN_EDIT, BTN_TOPUP, STATUS_DISABLED, STATUS_ENABLED};
use ui::{ActionButtonGroup, ActionSpec, ActionTone};

/// 用户卡底部操作行(编辑 / 充值 / 启停三键)。
///
/// 【是什么】`ui::ActionButtonGroup` 的页面装配:3 颗 `ActionSpec` + 下标分派。
///
/// 【做什么】按 `user.status` 推导第三颗按钮的文案与 wire 动作;按下时按下标
/// 分派到 `on_edit` / `on_topup` / `on_toggle`。
///
/// 【交互逻辑】
/// - 下标 0 = 编辑 → `on_edit(key)`。
/// - 下标 1 = 充值 → `on_topup(key)`。
/// - 下标 2 = 启停 → `on_toggle((key, action, None))`,`action` 是 snake_case
///   `enable` / `disable`(后端 `ManageUserAction` 拒中文变体,按钮文案仍显示中文)。
///
/// 【样式】无自有样式:容器与按钮全部来自 `ui::ActionButtonGroup`(tone 着色,
/// 与旧卡 class 逐字一致)。
///
/// 【子组件组成】`ui::ActionButtonGroup`(内含 `ui::ActionButton` × 3)。
///
/// 【数据流】
/// - 对内(入):`user`(单条 DTO)、三个意图回调(均以 `user.key` 定位行)。
/// - 对外(出):按按下下标分派到三个回调。
#[component]
pub fn UserActions(
    user: AdminUserDto,
    on_edit: EventHandler<String>,
    on_topup: EventHandler<String>,
    on_toggle: EventHandler<(String, String, Option<String>)>,
) -> Element {
    let key = user.key.clone();
    let enabled = user.status == 1;
    let specs = vec![
        ActionSpec {
            label: BTN_EDIT.to_string(),
            tone: ActionTone::Neutral,
            disabled: false,
            testid: Some("user-edit".to_string()),
        },
        ActionSpec {
            label: BTN_TOPUP.to_string(),
            tone: ActionTone::SuccessSoft,
            disabled: false,
            testid: Some("user-topup".to_string()),
        },
        ActionSpec {
            label: if enabled {
                STATUS_DISABLED.to_string()
            } else {
                STATUS_ENABLED.to_string()
            },
            tone: ActionTone::Warning,
            disabled: false,
            testid: Some("user-toggle".to_string()),
        },
    ];
    rsx! {
        ActionButtonGroup {
            actions: specs,
            testid_prefix: "user-actions".to_string(),
            on_press: move |i| match i {
                0 => on_edit.call(key.clone()),
                1 => on_topup.call(key.clone()),
                _ => on_toggle.call((
                    key.clone(),
                    // wire 动作名是 snake_case 英文(后端 ManageUserAction 拒中文变体);
                    // 按钮文案仍显示中文,仅 value 走 enable/disable
                    if enabled { "disable".to_string() } else { "enable".to_string() },
                    None,
                )),
            },
        }
    }
}
