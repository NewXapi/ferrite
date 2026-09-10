use gloo_net::http::Request;
use http::{Method, StatusCode};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::manage_auth_token::{AuthState, SharedAuthState, TokenFuture};

/// 刷新成功后的竞态抑制窗口: 窗口内的 401 视为并发尾巴, 直接重试或跳过清理。
const UNAUTHORIZED_SUPPRESS_WINDOW: Duration = Duration::from_secs(5);
use crate::{ApiError, ApiResult, Envelope};

/// HTTP client for the New API backend with automatic Bearer token injection
/// and one-shot 401 token refresh.
///
/// Mirrors the React axios client contract: same-origin requests, `Cache-Control:
/// no-store`, Bearer auth from shared state, and a single refresh retry on 401.
pub struct ApiClient {
    auth: SharedAuthState,
    base_url: String,
}

impl Clone for ApiClient {
    /// Cheap clone — shares the same `AuthState` via `Rc<RefCell<…>>`.
    fn clone(&self) -> Self {
        Self {
            auth: self.auth.clone(),
            base_url: self.base_url.clone(),
        }
    }
}

impl ApiClient {
    /// 进程级共享 Client（wasm 单线程）。thread_local Cell 持有 owned
    /// 实例，`shared()` 返回 clone——所有 clone 经 `Rc<RefCell<AuthState>>`
    /// 共享同一登录态，登录后 `set_token` 全局可见。
    pub fn shared() -> ApiClient {
        thread_local! {
            static SHARED: ApiClient = ApiClient::new();
        }
        SHARED.with(|c| c.clone())
    }

