//! OIDC 身份解析：把外部 (provider_slug, subject, email) 映射到本站 auth_users，
//! 支持自动首次登录建号与绑定。整个流程在单个事务里执行，防并发首登重复建号。
//!
//! 查询一律用 `sqlx::query_as` + `FromRow`（**不用 `sqlx::query!` 宏**）：
//! 宏要编译期连库校验（`DATABASE_URL` / `cargo sqlx prepare` 缓存），而本
//! crate 既有代码零宏依赖，引入宏会连带要求建离线缓存。运行时查询在本
//! crate 的既有约定内，且新表（0019）无历史数据形状需要编译期守护。

use crate::error::AuthError;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

/// 解析出的本站用户三元组 —— `jwt::issue` 的直接入参。
#[derive(Debug, Clone, FromRow)]
struct IdentityHit {
    user_key: Uuid,
    role: i16,
    auth_version: i64,
}

/// 解析外部身份为本站用户三元组。
///
/// # 参数
/// - `pool`: 数据库连接池
/// - `provider_slug`: identity_providers 表的 slug（如 "google"）
/// - `subject`: ID token 的 `sub` 字段（OIDC provider 为用户分配的唯一标识）
/// - `email`: 可选，ID token 的 `email` claim
/// - `email_verified`: IdP 是否确认了该邮箱归属。**`false` 时禁止按 email
///   匹配已有账号**——未验证邮箱可被任意人写入 claim，按它 Bind 等于把
///   他人账号拱手让人（账号劫持）。此情形只能走 subject 精确命中或新建账号。
/// - `display_name`: 可选，ID token 的 `name` 或 `preferred_username`
/// - `default_role`: 首次自动建号时赋予的角色（夹到普通用户，见下）
///
/// # 返回
/// `(user_key 文本, role, auth_version)` —— 正是 `jwt::issue` 所需的三元组。
///
/// # 事务语义
/// 整个查找/建号/绑定过程包在 **单个 sqlx 事务** 里，防并发首登时：
/// - 多个 callback 同时到达 → 只有第一个能拿到行锁并建号/绑定
/// - 其余并发请求在事务序列化后复用已建/已绑定的行
///
/// # 并发首登防护
/// 1. `SELECT ... FOR UPDATE` 锁定 `user_identities` (provider_slug, subject) 组合
/// 2. 不存在且 `email_verified` 时再 `SELECT ... FOR UPDATE` 锁定 `auth_users` 按 email 查找行
/// 3. 再次不存在时 `INSERT` 新用户 + `INSERT` user_identities
pub async fn resolve_identity_user(
    pool: &PgPool,
    provider_slug: &str,
    subject: &str,
    email: Option<&str>,
    email_verified: bool,
    display_name: Option<&str>,
    default_role: u16,
) -> Result<(String, u16, i64), AuthError> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await.map_err(AuthError::Db)?;

    // 1) 尝试按 (provider_slug, subject) 直接命中
    if let Some(hit) = try_find_by_subject(&mut tx, provider_slug, subject).await? {
        tx.commit().await.map_err(AuthError::Db)?;
        return Ok(hit.into_tuple());
    }

    // 2) 未命中且邮箱已获 IdP 验证 -> 按 email 查找已有 auth_users 并绑定
    //    （可能是已注册的本地账号，登录即自动关联）
    if email_verified
        && let Some(e) = email
        && let Some(hit) = try_find_by_email_and_bind(&mut tx, provider_slug, subject, e).await?
    {
        tx.commit().await.map_err(AuthError::Db)?;
        return Ok(hit.into_tuple());
    }

    // 3) 都没有 -> 自动建号（auto_create_user 由 provider 配置在调用端控制；
    //    此处 email 仍如实落库，只是不参与「匹配已有账号」）
    let (user_key, role, auth_version) = create_user_and_bind(
        &mut tx,
        provider_slug,
        subject,
        email,
        display_name,
        default_role,
    )
    .await?;

    tx.commit().await.map_err(AuthError::Db)?;
    Ok((user_key, role, auth_version))
}

impl IdentityHit {
    /// 转成 `(user_key 文本, role, auth_version)`。
    ///
    /// `role` 是 SMALLINT(i16)。负值/越界是脏数据，不能 `as u16` 静默放大
    /// （-1 -> 65535 = 越权成 root）；一律按普通用户兜底。
    fn into_tuple(self) -> (String, u16, i64) {
        (
            self.user_key.to_string(),
            u16::try_from(self.role).unwrap_or(1),
            self.auth_version,
        )
    }
}

