//! AuthError → HTTP 状态映射的不变量:
//! 所有"凭证/会话无效"类错误必须映射 401, 前端的 401 静默刷新链依赖该语义
//! (曾因 Jwt(_) 归入 500 导致无效 token 整页 500 且刷新链失效)。

use auth::AuthError;
use axum::http::StatusCode;

#[test]
fn auth_failures_map_to_401() {
    // JWT 解析失败 (伪造/损坏的 Bearer) → Jwt(_) → 必须 401
    let jwt_err = jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidToken);
    assert_eq!(
        AuthError::Jwt(jwt_err).status(),
        StatusCode::UNAUTHORIZED,
        "无效 JWT 必须 401, 否则前端刷新链永远不触发"
    );
    assert_eq!(AuthError::InvalidToken.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        AuthError::InvalidCredentials.status(),
        StatusCode::UNAUTHORIZED
    );
}

#[test]
fn server_faults_stay_500() {
    assert_eq!(
        AuthError::Db(sqlx::Error::RowNotFound).status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        AuthError::Internal("boom".into()).status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}
