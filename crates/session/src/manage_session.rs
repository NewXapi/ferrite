//! Global session signal: the single source of truth for login state.

use dioxus::prelude::*;

use crate::{refresh_token::refresh, AuthBundle};
use client::ApiClient;

/// Snapshot of the authenticated identity shared across all pages.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SessionState {
    pub user: Option<crate::User>,
    pub access_token: Option<String>,
    pub access_expires_at: Option<i64>,
    pub sid: Option<String>,
}

impl SessionState {
    /// Logged in iff both user and access token are present.
    pub fn is_authenticated(&self) -> bool {
        self.user.is_some() && self.access_token.is_some()
    }

    /// Token expired iff there is no expiry or it has already passed.
    pub fn is_expired(&self, now_unix: i64) -> bool {
        match self.access_expires_at {
            Some(exp) => exp <= now_unix,
            None => true,
        }
    }
}

/// Global signal every page reads to know the login state.
pub static SESSION: GlobalSignal<SessionState> = Signal::global(SessionState::default);

/// Apply a fresh auth bundle to the global session signal.
pub fn apply_bundle(bundle: AuthBundle) {
    SESSION.with_mut(|s| {
        *s = SessionState {
            user: Some(bundle.user),
            access_token: Some(bundle.access_token),
            access_expires_at: Some(bundle.access_expires_at),
            sid: Some(bundle.session.sid),
        };
    });
}

/// Clear all login state back to the default (logged out).
pub fn clear_session() {
    SESSION.with_mut(|s| *s = SessionState::default());
}

/// Wire the client's 401-refresh and on-unauthorized hooks once at app start.
pub fn init(client: &ApiClient) {
    let refreshed = client.clone();
    client.set_refresher(move || {
        let c = refreshed.clone();
        Box::pin(async move {
            match refresh(&c).await {
                Ok(bundle) => Some(bundle.access_token),
                Err(_) => None,
            }
        })
    });
    client.set_on_unauthorized(clear_session);
}

/// Log out: best-effort server call then always clear local state.
///
/// `logout.rs` from the V3 sketch is merged here because logout is a state
/// transition, not a separate concern.
pub async fn logout(client: &ApiClient) {
    // Clone the sid out of the signal before awaiting so the ReadableRef
    // borrow is released before the network call.
    let sid_owned: Option<String> = {
        let session = SESSION.read();
        session.sid.clone()
    };
    let headers: Option<Vec<(&str, &str)>> = sid_owned
        .as_ref()
        .map(|sid| vec![("X-Auth-Session", sid.as_str())]);
    let headers_ref = headers.as_ref().map(|h| h.as_slice());

    // Best-effort: ignore the result so we always clear below.
    let _ = client
        .post_once::<(), ()>("/api/user/auth/logout", &(), headers_ref)
        .await;

    clear_session();
    client.set_token(None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_authenticated_none_when_missing() {
        let state = SessionState::default();
        assert!(!state.is_authenticated());

        let state = SessionState {
            user: Some(crate::User {
                id: 1,
                username: "x".into(),
                display_name: "X".into(),
                role: 1,
                status: 1,
                email: String::new(),
                group: String::new(),
                quota: 0,
                used_quota: 0,
                request_count: 0,
            }),
            access_token: None,
            access_expires_at: None,
            sid: None,
        };
        assert!(!state.is_authenticated());
    }

    #[test]
    fn is_authenticated_true_when_both_present() {
        let state = SessionState {
            user: Some(crate::User {
                id: 1,
                username: "x".into(),
                display_name: "X".into(),
                role: 1,
                status: 1,
                email: String::new(),
                group: String::new(),
                quota: 0,
                used_quota: 0,
                request_count: 0,
            }),
            access_token: Some("tok".into()),
            access_expires_at: Some(100),
            sid: Some("s".into()),
        };
        assert!(state.is_authenticated());
    }

    #[test]
    fn is_expired_none_is_true() {
        let state = SessionState::default();
        assert!(state.is_expired(0));
    }

    #[test]
    fn is_expired_past_is_true() {
        let state = SessionState {
            access_expires_at: Some(100),
            ..Default::default()
        };
        assert!(state.is_expired(100));
        assert!(state.is_expired(200));
    }

    #[test]
    fn is_expired_future_is_false() {
        let state = SessionState {
            access_expires_at: Some(100),
            ..Default::default()
        };
        assert!(!state.is_expired(50));
    }
}
