//! 密钥读写与对外掩码。

use tavern_secrets::{read, remove, state, write};

fn tmp(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("ferrite-sec-{}-{tag}.json", std::process::id()))
}

#[test]
fn state_never_exposes_plaintext() {
    let p = tmp("state");
    std::fs::remove_file(&p).ok();
    write(&p, "api_key_openai", "sk-secret-value").unwrap();
    let st = state(&p).unwrap();
    assert_eq!(st.get("api_key_openai"), Some(&true));
    let rendered = serde_json::to_string(&st).unwrap();
    assert!(!rendered.contains("sk-secret-value"));
    std::fs::remove_file(&p).ok();
}

#[test]
fn write_read_remove_cycle() {
    let p = tmp("cycle");
    std::fs::remove_file(&p).ok();
    assert_eq!(read(&p, "k").unwrap(), None);
    write(&p, "k", "v").unwrap();
    assert_eq!(read(&p, "k").unwrap().as_deref(), Some("v"));
    remove(&p, "k").unwrap();
    assert_eq!(read(&p, "k").unwrap(), None);
    std::fs::remove_file(&p).ok();
}
