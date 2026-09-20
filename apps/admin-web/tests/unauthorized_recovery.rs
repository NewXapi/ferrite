//! 会话不可恢复时的导航决策矩阵（`unauthorized_recovery` 纯函数，不碰 window）。
//!
//! 背景：`localStorage` 里留着过期 token 时，Zen(Firefox) 打开页面是**纯白且不恢复**
//! （受控实测：同 prefs / 同扩展，只有 origin 存储不同 —— 有旧 token 白屏，清掉即渲染）。
//! 修复让"曾有会话"的 401 走整页 reload，使下一次启动落到已验证可用的"无 token"路径。
//!
//! 这里钉住的是安全不变量：**没有会话时绝不能 reload**。否则无 token → 401 →
//! reload → 无 token → 401 会无限循环，用户连"再试一次"都做不到。

use admin_web::{UnauthorizedRecovery, unauthorized_recovery};

/// 曾有会话 → reload：给下一次启动一个无 token 的干净起点，终止 401 风暴。
#[test]
fn session_loss_reloads() {
    assert_eq!(unauthorized_recovery(true), UnauthorizedRecovery::Reload);
}

/// 无会话 → 只跳登录页。这一条是防无限 reload 循环的护栏，改坏了整站就无法打开。
#[test]
fn no_session_never_reloads() {
    assert_eq!(unauthorized_recovery(false), UnauthorizedRecovery::ToSignUp);
}
