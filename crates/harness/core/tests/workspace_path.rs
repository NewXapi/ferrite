//! `WorkspacePath` 安全路径校验的契约测试。
//!
//! 覆盖 02-harness.md §1.4 的验收清单：
//! - `../etc/passwd` → 拒绝
//! - `C:\Windows` → 拒绝
//! - `\0` → 拒绝
//! - `./foo/./bar` → `foo/bar`
//! - 空字符串 → 拒绝

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