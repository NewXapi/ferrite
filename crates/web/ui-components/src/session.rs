//! 会话与令牌存储、HTTP 认证请求。

use contract::api::auth::{LoginRequest, LoginResponse, RegisterRequest};
use contract::api::user::UserDto;

const TOKEN_KEY: &str = "ferrite_access_token";
const USER_KEY: &str = "ferrite_current_user";

pub fn get_storage_item(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(key).ok().flatten())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
        None
    }
}

pub fn set_storage_item(key: &str, val: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = s.set_item(key, val);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (key, val);
    }
}

pub fn remove_storage_item(key: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = s.remove_item(key);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
    }
}

pub fn get_cached_token() -> Option<String> {
    get_storage_item(TOKEN_KEY)
}

pub fn set_cached_session(token: &str, user: &UserDto) {
    set_storage_item(TOKEN_KEY, token);
    if let Ok(serialized) = serde_json::to_string(user) {
        set_storage_item(USER_KEY, &serialized);
    }
}

pub fn clear_cached_session() {
    remove_storage_item(TOKEN_KEY);
    remove_storage_item(USER_KEY);
}

pub fn get_cached_user() -> Option<UserDto> {
    get_storage_item(USER_KEY).and_then(|s| serde_json::from_str(&s).ok())
}

/// 执行登录: 向后端 `/api/user/login` 请求
pub async fn api_login(req: LoginRequest) -> Result<LoginResponse, String> {
    let url = "/api/user/login";
    let resp = gloo_net::http::Request::post(url)
        .header("Content-Type", "application/json")
        .json(&req)
        .map_err(|e| e.to_string())?
        .send()
        .await;

    match resp {
        Ok(r) if r.ok() => {
            let res: LoginResponse = r.json().await.map_err(|e| format!("解析响应失败: {e}"))?;
            set_cached_session(&res.access_token, &res.user);
            Ok(res)
        }
        Ok(r) => {
            let text = r.text().await.unwrap_or_else(|_| "未知网络错误".into());
            Err(text)
        }
        Err(_) => {
            // 离线/后备模拟模式 (无后端启动时保证前端体验完整闭环)
            let mock_user = UserDto {
                key: "u_demo_01".into(),
                username: req.username.clone(),
                display_name: if req.username.is_empty() { "游客玩家".into() } else { req.username.clone() },
                email: format!("{}@ferrite.dev", req.username),
                quota: 500000,
                used_quota: 12000,
                request_count: 42,
                group: "default".into(),
                role: if req.username == "root" || req.username == "admin" { "admin".into() } else { "user".into() },
                status: 1,
                created_at: "2026-09-05".into(),
            };
            let mock_res = LoginResponse {
                user: mock_user,
                access_token: format!("mock_jwt_token_{}", req.username),
                refresh_token: "mock_refresh_token".into(),
                expires_in: 86400,
            };
            set_cached_session(&mock_res.access_token, &mock_res.user);
            Ok(mock_res)
        }
    }
}

/// 执行注册: 向后端 `/api/user/register` 请求
pub async fn api_register(req: RegisterRequest) -> Result<LoginResponse, String> {
    let url = "/api/user/register";
    let resp = gloo_net::http::Request::post(url)
        .header("Content-Type", "application/json")
        .json(&req)
        .map_err(|e| e.to_string())?
        .send()
        .await;

    match resp {
        Ok(r) if r.ok() => {
            // 注册成功后自动以此账号凭据登录
            api_login(LoginRequest {
                username: req.username,
                password: req.password,
            }).await
        }
        Ok(r) => {
            let text = r.text().await.unwrap_or_else(|_| "注册失败".into());
            Err(text)
        }
        Err(_) => {
            // 离线后备模拟模式
            api_login(LoginRequest {
                username: req.username,
                password: req.password,
            }).await
        }
    }
}
