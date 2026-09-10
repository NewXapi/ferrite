use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Future produced by the refresher: `Some(new access token)` on success.
pub type TokenFuture = Pin<Box<dyn Future<Output = Option<String>>>>;
/// Registered by the session crate; invoked at most once per 401 request-cycle
/// and stays registered for the client's lifetime.
pub type Refresher = Box<dyn Fn() -> TokenFuture>;

/// Mutable auth state shared by every `ApiClient` clone.
///
/// Held behind `Rc<RefCell<…>>` because wasm is single-threaded and every
/// cheap clone of `ApiClient` must observe the same token/refresher.
#[doc(hidden)]
pub struct AuthState {
    token: Option<String>,
    refresher: Option<Refresher>,
    on_unauthorized: Option<Rc<dyn Fn()>>,
    /// 最近一次成功刷新的时刻; 用于识别并发 401 的竞态尾巴。
    last_refresh_at: Option<Instant>,
}

impl AuthState {
    #[doc(hidden)]
    pub fn new() -> Self {
        Self {
            token: None,
            refresher: None,
            on_unauthorized: None,
            last_refresh_at: None,
        }
    }

    #[doc(hidden)]
    pub fn set_token(&mut self, token: Option<String>) {
        self.token = token;
    }

    #[doc(hidden)]
    pub fn token(&self) -> Option<String> {
        self.token.clone()
    }

    pub(crate) fn set_refresher(&mut self, refresher: Refresher) {
        self.refresher = Some(refresher);
    }

    #[doc(hidden)]
    pub fn set_on_unauthorized(&mut self, f: Box<dyn Fn()>) {
        self.on_unauthorized = Some(Rc::new(f));
    }

    /// 记录一次成功刷新 (用于竞态窗口判定)。
    #[doc(hidden)]
    pub fn mark_refreshed(&mut self) {
        self.last_refresh_at = Some(Instant::now());
    }

    /// `window` 内是否刚成功刷新过。并发 401 时第二路的 refresh 会拿走已被
    /// 轮换的旧 refresh token 而失败, 若据此清登录态会误伤刚刷新成功的会话。
    #[doc(hidden)]
    pub fn recently_refreshed(&self, window: Duration) -> bool {
        self.last_refresh_at
            .map(|t| t.elapsed() < window)
            .unwrap_or(false)
    }

    /// 取出未授权回调的克隆, 供调用方在释放 borrow 之后再触发
    /// (回调内通常会 set_token, 在 borrow 作用域内调用会重入 panic)。
    #[doc(hidden)]
    pub fn cloned_on_unauthorized(&self) -> Option<Rc<dyn Fn()>> {
        self.on_unauthorized.clone()
    }

    /// Invoke the unauthorized hook if one was registered.
    #[doc(hidden)]
    pub fn fire_unauthorized(&self) {
        if let Some(f) = &self.on_unauthorized {
            f();
        }
    }
    /// Produce a refresh future from the registered refresher, keeping it
    /// registered so later 401s can refresh too. `Fn` is callable via `&self`.
    pub(crate) fn refresh(&self) -> Option<TokenFuture> {
        self.refresher.as_ref().map(|f| f())
    }
}

/// Shared auth state handle.
pub(crate) type SharedAuthState = Rc<RefCell<AuthState>>;
