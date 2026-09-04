use session::{outcome_from_data, ApiError, LoginData, LoginOutcome, SessionInfo, User};

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
    let session: SessionInfo = serde_json::from_str(json).unwrap();
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
