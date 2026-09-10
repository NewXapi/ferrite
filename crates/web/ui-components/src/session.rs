//! 会话与令牌存储、HTTP 认证请求。

use contract::api::auth::{LoginRequest, LoginResponse, RegisterRequest};
use contract::api::user::UserDto;

const TOKEN_KEY: &str = "ferrite_access_token";
const USER_KEY: &str = "ferrite_current_user";

pub fn get_storage_item(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let w = web_sys::window()?;
        // 优先 localStorage (勾选 Remember me), 回退 sessionStorage (未勾选的当前会话)。
        // 否则未勾选时 token 只写在 sessionStorage, 而恢复逻辑只读 localStorage,
        // 会导致登录后立刻掉登录。
        for s in [w.local_storage().ok(), w.session_storage().ok()]
            .into_iter()
            .flatten()
            .flatten()
        {
            if let Ok(Some(v)) = s.get_item(key) {
                return Some(v);
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
        if let Some(w) = web_sys::window() {
            // 两条存储都清, 覆盖 Remember me 勾选 / 未勾选两种情况。
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
        // 离线/网络错误时不再写 mock 假 token (会绕过「API 必须有用户身份」原则,
        // 让受保护接口全部 401 但顶栏仍显示用户名)。直接上抛错误, 由调用方展示。
        Err(_) => Err("无法连接后端 (网络错误或后端未启动)".into()),
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
        // 网络错误直接上抛, 不做离线模拟登录 (同 api_login 的口径)。
        Err(_) => Err("无法连接后端 (网络错误或后端未启动)".into()),
    }
}

/// 当前 access token 落在哪级存储: true = localStorage (勾过 Remember me),
/// false = 仅 sessionStorage (或未登录)。401 静默刷新轮换 token 时按原级别写回,
/// 避免把会话级登录升级成长期登录。
pub fn token_is_persistent() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.local_storage().ok())
            .flatten()
            .and_then(|s| s.get_item(TOKEN_KEY).ok())
            .flatten()
            .is_some()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

/// 复制文本到系统剪贴板 (Clipboard API, 需要 secure context; localhost 视为安全)。
/// 只发起写入不等待 Promise 结果; 是否成功由调用方按需另行提示。
pub fn copy_text_to_clipboard(text: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(w) = web_sys::window() {
            // write_text 返回 Promise (fire-and-forget), 提交即视为已发起
            let _ = w.navigator().clipboard().write_text(text);
            return true;
        }
        false
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = text;
        false
    }
}
