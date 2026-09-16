//! debug-auto-login 触发判据矩阵（`should_debug_auto_login` 纯函数，不碰 window）。
//! 覆盖：feature 关恒 false；feature 开时「无 token + 非 auth hash」才触发；
//! 登录页 hash（#login/#signup/#auth）不触发；空 hash 视作普通页。
//!
//! 判据函数与 feature 无关地恒编译（feature_on 由调用方以 `cfg!` 传入），
//! 因此本测试无论是否带 `--features debug-auto-login` 都应通过。

use admin_web::should_debug_auto_login;

/// feature 关：无论 token / hash 处于什么状态，一律不触发（编译产物行为与现状零变化）。
#[test]
fn feature_off_never_triggers() {
    // 有 token 的正常会话
    assert!(!should_debug_auto_login(false, true, "#manage"));
    // 未登录的普通页（本该触发的形态，但 feature 关必须拦住）
    assert!(!should_debug_auto_login(false, false, "#manage"));
    assert!(!should_debug_auto_login(false, false, ""));
    // 登录页 hash 更不应触发
    assert!(!should_debug_auto_login(false, false, "#login"));
}

/// feature 开 + 已有登录态：不重复触发（自动登录写过 token 后 reload 回来正是此形态）。
#[test]
fn feature_on_with_token_never_triggers() {
    assert!(!should_debug_auto_login(true, true, "#manage"));
    assert!(!should_debug_auto_login(true, true, ""));
    assert!(!should_debug_auto_login(true, true, "#overview"));
}

/// feature 开 + 无 token + 普通 console hash：触发免登录进控制台。
#[test]
fn feature_on_without_token_on_console_hash_triggers() {
    assert!(should_debug_auto_login(true, false, "#manage"));
    assert!(should_debug_auto_login(true, false, "#overview"));
    assert!(should_debug_auto_login(true, false, "#system"));
    // 拓扑分支页不是 auth 页，同样触发
    assert!(should_debug_auto_login(true, false, "#retro"));
}

/// feature 开 + 无 token + 空 hash：视作普通页，触发（首次打开站点即免登录直达控制台）。
#[test]
fn empty_hash_is_treated_as_console() {
    assert!(should_debug_auto_login(true, false, ""));
}

/// feature 开 + 无 token + 登录/注册页 hash：不触发，保留手动调试登录页的能力。
#[test]
fn auth_hashes_never_trigger() {
    assert!(!should_debug_auto_login(true, false, "#login"));
    assert!(!should_debug_auto_login(true, false, "#signup"));
    assert!(!should_debug_auto_login(true, false, "#auth"));
}

/// hash 精确匹配而非包含匹配：`#login` 的前缀/子串变形不算登录页（避免误拦 `#loginfoo`），
/// 反过来带 query 片段的写法不在判据内，按普通页处理。
#[test]
fn hash_matching_is_exact() {
    // 子串变形 ≠ 登录页 → 触发
    assert!(should_debug_auto_login(true, false, "#loginx"));
    assert!(should_debug_auto_login(true, false, "#xlogin"));
    // 前后空白不剥离（浏览器 location.hash 不会带空白，不做隐式 trim）
    assert!(should_debug_auto_login(true, false, " #login"));
}
