//! Token refresh flow (`POST /api/user/auth/refresh`).

use client::{ApiClient, ApiResult};
use dioxus::prelude::ReadableExt;

use crate::{AuthBundle, manage_session::apply_bundle};

/// Refresh the access token via `POST /api/user/auth/refresh`.
///
/// The httpOnly session cookie drives the refresh server-side; the current
/// `sid` (when known) is sent as `X-Auth-Session` so the backend picks the
/// right session. Uses `post_once` so a failing refresh never recurses through
/// the client's own 401 retry loop. On success the bundle is applied to the
/// global session signal.
// ponytail: proactive pre-expiry refresh timer skipped — bootstrap refresh on
// app start is page-crate work; add a background timer when needed.
pub async fn refresh(client: &ApiClient) -> ApiResult<AuthBundle> {
    // Clone the sid out of the signal before awaiting so the ReadableRef
    // borrow is released before the network call.
    let sid_owned: Option<String> = {
        let session = crate::SESSION.read();
        session.sid.clone()
    };
    let headers: Option<Vec<(&str, &str)>> = sid_owned
        .as_ref()
        .map(|sid| vec![("X-Auth-Session", sid.as_str())]);
    let headers_ref = headers.as_ref().map(|h| h.as_slice());

    let bundle: AuthBundle = client
        .post_once("/api/user/auth/refresh", &(), headers_ref)
        .await?;
    apply_bundle(bundle.clone());
    Ok(bundle)
}
