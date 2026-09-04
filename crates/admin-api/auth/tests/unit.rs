//! 纯单元测试：password round-trip、jwt round-trip、refresh split。

#[test]
fn password_hash_and_verify_round_trip() {
    let phc = auth::password::hash("hunter2hunter").expect("hash");
    assert!(auth::password::verify("hunter2hunter", &phc));
    assert!(!auth::password::verify("wrong", &phc));
    // PHC 字符串以 $argon2id$ 开头
    assert!(phc.starts_with("$argon2id$"));
}

#[test]
fn jwt_round_trip() {
    let secret = b"super-secret-test-key";
    let (token, exp) = auth::jwt::issue(secret, "user-uuid-1", 1, 1, "sid-1").expect("issue");
    assert!(exp > 0);
    let claims = auth::jwt::parse(secret, &token).expect("parse");
    assert_eq!(claims.sub, "user-uuid-1");
    assert_eq!(claims.role, 1);
    assert_eq!(claims.auth_version, 1);
    assert_eq!(claims.sid, "sid-1");
}

#[test]
fn jwt_wrong_secret_rejected() {
    let (token, _) = auth::jwt::issue(b"good", "u", 1, 1, "s").unwrap();
    assert!(auth::jwt::parse(b"bad", &token).is_err());
}
