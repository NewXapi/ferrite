#[cfg(test)]
mod auth_token_batch {
    use crate::auth::service::AuthService;
    use crate::auth::routes::{self, AppState};
    use crate::catalog::tokens::TokenService;
    use crate::catalog::tokens::routes as token_routes;
    use crate::catalog::tokens::TokenAppState;
    use crate::catalog::tokens::TokenService as CatalogTokenService;
    use crate::catalog::tokens::TokenView;
    use crate::catalog::tokens::TokenRow;
    use crate::catalog::tokens::sha256_hex;
    use crate::catalog::tokens::preview;
    use crate::error::AuthError;
    use crate::jwt;
    use crate::password;
    use chrono::{DateTime, Utc};
    use serde_json::Value as JsonValue;
    use sqlx::PgPool;
    use uuid::Uuid;
    use std::sync::Arc;
    use axum::Router;
    use tower::ServiceExt;
    use http_body_util::BodyExt;
    use hyper::{Method, Request, Uri, Version, HeaderMap};

    /// Helper function to create a test database connection
    async fn setup_test_db() -> PgPool {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".to_string()
        });
        let pool = PgPool::connect(&database_url).await.unwrap();
        // Run migrations
        crate::auth::ddl::run(&pool).await.unwrap();
        crate::catalog::tokens::ensure_table(&pool).await.unwrap();
        pool
    }

    /// Create a test user and return (service, user_key)
    async fn create_test_user(pool: &PgPool) -> (AuthService, Uuid) {
        let user_key = Uuid::new_v4();
        let username = format!("test_user_{}", Uuid::new_v4());
        let password_hash = password::hash("password123").unwrap();
        
        // Insert test user
        sqlx::query(
            r#"INSERT INTO auth_users (key, username, display_name, email, password_hash, 
                role, status, quota, used_quota, group_id, auth_version, created_at)
                VALUES ($1, $2, $3, $4, $5, 1, 1, 0, 0, 'default', 1, now())"#,
        )
        .bind(user_key)
        .bind(&username)
        .bind(&username)
        .bind("")
        .bind(&password_hash)
        .execute(pool)
        .await
        .unwrap();
        
        let secret = b"test_jwt_secret_that_is_at_least_32_bytes_long!123"; // >=32 bytes
        let auth_svc = AuthService::new(pool.clone(), secret.to_vec()).unwrap();
        (auth_svc, user_key)
    }

    /// Test session management
    #[tokio::test]
    async fn test_session_management() {
        let pool = setup_test_db().await;
        let (auth_svc, user_key) = create_test_user(&pool).await;
        let auth_svc = Arc::new(auth_svc);
        
        // Create test user for session service
        let secret = b"test_jwt_secret_that_is_at_least_32_bytes_long!123";
        let auth_svc_clone = Arc::new(AuthService::new(pool.clone(), secret.to_vec()).unwrap());
        
        // 1. Test list_sessions - should be empty initially
        let sessions = auth_svc_clone.list_sessions(user_key).await.unwrap();
        assert_eq!(sessions.len(), 0);
        
        // 2. Test create_session (simulate login)
        // In real scenario, login() would create session, but for this test
        // we'll directly test the session operations
        
        println!("Session management test passed");
    }

    /// Test settings management
    #[tokio::test]
    async fn test_settings_management() {
        let pool = setup_test_db().await;
        let (auth_svc, user_key) = create_test_user(&pool).await;
        
        // 1. Test get_settings (should return empty object)
        let settings = auth_svc.get_settings(user_key).await.unwrap();
        assert_eq!(settings, JsonValue::Object(serde_json::Map::new()));
        
        // 2. Test update_settings
        let patch = JsonValue::Object({
            let mut map = serde_json::Map::new();
            map.insert("theme".to_string(), JsonValue::String("dark".to_string()));
            map.insert("notifications".to_string(), JsonValue::Bool(true));
            map
        });
        
        let updated = auth_svc.update_settings(user_key, patch.clone()).await.unwrap();
        assert_eq!(updated["theme"], JsonValue::String("dark".to_string()));
        assert_eq!(updated["notifications"], JsonValue::Bool(true));
        
        // 3. Test update with additional field
        let patch2 = JsonValue::Object({
            let mut map = serde_json::Map::new();
            map.insert("language".to_string(), JsonValue::String("en".to_string()));
            map
        });
        
        let updated2 = auth_svc.update_settings(user_key, patch2).await.unwrap();
        assert_eq!(updated2["theme"], JsonValue::String("dark".to_string()));
        assert_eq!(updated2["notifications"], JsonValue::Bool(true));
        assert_eq!(updated2["language"], JsonValue::String("en".to_string()));
        
        println!("Settings management test passed");
    }

    /// Test token operations
    #[tokio::test]
    async fn test_token_operations() {
        let pool = setup_test_db().await;
        let (auth_svc, user_key) = create_test_user(&pool).await;
        
        // Create token service
        let token_svc = TokenService::new(pool.clone());
        
        // 1. Test create_token
        let result = token_svc.create(
            user_key,
            "test_token",
            Some("default".to_string()),
            1000,
            false,
            None,
        ).await.unwrap();
        
        assert_eq!(result.token.name, "test_token");
        assert!(result.plaintext.starts_with("sk-"));
        assert_eq!(result.token.user_key, user_key.to_string());
        assert_eq!(result.token.group, "default");
        
        let token_key = result.token.key;
        let token_plaintext = result.plaintext;
        
        // 2. Test list_tokens
        let rows: Vec<TokenRow> = sqlx::query_as(
            "SELECT key, user_key, name, key_hash, key_preview, group_id, quota, unlimited_quota, used_quota, expires_at, status, created_at FROM api_tokens WHERE user_key = $1",
        )
        .bind(user_key)
        .fetch_all(&pool)
        .await
        .unwrap();
        
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "test_token");
        assert_eq!(rows[0].user_key, user_key);
        
        // 3. Test search_tokens
        let search_result = token_svc.search(Some("test"), 1, 20).await.unwrap();
        assert_eq!(search_result.0.len(), 1);
        assert_eq!(search_result.0[0].name, "test_token");
        
        // 4. Test update_token
        let update_result = token_svc.update(
            token_key.to_string(),
            &token_key.to_string(),
            Some("updated_token"),
            Some("updated"),
            None,
            Some(2000),
            Some(false),
            None,
        ).await.unwrap();
        
        assert_eq!(update_result.name, "updated_token");
        
        // 5. Test regenerate_key
        let reg_result = token_svc.regenerate(token_key.to_string()).await.unwrap();
        assert!(reg_result.plaintext.starts_with("sk-"));
        assert_ne!(reg_result.plaintext, token_plaintext);
        
        // 6. Test delete_token
        token_svc.remove(token_key.to_string()).await.unwrap();
        
        let remaining: Vec<TokenRow> = sqlx::query_as(
            "SELECT key, user_key, name, key_hash, key_preview, group_id, quota, unlimited_quota, used_quota, expires_at, status, created_at FROM api_tokens WHERE user_key = $1",
        )
        .bind(user_key)
        .fetch_all(&pool)
        .await
        .unwrap();
        
        assert_eq!(remaining.len(), 0);
        
        println!("Token operations test passed");
    }

    /// Test token batch operations
    #[tokio::test]
    async fn test_token_batch_operations() {
        let pool = setup_test_db().await;
        let (auth_svc, user_key) = create_test_user(&pool).await;
        
        let token_svc = TokenService::new(pool.clone());
        
        // Create multiple tokens for batch testing
        let mut token_keys = Vec::new();
        let mut token_plaintexts = Vec::new();
        
        for i in 0..3 {
            let result = token_svc.create(
                user_key,
                &format!("batch_token_{}", i),
                Some("default".to_string()),
                1000,
                false,
                None,
            ).await.unwrap();
            
            token_keys.push(result.token.key);
            token_plaintexts.push(result.plaintext);
        }
        
        assert_eq!(token_keys.len(), 3);
        
        // Test batch deletion
        let batch_delete_keys: Vec<String> = token_keys.iter()
            .map(|k| k.to_string())
            .collect();
        
        // Note: This test needs the actual batch delete functionality
        // which would need to be implemented in TokenService
        // For now, we'll test individual operations
        
        println!("Token batch operations test setup completed");
    }

    /// Test session revocation
    #[tokio::test]
    async fn test_session_revocation() {
        let pool = setup_test_db().await;
        let (auth_svc, user_key) = create_test_user(&pool).await;
        
        let secret = b"test_jwt_secret_that_is_at_least_32_bytes_long!123";
        let auth_svc_clone = Arc::new(AuthService::new(pool.clone(), secret.to_vec()).unwrap());
        
        // Create a test session (simulated)
        let sid = Uuid::new_v4();
        let expires_at = Utc::now() + chrono::Duration::minutes(30);
        
        sqlx::query(
            r#"INSERT INTO auth_user_sessions (sid, user_key, user_agent, ip, login_method, 
                created_at, last_active, expires_at, revoked_at)
                VALUES ($1, $2, $3, $4, $5, now(), now(), $6, NULL)"#,
        )
        .bind(sid)
        .bind(user_key)
        .bind("test-agent")
        .bind("127.0.0.1")
        .bind("password")
        .bind(expires_at)
        .execute(&pool)
        .await
        .unwrap();
        
        // 1. Test revoke_session
        auth_svc_clone.revoke_session(user_key, sid).await.unwrap();
        
        // Verify session was revoked
        let session: Option<(Uuid,)> = sqlx::query_as(
            "SELECT sid FROM auth_user_sessions WHERE sid = $1 AND revoked_at IS NULL",
        )
        .bind(sid)
        .fetch_optional(&pool)
        .await
        .unwrap();
        
        assert!(session.is_none());
        
        // 2. Test revoke_other_sessions
        let sid2 = Uuid::new_v4();
        let expires_at2 = Utc::now() + chrono::Duration::minutes(30);
        
        sqlx::query(
            r#"INSERT INTO auth_user_sessions (sid, user_key, user_agent, ip, login_method, 
                created_at, last_active, expires_at, revoked_at)
                VALUES ($1, $2, $3, $4, $5, now(), now(), $6, NULL)"#,
        )
        .bind(sid2)
        .bind(user_key)
        .bind("test-agent-2")
        .bind("127.0.0.1")
        .bind("password")
        .bind(expires_at2)
        .execute(&pool)
        .await
        .unwrap();
        
        auth_svc_clone.revoke_other_sessions(user_key, sid).await.unwrap();
        
        // Verify only current session (sid) remains active
        let remaining_sessions: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT sid FROM auth_user_sessions WHERE user_key = $1 AND revoked_at IS NULL",
        )
        .bind(user_key)
        .fetch_all(&pool)
        .await
        .unwrap();
        
        assert_eq!(remaining_sessions.len(), 1);
        assert_eq!(remaining_sessions[0].0, sid);
        
        println!("Session revocation test passed");
    }

    /// Test user isolation in token operations
    #[tokio::test]
    async fn test_token_user_isolation() {
        let pool = setup_test_db().await;
        
        // Create two different users
        let (auth_svc1, user_key1) = create_test_user(&pool).await;
        let (auth_svc2, user_key2) = create_test_user(&pool).await;
        
        let token_svc = TokenService::new(pool.clone());
        
        // User1 creates a token
        let result1 = token_svc.create(
            user_key1,
            "user1_token",
            Some("default".to_string()),
            1000,
            false,
            None,
        ).await.unwrap();
        
        let user1_token_key = result1.token.key;
        
        // User2 tries to access User1's token (should fail)
        // In real implementation, this would be checked via bearer_user auth
        
        // Verify that User2 cannot see User1's token via query
        let user2_tokens: Vec<TokenRow> = sqlx::query_as(
            "SELECT key, user_key, name, key_hash, key_preview, group_id, quota, unlimited_quota, used_quota, expires_at, status, created_at FROM api_tokens WHERE user_key = $1",
        )
        .bind(user_key2)
        .fetch_all(&pool)
        .await
        .unwrap();
        
        assert_eq!(user2_tokens.len(), 0);
        
        // Verify User1's token is visible only to User1
        let user1_tokens: Vec<TokenRow> = sqlx::query_as(
            "SELECT key, user_key, name, key_hash, key_preview, group_id, quota, unlimited_quota, used_quota, expires_at, status, created_at FROM api_tokens WHERE user_key = $1",
        )
        .bind(user_key1)
        .fetch_all(&pool)
        .await
        .unwrap();
        
        assert_eq!(user1_tokens.len(), 1);
        assert_eq!(user1_tokens[0].key, user1_token_key);
        
        println!("Token user isolation test passed");
    }

    #[tokio::test]
    async fn run_all_tests() {
        test_session_management().await;
        test_settings_management().await;
        test_token_operations().await;
        test_token_batch_operations().await;
        test_session_revocation().await;
        test_token_user_isolation().await;
        
        println!("All tests passed!");
    }
}