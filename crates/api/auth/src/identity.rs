//! OIDC 身份解析：把外部 (provider_slug, subject, email) 映射到本站 auth_users，
//! 支持自动首次登录建号与绑定。整个流程在单个事务里执行，防并发首登重复建号。

use crate::error::AuthError;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

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
/// - `default_role`: 首次自动建号时赋予的角色（默认 1 = 普通用户）
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
    if let Some((user_key, role, auth_version)) = try_find_by_subject(&mut tx, provider_slug, subject).await? {
        tx.commit().await.map_err(AuthError::Db)?;
        return Ok((user_key, role, auth_version));
    }

    // 2) 未命中且邮箱已获 IdP 验证 -> 按 email 查找已有 auth_users 并绑定
    //    （可能是已注册的本地账号，登录即自动关联）
    if email_verified && let Some(e) = email {
        if let Some((user_key, role, auth_version)) = try_find_by_email_and_bind(&mut tx, provider_slug, subject, e).await? {
            tx.commit().await.map_err(AuthError::Db)?;
            return Ok((user_key, role, auth_version));
        }
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

/// 尝试按 (provider_slug, subject) 查找并更新 last_login_at
async fn try_find_by_subject(
    tx: &mut Transaction<'_, Postgres>,
    provider_slug: &str,
    subject: &str,
) -> Result<Option<(String, u16, i64)>, AuthError> {
    // FOR UPDATE 防止并发建号/绑定竞争
    let row = sqlx::query!(
        r#"
        SELECT ui.user_key, u.role, u.auth_version
        FROM user_identities ui
        JOIN auth_users u ON u.key = ui.user_key
        WHERE ui.provider_slug = $1 AND ui.subject = $2
        FOR UPDATE
        "#,
        provider_slug,
        subject
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(AuthError::Db)?;

    if let Some(row) = row {
        // 更新 last_login_at
        sqlx::query!(
            r#"UPDATE user_identities SET last_login_at = now() WHERE provider_slug = $1 AND subject = $2"#,
            provider_slug,
            subject
        )
        .execute(&mut **tx)
        .await
        .map_err(AuthError::Db)?;

        return Ok(Some((
            row.user_key.to_string(),
            // role 是 SMALLINT(i16)。负值/越界值是脏数据，不能 `as u16`
            // 静默放大（-1 -> 65535 = 越权成 root）；一律按普通用户兜底。
            u16::try_from(row.role).unwrap_or(1),
            row.auth_version,
        )));
    }
    Ok(None)
}

/// 尝试按 email 查找已有用户并绑定外部身份
async fn try_find_by_email_and_bind(
    tx: &mut Transaction<'_, Postgres>,
    provider_slug: &str,
    subject: &str,
    email: &str,
) -> Result<Option<(String, u16, i64)>, AuthError> {
    // FOR UPDATE 锁定该 email 的 auth_users 行
    let row = sqlx::query!(
        r#"
        SELECT key, role, auth_version
        FROM auth_users
        WHERE email = $1
        FOR UPDATE
        "#,
        email
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(AuthError::Db)?;

    if let Some(user) = row {
        // 绑定外部身份
        sqlx::query!(
            r#"
            INSERT INTO user_identities (provider_slug, subject, user_key, email)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (provider_slug, subject) DO UPDATE SET
                user_key = EXCLUDED.user_key,
                email = EXCLUDED.email,
                last_login_at = now()
            "#,
            provider_slug,
            subject,
            user.key,
            email
        )
        .execute(&mut **tx)
        .await
        .map_err(AuthError::Db)?;

        return Ok(Some((
            user.key.to_string(),
            // 同 try_find_by_subject：负值不静默放大成高权限。
            u16::try_from(user.role).unwrap_or(1),
            user.auth_version,
        )));
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
    // 取 username 基础：email local-part 或 display_name
    let base_name = email
        .and_then(|e| e.split('@').next())
        .or(display_name)
        .unwrap_or("oidc_user")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect::<String>();

    let base_name = if base_name.is_empty() { "oidc_user".to_string() } else { base_name };

    // 最多重试 10 次，加数字后缀
    for attempt in 0..10 {
        let username = if attempt == 0 {
            base_name.clone()
        } else {
            format!("{}{}", base_name, attempt + 1)
        };

        let key = Uuid::new_v4();
        let now = chrono::Utc::now();
        let password_hash = ""; // OIDC 用户不走密码登录，留空不可用

        let res = sqlx::query!(
            r#"
            INSERT INTO auth_users (key, username, display_name, email, password_hash,
                                    role, status, quota, used_quota, groups,
                                    auth_version, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, 1, 0, 0, ARRAY['default'], 1, $7)
            "#,
            key,
            username,
            display_name.unwrap_or(&username),
            email,
            password_hash,
            default_role as i16,
            now
        )
        .execute(&mut **tx)
        .await;

        match res {
            Ok(_) => {
                // 建号成功，绑定外部身份
                sqlx::query!(
                    r#"
                    INSERT INTO user_identities (provider_slug, subject, user_key, email)
                    VALUES ($1, $2, $3, $4)
                    "#,
                    provider_slug,
                    subject,
                    key,
                    email
                )
                .execute(&mut **tx)
                .await
                .map_err(AuthError::Db)?;

                return Ok((
                    key.to_string(),
                    // default_role 来自 identity_providers 表（admin 可写）。
                    // 夹到普通用户：provider 配错不该能造出 admin/root 账号；
                    // 确需提权走管理台显式改该用户的 role。
                    default_role.min(1),
                    1, // auth_version 默认 1
                ));
            }
            Err(e) => {
                // 唯一冲突 -> 重试
                if let sqlx::Error::Database(db) = &e
                    && db.code().as_deref() == Some("23505")
                {
                    let constraint = db.constraint().unwrap_or_default();
                    // 仅 username 冲突时重试；email 冲突不应发生（前面已按 email 查过）
                    if constraint.contains("username") {
                        continue;
                    }
                    // email 冲突按逻辑不该发生，但若发生则报错
                    if constraint.contains("email") {
                        return Err(AuthError::EmailTaken);
                    }
                }
                return Err(AuthError::Db(e));
            }
        }
    }

    // 超过重试次数
    Err(AuthError::Identity("username conflict after max retries".into()))
}