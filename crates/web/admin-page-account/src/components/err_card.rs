//! 邀请·奖励 tab 共享组件:四个区 (钱包/充值/拉人统计/被邀人) 共用的错误态卡。
//!
//! 原 `rewards.rs` 只有一个 `err_card`, 拆分后每个区各持一份必然漂移,
//! 故收敛回单一定义供各区 `use`。

use dioxus::prelude::*;

/// 错误态统一渲染:柔和红边卡片 (非满屏红),对齐 keys.rs 的诚实降级文案。
#[component]
pub fn ErrCard(testid: &'static str, what: &'static str, msg: String) -> Element {
    rsx! {
        div {
            class: "rounded-xl border border-red-500/40 bg-zinc-900 p-4",
            "data-testid": testid,
            p { class: "text-sm text-red-300", "无法加载{what} (未登录或请求失败): {msg}" }
        }
    }
}
