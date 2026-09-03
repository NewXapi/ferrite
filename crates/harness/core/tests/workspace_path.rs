//! `WorkspacePath` 安全路径校验的契约测试。
//!
//! 覆盖 02-harness.md §1.4 的验收清单：
//! - `../etc/passwd` → 拒绝
//! - `C:\Windows` → 拒绝
//! - `\0` → 拒绝
//! - `./foo/./bar` → `foo/bar`
//! - 空字符串 → 拒绝
//!
//! 同时覆盖 serde ABI 边界：反序列化必须强制走 `WorkspacePath::parse`，
//! 避免字符串字段绕过路径校验直接落入 `WorkspacePath` 内部。

use harness_core::workspace_path::{WorkspacePath, WorkspacePathError};

#[test]
fn rejects_parent_traversal() {
    assert_eq!(
        WorkspacePath::parse("../etc/passwd"),
        Err(WorkspacePathError::ParentTraversal),
        "目录穿越必须被拒绝"
    );
}

#[test]
fn rejects_windows_drive_prefix() {
    assert_eq!(
        WorkspacePath::parse("C:\\Windows"),
        Err(WorkspacePathError::WindowsDrivePrefix),
        "Windows 盘符前缀必须被拒绝"
    );
}

#[test]
fn rejects_nul_byte() {
    assert_eq!(
        WorkspacePath::parse("\0"),
        Err(WorkspacePathError::ContainsNul),
        "NUL 字节必须被拒绝"
    );
    assert_eq!(
        WorkspacePath::parse("foo\0bar"),
        Err(WorkspacePathError::ContainsNul),
        "路径段中包含的 NUL 也必须被拒绝"
    );
}

#[test]
fn normalizes_curdir_segments() {
    let path = WorkspacePath::parse("./foo/./bar").expect("./foo/./bar 应当合法");
    assert_eq!(
        path.as_str(),
        "foo/bar",
        "`./` 段必须被折叠,反斜杠规范化同时进行"
    );
}

#[test]
fn rejects_empty_string() {
    assert_eq!(
        WorkspacePath::parse(""),
        Err(WorkspacePathError::Empty),
        "空字符串必须被拒绝"
    );
}

#[test]
fn accepts_nested_relative_paths() {
    let path = WorkspacePath::parse("output/2026-09/main.md").expect("nested relative path");
    assert_eq!(path.as_str(), "output/2026-09/main.md");
}

#[test]
fn normalizes_backslashes_to_forward_slashes() {
    let path = WorkspacePath::parse("output\\drafts\\main.md").expect("windows path");
    assert_eq!(path.as_str(), "output/drafts/main.md");
}

#[test]
fn rejects_absolute_unix_paths() {
    assert_eq!(
        WorkspacePath::parse("/etc/passwd"),
        Err(WorkspacePathError::Absolute)
    );
}

#[test]
fn rejects_absolute_windows_paths() {
    assert_eq!(
        WorkspacePath::parse("\\Users\\me"),
        Err(WorkspacePathError::Absolute)
    );
}

#[test]
fn deserialization_rejects_parent_traversal() {
    let result: Result<WorkspacePath, _> = serde_json::from_str("\"../etc/passwd\"");
    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some(WorkspacePathError::ParentTraversal.to_string()),
        "JSON 形式的 `../etc/passwd` 必须经 parse 拒绝"
    );
}

#[test]
fn deserialization_rejects_windows_drive_prefix() {
    let result: Result<WorkspacePath, _> = serde_json::from_str("\"C:\\\\Windows\"");
    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some(WorkspacePathError::WindowsDrivePrefix.to_string()),
        "JSON 形式的 Windows 盘符前缀必须经 parse 拒绝"
    );
}

#[test]
fn deserialization_rejects_nul_byte() {
    let result: Result<WorkspacePath, _> = serde_json::from_str("\"\\u0000\"");
    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some(WorkspacePathError::ContainsNul.to_string()),
        "JSON 形式 (`\\u0000`) 的 NUL 字节必须经 parse 拒绝"
    );
}

#[test]
fn deserialization_normalizes_curdir_segments() {
    let path: WorkspacePath =
        serde_json::from_str("\"./foo/./bar\"").expect("JSON `./foo/./bar` 必须经 parse 规范化");
    assert_eq!(
        path.as_str(),
        "foo/bar",
        "反序列化必须保留 `./` 折叠与反斜杠规范化"
    );
}

#[test]
fn deserialization_preserves_serialized_shape() {
    let path = WorkspacePath::parse("output/drafts/main.md").expect("valid path");
    let json = serde_json::to_string(&path).expect("serialize");
    assert_eq!(
        json, "\"output/drafts/main.md\"",
        "序列化形状必须是裸字符串，与 ABI 一致"
    );

    let round_tripped: WorkspacePath = serde_json::from_str(&json).expect("round trip");
    assert_eq!(round_tripped, path);
}

#[test]
fn deserialization_rejects_absolute_paths() {
    let result: Result<WorkspacePath, _> = serde_json::from_str("\"/abs\"");
    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some(WorkspacePathError::Absolute.to_string()),
        "JSON 形式的绝对路径必须经 parse 拒绝"
    );
}

#[test]
fn deserialization_rejects_empty_string() {
    let result: Result<WorkspacePath, _> = serde_json::from_str("\"\"");
    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some(WorkspacePathError::Empty.to_string()),
        "JSON 形式的空路径必须经 parse 拒绝"
    );
}
