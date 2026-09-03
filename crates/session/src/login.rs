use client::{ApiClient, ApiError, ApiResult};
use serde::{Deserialize, Serialize};

use crate::{AuthBundle, manage_session::apply_bundle};

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
#[doc(hidden)]
pub struct LoginData {
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
#[doc(hidden)]
pub fn outcome_from_data(data: LoginData) -> ApiResult<LoginOutcome> {
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
