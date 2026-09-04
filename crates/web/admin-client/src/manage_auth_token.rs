use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

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
    on_unauthorized: Option<Box<dyn Fn()>>,
}

impl AuthState {
    #[doc(hidden)]
    pub fn new() -> Self {
        Self {
            token: None,
            refresher: None,
            on_unauthorized: None,
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
        self.on_unauthorized = Some(f);
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