/// 尝试按 (provider_slug, subject) 查找并更新 last_login_at
async fn try_find_by_subject(
    tx: &mut Transaction<'_, Postgres>,
    provider_slug: &str,
    subject: &str,
) -> Result<Option<IdentityHit>, AuthError> {
    // FOR UPDATE 防止并发建号/绑定竞争
    let row: Option<IdentityHit> = sqlx::query_as(
        r#"
        SELECT ui.user_key, u.role, u.auth_version
        FROM user_identities ui
        JOIN auth_users u ON u.key = ui.user_key
        WHERE ui.provider_slug = $1 AND ui.subject = $2
        FOR UPDATE
        "#,
    )
    .bind(provider_slug)
    .bind(subject)
    .fetch_optional(&mut **tx)
    .await
    .map_err(AuthError::Db)?;

    if let Some(hit) = row {
        sqlx::query(
            "UPDATE user_identities SET last_login_at = now()
             WHERE provider_slug = $1 AND subject = $2",
        )
        .bind(provider_slug)
        .bind(subject)
        .execute(&mut **tx)
        .await
        .map_err(AuthError::Db)?;

        return Ok(Some(hit));
    }
    Ok(None)
}

/// 尝试按 email 查找已有用户并绑定外部身份
async fn try_find_by_email_and_bind(
    tx: &mut Transaction<'_, Postgres>,
    provider_slug: &str,
    subject: &str,
    email: &str,
) -> Result<Option<IdentityHit>, AuthError> {
    // FOR UPDATE 锁定该 email 的 auth_users 行
    let row: Option<IdentityHit> = sqlx::query_as(
        r#"
        SELECT key AS user_key, role, auth_version
        FROM auth_users
        WHERE email = $1
        FOR UPDATE
        "#,
    )
    .bind(email)
    .fetch_optional(&mut **tx)
    .await
    .map_err(AuthError::Db)?;

    if let Some(hit) = row {
        sqlx::query(
            r#"
            INSERT INTO user_identities (provider_slug, subject, user_key, email)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (provider_slug, subject) DO UPDATE SET
                user_key = EXCLUDED.user_key,
                email = EXCLUDED.email,
                last_login_at = now()
            "#,
        )
        .bind(provider_slug)
        .bind(subject)
        .bind(hit.user_key)
        .bind(email)
        .execute(&mut **tx)
        .await
        .map_err(AuthError::Db)?;

        return Ok(Some(hit));
    }
    Ok(None)
}

/// 创建新用户并绑定外部身份（需处理 username 唯一冲突重试）
async fn create_user_and_bind(
    tx: &mut Transaction<'_, Postgres>,
    provider_slug: &str,
    subject: &str,
    email: Option<&str>,
    display_name: Option<&str>,
    default_role: u16,
) -> Result<(String, u16, i64), AuthError> {
    // 取 username 基础：email local-part 或 display_name，过滤掉非法字符
    let base_name = email
        .and_then(|e| e.split('@').next())
        .or(display_name)
        .unwrap_or("oidc_user")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect::<String>();

    let base_name = if base_name.is_empty() {
        "oidc_user".to_string()
    } else {
        base_name
    };

    // default_role 来自 identity_providers 表（admin 可写）。夹到普通用户：
    // provider 配错不该能造出 admin/root 账号；确需提权走管理台显式改用户 role。
    let role = default_role.min(1) as i16;

    // 最多重试 10 次，加数字后缀
    for attempt in 0..10 {
        let username = if attempt == 0 {
            base_name.clone()
        } else {
            format!("{}{}", base_name, attempt + 1)
        };

        let key = Uuid::new_v4();
        // OIDC 用户不走密码登录，password_hash 留空。空串在本 crate 的
        // login 路径不可用（Argon2 校验空串恒失败），不是后门。
        let password_hash = "";

        let res = sqlx::query(
            r#"
            INSERT INTO auth_users (key, username, display_name, email, password_hash,
                                    role, status, quota, used_quota, groups,
                                    auth_version, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, 1, 0, 0, ARRAY['default'], 1, now())
            "#,
        )
        .bind(key)
        .bind(&username)
        .bind(display_name.unwrap_or(&username))
        .bind(email)
        .bind(password_hash)
        .bind(role)
        .execute(&mut **tx)
        .await;

        match res {
            Ok(_) => {
                sqlx::query(
                    r#"
                    INSERT INTO user_identities (provider_slug, subject, user_key, email)
                    VALUES ($1, $2, $3, $4)
                    "#,
                )
                .bind(provider_slug)
                .bind(subject)
                .bind(key)
                .bind(email)
                .execute(&mut **tx)
                .await
                .map_err(AuthError::Db)?;

                return Ok((key.to_string(), role as u16, 1));
            }
            Err(e) => {
                // 唯一冲突 -> 重试（仅 username；email 冲突按逻辑不该发生，
                // 因为上面已按 email 查过一遍）
                if let sqlx::Error::Database(db) = &e
                    && db.code().as_deref() == Some("23505")
                {
                    let constraint = db.constraint().unwrap_or_default();
                    if constraint.contains("username") {
                        continue;
                    }
                    if constraint.contains("email") {
                        return Err(AuthError::EmailTaken);
                    }
                }
                return Err(AuthError::Db(e));
            }
        }
    }

    Err(AuthError::Identity(
        "username conflict after max retries".into(),
    ))
}
