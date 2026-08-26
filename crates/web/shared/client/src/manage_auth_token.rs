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
pub(crate) struct AuthState {
    token: Option<String>,
    refresher: Option<Refresher>,
    on_unauthorized: Option<Box<dyn Fn()>>,
}

impl AuthState {
    pub(crate) fn new() -> Self {
        Self {
            token: None,
            refresher: None,
            on_unauthorized: None,
        }
    }

    pub(crate) fn set_token(&mut self, token: Option<String>) {
        self.token = token;
    }

    pub(crate) fn token(&self) -> Option<String> {
        self.token.clone()
    }

    pub(crate) fn set_refresher(&mut self, refresher: Refresher) {
        self.refresher = Some(refresher);
    }

    pub(crate) fn set_on_unauthorized(&mut self, f: Box<dyn Fn()>) {
        self.on_unauthorized = Some(f);
    }

    /// Invoke the unauthorized hook if one was registered.
    pub(crate) fn fire_unauthorized(&self) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn token_roundtrip() {
        let mut state = AuthState::new();
        assert_eq!(state.token(), None);
        state.set_token(Some("abc".to_string()));
        assert_eq!(state.token(), Some("abc".to_string()));
        state.set_token(None);
        assert_eq!(state.token(), None);
    }

    #[test]
    fn fire_unauthorized_calls_hook() {
        let fired = Rc::new(Cell::new(false));
        let mut state = AuthState::new();
        let fired_clone = fired.clone();
        state.set_on_unauthorized(Box::new(move || fired_clone.set(true)));
        assert!(!fired.get());
        state.fire_unauthorized();
        assert!(fired.get());
    }
}
