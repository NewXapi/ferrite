//! OIDC 协议流程 —— 授权 URL 构造 + callback 换 token 验 claims。
//!
//! 流程（标准授权码 + PKCE）：
//! ```text
//! authorize: 生成 state/nonce/PKCE → 存 store → 拼授权 URL 让浏览器跳转
//! callback: 凭 state 取回 pending（取出即删）→ code + verifier 换 token
//!           → 验 ID token（签名/iss/aud/exp/nonce）→ 提取 claims
//! ```
//!
//! 安全关键路径（JWKS 轮换、nonce/state 校验、PKCE、签名验证）全部委托
//! `openidconnect` crate，**本模块不手写任何密码学验证**。

use openidconnect::core::{
    CoreAuthenticationFlow, CoreClient, CoreIdToken, CoreIdTokenClaims, CoreProviderMetadata,
    CoreTokenResponse,
};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope,
};

use crate::provider::{IdentityProvider, OidcError};
use crate::store::{AuthStateStore, new_state};

/// 验过的 ID token claims —— 调用方据此解析本站用户。
///
/// 只保留解析用户必需的字段；原始 claims 不往上传。
#[derive(Debug, Clone)]
pub struct VerifiedClaims {
    /// ID token 的 `sub` —— provider 内唯一的用户标识。
    pub subject: String,
    /// `email` claim；缺失时调用方需按 display_name 建号。
    pub email: Option<String>,
    /// `email_verified` claim。false 时邮箱未经 IdP 确认，
    /// 自动建号链路应视为不可信（调用方决定是否仍按邮箱匹配老账号）。
    pub email_verified: bool,
    /// `name` claim。
    pub name: Option<String>,
    /// `preferred_username` claim。
    pub preferred_username: Option<String>,
}

/// 构造授权跳转 URL。
///
/// 生成 state/nonce/PKCE verifier 落库，返回完整授权 URL（含
/// `response_type=code`、client_id、redirect_uri、scope、state、nonce、
/// PKCE challenge）。调用方应把浏览器 302 到这个 URL。
///
/// - `provider`: 目标 provider（须 enabled）
/// - `store`: state 存储
/// - `redirect_after`: 登录成功后要跳转的前端相对路径（可为 `None`）
/// - `ttl_secs`: state 有效期
pub async fn authorize_url(
    provider: &IdentityProvider,
    store: &AuthStateStore,
    redirect_after: Option<String>,
    ttl_secs: u64,
) -> Result<String, OidcError> {
    let (state, nonce, pkce_verifier) = new_state();
    store
        .insert(
            &state,
            &provider.slug,
            &nonce,
            &pkce_verifier,
            redirect_after.as_deref(),
            ttl_secs,
        )
        .await?;

    let client = build_client(provider).await?;
    let (auth_url, _, _) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            || openidconnect::CsrfToken::new(state.clone()),
            || Nonce::new(nonce.clone()),
        )
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(PkceCodeChallenge::from(PkceCodeVerifier::new(
            pkce_verifier,
        )))
        .url();

    Ok(auth_url.to_string())
}

/// callback：code 换 token，验 ID token，提取 claims。
///
/// 校验顺序（任一失败即拒）：
/// 1. 凭 `state` 取回 pending —— 取出即删，重放/过期/CSRF 全部落空
/// 2. `code` + PKCE verifier 换 token
/// 3. ID token 验签（JWKS）+ `iss` == provider.issuer_url
///    + `aud` 含 client_id + `exp` 未过期 + `nonce` == pending.nonce
///
/// - `code`: 授权回调带回的授权码
/// - `state`: 授权回调带回的 state
pub async fn exchange_and_verify(
    provider: &IdentityProvider,
    store: &AuthStateStore,
    code: &str,
    state: &str,
) -> Result<VerifiedClaims, OidcError> {
    let pending = store
        .take(state)
        .await?
        .ok_or_else(|| OidcError::State("state not found or expired".into()))?;
    if pending.provider_slug != provider.slug {
        return Err(OidcError::State("state belongs to another provider".into()));
    }

    let client = build_client(provider).await?;
    let token_response = client
        .exchange_code(AuthorizationCode::new(code.to_string()))?
        .set_pkce_verifier(PkceCodeVerifier::new(pending.pkce_verifier))
        .request_async(async_http_client)
        .await
        .map_err(|e| OidcError::Token(e.to_string()))?;

    let id_token: &CoreIdToken = token_response
        .id_token()
        .ok_or_else(|| OidcError::Verification("no id_token in response".into()))?;

    let claims: &CoreIdTokenClaims = id_token
        .claims(&client.id_token_verifier(), &Nonce::new(pending.nonce))
        .map_err(|e| OidcError::Verification(e.to_string()))?;

    Ok(VerifiedClaims {
        subject: claims.subject().to_string(),
        email: claims.email().map(|e| e.to_string()),
        email_verified: claims.email_verified().unwrap_or(false),
        name: claims
            .name()
            .and_then(|n| n.get(None))
            .map(|n| n.to_string()),
        preferred_username: claims.preferred_username().map(|u| u.to_string()),
    })
}

/// 建 `CoreClient` —— 拉 discovery document 填 endpoints。
///
/// 每次调用都重新 discover（不缓存 metadata）：登录是低频操作，
/// 而 discovery 让 JWKS 轮换、端点变更自动生效，缓存反而要管失效。
async fn build_client(provider: &IdentityProvider) -> Result<CoreClient, OidcError> {
    let issuer = IssuerUrl::new(provider.issuer_url.clone())
        .map_err(|e| OidcError::Discovery(e.to_string()))?;
    let metadata = CoreProviderMetadata::discover_async(issuer, async_http_client)
        .await
        .map_err(|e| OidcError::Discovery(e.to_string()))?;

    CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(provider.client_id.clone()),
        Some(ClientSecret::new(provider.client_secret.clone())),
    )
    .set_redirect_uri(
        RedirectUrl::new(provider.redirect_uri.clone())
            .map_err(|e| OidcError::Discovery(e.to_string()))?,
    )
    .ok_or_else(|| OidcError::Discovery("client build failed".into()))
}

/// `openidconnect` 的 async HTTP client 适配 —— 用 workspace 的 reqwest。
///
/// 签名由 crate 的 `SetHttpClient` trait 约束；返回 `Result` 而非
/// `impl Future`，便于 `request_async` 直接传入。
fn async_http_client(
    request: openidconnect::HttpRequest,
) -> impl Future<Output = Result<openidconnect::HttpResponse, String>> + Send {
    async move {
        let method = reqwest::Method::from_bytes(request.method().as_str().as_bytes())
            .map_err(|e| e.to_string())?;
        let mut req = reqwest::Client::new().request(method, request.uri().to_string());
        for (name, value) in request.headers().iter() {
            req = req.header(name.as_str(), value.as_bytes());
        }
        let response = req
            .body(request.body().to_vec())
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                v.to_str()
                    .ok()
                    .map(|val| (k.as_str().to_string(), val.to_string()))
            })
            .collect();
        let body = response.bytes().await.map_err(|e| e.to_string())?.to_vec();
        Ok(openidconnect::HttpResponse {
            status_code: status.into(),
            headers,
            body,
        })
    }
}
