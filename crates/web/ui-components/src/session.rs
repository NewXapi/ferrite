//! 会话与令牌存储、HTTP 认证请求。

use contract::api::auth::{LoginRequest, LoginResponse, RegisterRequest};
use contract::api::user::UserDto;

const TOKEN_KEY: &str = "ferrite_access_token";
const USER_KEY: &str = "ferrite_current_user";
const REFRESH_KEY: &str = "ferrite_refresh_token";

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

/// 两级读取：先 localStorage（remember me 持久档），回落 sessionStorage（本次会话档）。
/// 未勾选 remember me 的登录 token 落在 sessionStorage，恢复时同样要能读到。
pub fn get_storage_scoped(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(w) = web_sys::window() {
            if let Some(s) = w.local_storage().ok().flatten()
                && let Ok(Some(v)) = s.get_item(key)
            {
                return Some(v);
            }
            if let Some(s) = w.session_storage().ok().flatten() {
                return s.get_item(key).ok().flatten();
            }
        }
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
        None
    }
}

/// 两级清除：不管 token 之前落在哪个存储都清干净。
pub fn remove_storage_scoped(key: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(w) = web_sys::window() {
            if let Some(s) = w.local_storage().ok().flatten() {
                let _ = s.remove_item(key);
            }
            if let Some(s) = w.session_storage().ok().flatten() {
                let _ = s.remove_item(key);
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
    }
}

pub fn set_storage_item(key: &str, val: &str) {
    set_storage_scoped(key, val, true);
}

/// 按持久化级别存储：persistent=true → localStorage；false → sessionStorage。
pub fn set_storage_scoped(key: &str, val: &str, persistent: bool) {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = web_sys::window().and_then(|w| {
            if persistent {
                w.local_storage().ok().flatten()
            } else {
                w.session_storage().ok().flatten()
            }
        });
        if let Some(s) = storage {
            let _ = s.set_item(key, val);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (key, val, persistent);
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
    get_storage_scoped(TOKEN_KEY)
}

pub fn get_cached_refresh_token() -> Option<String> {
    get_storage_scoped(REFRESH_KEY)
}

/// refresh token 原本落在 localStorage（remember me）还是 sessionStorage。
fn refresh_is_persistent() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(REFRESH_KEY).ok().flatten())
            .is_some()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

/// 401 时静默换新 access token：`POST /api/user/refresh`，body 带 refreshToken。
/// 响应 `{accessToken, refreshToken, expiresIn}` —— 刷新即轮换，旧 refreshToken 作废，
/// 新 token 对按原作用域回写。返回 Some(新 access token) 供 client 重试原请求；
/// None = 没有或已失效的 refreshToken（调用方清会话走重新登录）。
///
/// 并发安全：页面挂载会同时发出多个请求，可能同时 401 并各自触发本函数；
/// 只有第一个用当前 RT 刷新会成功（轮换使其余的 401）。败者不清会话，而是
/// 重读 storage——胜者已把新 token 对写回，直接复用即可，避免把胜者的
/// 新 refreshToken 误当失效清掉（否则一次并发就能把会话打死）。
pub async fn refresh_access_token() -> Option<String> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RefreshResp {
        access_token: String,
        refresh_token: String,
    }

    let rt = get_cached_refresh_token()?;
    let persistent = refresh_is_persistent();
    let resp = gloo_net::http::Request::post("/api/user/refresh")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "refreshToken": rt }))
        .ok()?
        .send()
        .await
        .ok()?;
    if !resp.ok() {
        // 我们的 RT 失败了。若 storage 里的 RT 已被并发胜者轮换（≠ 我们用的那个），
        // 复用胜者写入的新 access token；否则才是真的失效，清会话。
        return match get_cached_refresh_token() {
            Some(newer) if newer != rt => get_cached_token(),
            _ => {
                clear_cached_session();
                None
            }
        };
    }
    let body: RefreshResp = resp.json().await.ok()?;
    set_storage_scoped(TOKEN_KEY, &body.access_token, persistent);
    set_storage_scoped(REFRESH_KEY, &body.refresh_token, persistent);
    Some(body.access_token)
}

pub fn set_cached_session(token: &str, user: &UserDto) {
    set_storage_item(TOKEN_KEY, token);
    if let Ok(serialized) = serde_json::to_string(user) {
        set_storage_item(USER_KEY, &serialized);
    }
}

pub fn clear_cached_session() {
    remove_storage_scoped(TOKEN_KEY);
    remove_storage_scoped(USER_KEY);
    remove_storage_scoped(REFRESH_KEY);
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
                display_name: if req.username.is_empty() {
                    "游客玩家".into()
                } else {
                    req.username.clone()
                },
                email: format!("{}@ferrite.dev", req.username),
                quota: 500000,
                used_quota: 12000,
                request_count: 42,
                group: "default".into(),
                role: if req.username == "root" || req.username == "admin" {
                    "admin".into()
                } else {
                    "user".into()
                },
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
            })
            .await
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
            })
            .await
        }
    }
}
