use axum::http::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("username taken")]
    UsernameTaken,

    #[error("email taken")]
    EmailTaken,

    #[error("token expired or invalid")]
    InvalidToken,

    #[error("user disabled")]
    UserDisabled,

    #[error("user not found")]
    UserNotFound,

    #[error("missing JWT secret")]
    MissingSecret,

    #[error("forbidden: admin required")]
    Forbidden,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("db error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("jwt error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl AuthError {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::InvalidCredentials | Self::InvalidToken => StatusCode::UNAUTHORIZED,
            Self::UserDisabled => StatusCode::FORBIDDEN,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::UsernameTaken | Self::EmailTaken | Self::Conflict(_) => StatusCode::CONFLICT,
            Self::UserNotFound => StatusCode::NOT_FOUND,
            Self::MissingSecret => StatusCode::INTERNAL_SERVER_ERROR,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Db(_) | Self::Crypto(_) | Self::Jwt(_) | Self::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::NotFound(_) => StatusCode::NOT_FOUND,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidCredentials => "INVALID_CREDENTIALS",
            Self::InvalidToken => "INVALID_TOKEN",
            Self::UserDisabled => "USER_DISABLED",
            Self::UsernameTaken => "USERNAME_TAKEN",
            Self::EmailTaken => "EMAIL_TAKEN",
            Self::UserNotFound => "USER_NOT_FOUND",
            Self::Forbidden => "FORBIDDEN",
            Self::Conflict(_) => "CONFLICT",
            Self::MissingSecret => "MISSING_JWT_SECRET",
            Self::BadRequest(_) => "BAD_REQUEST",
            Self::Db(_) => "DB_ERROR",
            Self::Crypto(_) => "CRYPTO_ERROR",
            Self::Jwt(_) => "JWT_ERROR",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }
}