    /// Create a client for same-origin requests (empty `base_url`).
    pub fn new() -> Self {
        Self::with_base_url("")
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            auth: Rc::new(RefCell::new(AuthState::new())),
            base_url: base_url.into(),
        }
    }

    /// Set the access token; `None` clears it.
    pub fn set_token(&self, token: Option<String>) {
        self.auth.borrow_mut().set_token(token);
    }

    /// Get the current access token if set.
    pub fn token(&self) -> Option<String> {
        self.auth.borrow().token()
    }

    /// Register a token refresher. Called at most once per 401 response.
    ///
    /// The refresher must return `Some(new_token)` on success, `None` on failure.
    pub fn set_refresher(&self, f: impl Fn() -> TokenFuture + 'static) {
        self.auth.borrow_mut().set_refresher(Box::new(f));
    }

    /// Register a callback fired when a 401 cannot be recovered.
    pub fn set_on_unauthorized(&self, f: impl Fn() + 'static) {
        self.auth.borrow_mut().set_on_unauthorized(Box::new(f));
    }

    /// 401 无法恢复时的统一出口。回调先克隆、释放 borrow 之后再调用 —
    /// 回调内部通常 set_token (borrow_mut), 在 borrow 作用域内同步触发会
    /// RefCell 重入 panic。并发 401 的竞态尾巴 (刚成功刷新过) 则跳过清理。
    fn dispatch_unauthorized(&self) {
        let cb = {
            let state = self.auth.borrow();
            if state.recently_refreshed(UNAUTHORIZED_SUPPRESS_WINDOW) {
                return;
            }
            state.cloned_on_unauthorized()
        };
        if let Some(cb) = cb {
            cb();
        }
    }

    /// GET request with 401 refresh retry.
    pub async fn get<T: DeserializeOwned + Default>(&self, path: &str) -> ApiResult<T> {
        self.request(Method::GET, path, None::<&()>).await
    }

    /// POST request with 401 refresh retry.
    pub async fn post<B: Serialize, T: DeserializeOwned + Default>(
        &self,
        path: &str,
        body: &B,
    ) -> ApiResult<T> {
        self.request(Method::POST, path, Some(body)).await
    }

    /// PUT request with 401 refresh retry.
    pub async fn put<B: Serialize, T: DeserializeOwned + Default>(
        &self,
        path: &str,
        body: &B,
    ) -> ApiResult<T> {
        self.request(Method::PUT, path, Some(body)).await
    }

    /// DELETE request with 401 refresh retry.
    pub async fn delete<T: DeserializeOwned + Default>(&self, path: &str) -> ApiResult<T> {
        self.request(Method::DELETE, path, None::<&()>).await
    }

    /// Single-attempt POST without 401 refresh loop.
    ///
    /// Used by the session crate for `POST /api/user/auth/refresh` itself to
    /// avoid recursive refresh. `headers` adds extra request headers
    /// (e.g. `X-Auth-Session`).
    pub async fn post_once<B: Serialize, T: DeserializeOwned + Default>(
        &self,
        path: &str,
        body: &B,
        headers: Option<&[(&str, &str)]>,
    ) -> ApiResult<T> {
        self.request_once(Method::POST, path, Some(body), headers)
            .await
    }

    /// Core request with 401 retry logic.
    async fn request<B, T>(&self, method: Method, path: &str, body: Option<&B>) -> ApiResult<T>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned + Default,
    {
        // First attempt
        let mut result = self.request_once(method.clone(), path, body, None).await;

        // Check if we got a 401 that we can retry
        if let Err(ApiError::Unauthorized) = &result {
            // 并发竞态: 另一路请求刚刚完成刷新 → 直接带新 token 重试,
            // 不再触发第二次 refresh (旧 refresh token 已被轮换, 必失败)。
            if self
                .auth
                .borrow()
                .recently_refreshed(UNAUTHORIZED_SUPPRESS_WINDOW)
            {
                result = self.request_once(method, path, body, None).await;
            } else {
                // Produce the refresh future without holding the borrow across await.
                let refresh_fut = self.auth.borrow().refresh();
                if let Some(fut) = refresh_fut {
                    if let Some(new_token) = fut.await {
                        self.set_token(Some(new_token));
                        self.auth.borrow_mut().mark_refreshed();
                        // Retry once with the new token.
                        result = self.request_once(method, path, body, None).await;
                    } else {
                        self.dispatch_unauthorized();
                    }
                } else {
                    self.dispatch_unauthorized();
                }
            }
        }

        result
    }

    /// Single request attempt (no 401 retry).
    async fn request_once<B, T>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        headers: Option<&[(&str, &str)]>,
    ) -> ApiResult<T>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned + Default,
    {
        let url = format!("{}{}", self.base_url, path);

        // gloo-net uses static methods per HTTP verb
        let builder = match method {
            Method::GET => Request::get(&url),
            Method::POST => Request::post(&url),
            Method::PUT => Request::put(&url),
            Method::DELETE => Request::delete(&url),
            _ => return Err(ApiError::Transport(format!("unsupported method: {method}"))),
        };

        let builder = if let Some(extra) = headers {
            let mut b = builder;
            for (name, value) in extra {
                b = b.header(name, value);
            }
            b
        } else {
            builder
        };

        // Inject Bearer token if present
        let builder = if let Some(token) = self.token() {
            builder.header("Authorization", &format!("Bearer {}", token))
        } else {
            builder
        };

        // Cookies: fetch default credentials is same-origin — nothing to do.

        // Add JSON body if present
        let request = if let Some(body) = body {
            builder
                .json(body)
                .map_err(|e| ApiError::Transport(e.to_string()))?
        } else {
            builder
                .build()
                .map_err(|e| ApiError::Transport(e.to_string()))?
        };

        let response = request
            .send()
            .await
            .map_err(|e| ApiError::Transport(e.to_string()))?;

        let status = response.status();
        if !response.ok() {
            let status_code =
                StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let message = match response.json::<Envelope<JsonValue>>().await {
                Ok(env) => env.message,
                Err(_) => status_code
                    .canonical_reason()
                    .unwrap_or("unknown error")
                    .to_string(),
            };

            if status == 401 {
                return Err(ApiError::Unauthorized);
            }
            return Err(ApiError::Http { status, message });
        }

        // 2xx: decode strategy —
        // 1) Envelope{success,message,data}（admin-catalog/observe 端点）
        // 2) 裸 JSON（auth crate 的 login/register/self 返回 SelfView/LoginResponse）
        let raw = response
            .text()
            .await
            .map_err(|e| ApiError::Decode(e.to_string()))?;
        if let Ok(env) = serde_json::from_str::<Envelope<serde_json::Value>>(&raw) {
            if !env.success {
                return Err(ApiError::Business(env.message));
            }
            let value = env.data.unwrap_or_default();
            let t: T =
                serde_json::from_value(value).map_err(|e| ApiError::Decode(e.to_string()))?;
            return Ok(t);
        }
        let t: T = serde_json::from_str(&raw).map_err(|e| ApiError::Decode(e.to_string()))?;
        Ok(t)
    }
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}

// ponytail: GET request dedup (inFlightGet map) skipped — add when needed
// ponytail: auth rotation (applyAuthRotation on success) skipped — add when needed
