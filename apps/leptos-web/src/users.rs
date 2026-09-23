//! 用户页的数据与动作。
//!
//! - SSR：`sample_users` 生成初始列表，序列化进 HTML 供 hydration。
//! - CSR：筛选和启停全在客户端信号上，不刷页面；启停通过 server function
//!   落库（试用应用里是进程内状态）。

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// 样例用户。status 1=启用 2=停用，role 1/10/100，quota 单位 500000 = ¥1。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub key: String,
    pub username: String,
    pub email: String,
    pub status: i32,
    pub role: i32,
    pub groups: Vec<String>,
    pub quota: i64,
    pub used_quota: i64,
    pub created_at: String,
}

impl User {
    pub fn enabled(&self) -> bool {
        self.status == 1
    }

    pub fn role_label(&self) -> &'static str {
        match self.role {
            100 => "超级管理员",
            10 => "管理员",
            _ => "普通用户",
        }
    }
}

/// 额度换算：500000 = ¥1。
pub fn cny(quota: i64) -> String {
    format!("¥{:.2}", quota as f64 / 500_000.0)
}

fn sample_users() -> Vec<User> {
    vec![
        u(
            "u1",
            "admin_dev",
            "admin@ferrite.local",
            1,
            100,
            &["default", "vip"],
            50_000_000,
            12_300_000,
            "2026-09-02",
        ),
        u(
            "u2",
            "alice",
            "alice@example.com",
            1,
            10,
            &["default"],
            10_000_000,
            4_200_000,
            "2026-09-14",
        ),
        u(
            "u3",
            "bob",
            "bob@example.com",
            2,
            1,
            &["trial"],
            2_500_000,
            2_500_000,
            "2026-08-21",
        ),
        u(
            "u4",
            "carol",
            "carol@example.com",
            1,
            1,
            &["default", "trial"],
            8_000_000,
            900_000,
            "2026-09-19",
        ),
        u(
            "u5",
            "dave",
            "dave@example.com",
            1,
            10,
            &["vip"],
            20_000_000,
            15_600_000,
            "2026-07-03",
        ),
        u(
            "u6",
            "erin",
            "erin@example.com",
            2,
            1,
            &["default"],
            5_000_000,
            100_000,
            "2026-09-08",
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn u(
    key: &str,
    name: &str,
    email: &str,
    status: i32,
    role: i32,
    groups: &[&str],
    quota: i64,
    used: i64,
    created: &str,
) -> User {
    User {
        key: key.into(),
        username: name.into(),
        email: email.into(),
        status,
        role,
        groups: groups.iter().map(|g| g.to_string()).collect(),
        quota,
        used_quota: used,
        created_at: created.into(),
    }
}

/// 进程内状态。SSR 请求读它做首屏渲染；客户端通过 server function 改它。
#[derive(Clone, Debug)]
pub struct PageState {
    pub users: Vec<User>,
}

impl PageState {
    pub fn fresh() -> Self {
        Self {
            users: sample_users(),
        }
    }

    pub fn toggle(&mut self, key: &str) {
        if let Some(user) = self.users.iter_mut().find(|user| user.key == key) {
            user.status = if user.enabled() { 2 } else { 1 };
        }
    }
}

/// 客户端筛选条件。hydration 后由本地信号持有，点胶囊只改信号，不发包。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Filter {
    pub query: String,
    pub group: String,
    pub status: String,
    pub role: String,
}

impl Filter {
    pub fn matches(&self, user: &User) -> bool {
        let q = self.query.trim().to_lowercase();
        if !q.is_empty()
            && !user.username.to_lowercase().contains(&q)
            && !user.email.to_lowercase().contains(&q)
        {
            return false;
        }
        if !self.group.is_empty() && !user.groups.iter().any(|g| g == &self.group) {
            return false;
        }
        if self.status == "on" && !user.enabled() || self.status == "off" && user.enabled() {
            return false;
        }
        let want_role = match self.role.as_str() {
            "1" => 1,
            "10" => 10,
            "100" => 100,
            _ => 0,
        };
        if want_role != 0 && user.role != want_role {
            return false;
        }
        true
    }
}

/// 取用户列表。SSR 直读进程内状态；CSR 走 HTTP 打同一注册路径。
#[server]
pub async fn list_users() -> Result<Vec<User>, ServerFnError> {
    let state = expect_context::<std::sync::Arc<tokio::sync::Mutex<PageState>>>();
    Ok(state.lock().await.users.clone())
}

/// 切换用户启停。客户端调它落库，回包后本地信号同步翻转。
#[server]
pub async fn toggle_user(key: String) -> Result<(), ServerFnError> {
    let state = expect_context::<std::sync::Arc<tokio::sync::Mutex<PageState>>>();
    state.lock().await.toggle(&key);
    Ok(())
}
