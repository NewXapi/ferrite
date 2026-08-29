use client::{ApiClient, ApiError, ApiResult};
use serde::{Deserialize, Serialize};

use crate::{manage_session::apply_bundle, AuthBundle};

/// Outcome of a password login: either fully authenticated, or a 2FA challenge.
#[derive(Debug, Clone, PartialEq)]
pub enum LoginOutcome {
    /// Login completed; tokens and user are now in the session.
    Authenticated(AuthBundle),
    /// Account requires 2FA; the user must submit a code before `expires_at`.
    Require2fa { flow_token: String, expires_at: i64 },
}

/// Raw envelope `data` for a login response — covers both the 2FA challenge
/// branch and the full auth bundle branch with all-`Option` fields.
#[derive(Debug, Default, Deserialize)]
struct LoginData {
    require_2fa: Option<bool>,
    flow_token: Option<String>,
    expires_at: Option<i64>,
    access_token: Option<String>,
    token_type: Option<String>,
    access_expires_at: Option<i64>,
    session: Option<crate::SessionInfo>,
    user: Option<crate::User>,
}

/// Map a decoded `LoginData` into a `LoginOutcome`.
///
/// Pure: no I/O and no global-signal access, so host tests can exercise it
/// without a dioxus runtime.
fn outcome_from_data(data: LoginData) -> ApiResult<LoginOutcome> {
    if data.require_2fa == Some(true) {
        match (data.flow_token, data.expires_at) {
            (Some(flow_token), Some(expires_at)) => Ok(LoginOutcome::Require2fa {
                flow_token,
                expires_at,
            }),
            _ => Err(ApiError::Decode("missing 2FA flow fields".into())),
        }
    } else {
        match (
            data.access_token,
            data.token_type,
            data.access_expires_at,
            data.session,
            data.user,
        ) {
            (
                Some(access_token),
                Some(token_type),
                Some(access_expires_at),
                Some(session),
                Some(user),
            ) => Ok(LoginOutcome::Authenticated(AuthBundle {
                access_token,
                token_type,
                access_expires_at,
                session,
                user,
            })),
            _ => Err(ApiError::Decode("missing auth bundle fields".into())),
        }
    }
}

/// Request body for `POST /api/user/login`.
#[derive(Serialize)]
struct LoginRequest<'a> {
    username: &'a str,
    password: &'a str,
}

/// Submit credentials to `POST /api/user/login`.
///
/// On a non-2FA success the returned bundle is applied to the global session
/// signal before this function returns.
pub async fn login(client: &ApiClient, username: &str, password: &str) -> ApiResult<LoginOutcome> {
    let data: LoginData = client
        .post("/api/user/login", &LoginRequest { username, password })
        .await?;
    match outcome_from_data(data)? {
        LoginOutcome::Authenticated(bundle) => {
            apply_bundle(bundle.clone());
            Ok(LoginOutcome::Authenticated(bundle))
        }
        two_fa => Ok(two_fa),
    }
}

/// Request body for `POST /api/user/login/2fa`.
#[derive(Serialize)]
struct TwoFaRequest<'a> {
    flow_token: &'a str,
    code: &'a str,
}

/// Submit a 2FA code to `POST /api/user/login/2fa`.
///
/// On success the returned bundle is applied to the global session signal.
pub async fn verify_2fa(client: &ApiClient, flow_token: &str, code: &str) -> ApiResult<AuthBundle> {
    let bundle: AuthBundle = client
        .post("/api/user/login/2fa", &TwoFaRequest { flow_token, code })
        .await?;
    apply_bundle(bundle.clone());
    Ok(bundle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::User;
    use serde_json;
    #[test]
    fn user_decodes_full_dto_ignoring_unknowns() {
        let json = r#"{
            "id": 42,
            "username": "alice",
            "display_name": "Alice L",
            "role": 100,
            "status": 1,
            "email": "alice@example.com",
            "group": "default",
            "quota": 1000000,
            "used_quota": 500,
            "request_count": 7,
            "permissions": 0,
            "github_id": ""
        }"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.id, 42);
        assert_eq!(user.username, "alice");
        assert_eq!(user.display_name, "Alice L");
        assert_eq!(user.role, 100);
        assert_eq!(user.status, 1);
        assert_eq!(user.email, "alice@example.com");
        assert_eq!(user.group, "default");
        assert_eq!(user.quota, 1_000_000);
        assert_eq!(user.used_quota, 500);
        assert_eq!(user.request_count, 7);
    }

    #[test]
    fn session_info_captures_sid_ignoring_extras() {
        let json = r#"{
            "sid": "sess-abc",
            "current": true,
            "login_method": "password",
            "ip": "127.0.0.1"
        }"#;
        let session: crate::SessionInfo = serde_json::from_str(json).unwrap();
        assert_eq!(session.sid, "sess-abc");
    }

    #[test]
    fn outcome_from_two_fa_json() {
        let json = r#"{
            "require_2fa": true,
            "flow_token": "flow-xyz",
            "expires_at": 1700000000
        }"#;
        let data: LoginData = serde_json::from_str(json).unwrap();
        let outcome = outcome_from_data(data).unwrap();
        match outcome {
            LoginOutcome::Require2fa {
                flow_token,
                expires_at,
            } => {
                assert_eq!(flow_token, "flow-xyz");
                assert_eq!(expires_at, 1_700_000_000);
            }
            _ => panic!("expected Require2fa"),
        }
    }

    #[test]
    fn outcome_from_bundle_json() {
        let json = r#"{
            "access_token": "tok-123",
            "token_type": "Bearer",
            "access_expires_at": 1700000000,
            "session": { "sid": "sess-1" },
            "user": {
                "id": 9,
                "username": "bob",
                "display_name": "Bob",
                "role": 1,
                "status": 1,
                "email": "bob@x.com",
                "group": "g",
                "quota": 0,
                "used_quota": 0,
                "request_count": 0
            }
        }"#;
        let data: LoginData = serde_json::from_str(json).unwrap();
        let outcome = outcome_from_data(data).unwrap();
        match outcome {
            LoginOutcome::Authenticated(bundle) => {
                assert_eq!(bundle.access_token, "tok-123");
                assert_eq!(bundle.user.id, 9);
            }
            _ => panic!("expected Authenticated"),
        }
    }

    #[test]
    fn outcome_from_two_fa_without_flow_token_errors() {
        let json = r#"{ "require_2fa": true, "expires_at": 1700000000 }"#;
        let data: LoginData = serde_json::from_str(json).unwrap();
        match outcome_from_data(data) {
            Err(ApiError::Decode(_)) => {}
            other => panic!("expected Decode error, got {other:?}"),
        }
    }
}
