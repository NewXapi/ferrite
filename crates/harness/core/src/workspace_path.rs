//! Workspace 安全路径校验。
//!
//! 整段搬运自 `tt-domain/src/models/agent/mod.rs:347-411`，但将错误类型
//! `DomainError::InvalidData(String)` 替换为本 crate 内部的 `WorkspacePathError`。
//!
//! 拒绝 NUL / 绝对路径 / Windows 盘符 / `..`；规范化 `\` → `/`；折叠 `CurDir`。

use std::fmt;
use std::path::Component;

use serde::{Deserialize, Serialize};

/// Workspace 路径校验错误。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum WorkspacePathError {
    /// 路径为空字符串 / 仅由 `.` 等无效段组成。
    Empty,
    /// 路径中包含 NUL 字节。
    ContainsNul,
    /// 路径以 `/` 或 `\` 开头（绝对路径）。
    Absolute,
    /// 路径使用 Windows 盘符前缀（如 `C:\`）。
    WindowsDrivePrefix,
    /// 路径包含 `..` 段。
    ParentTraversal,
    /// 路径包含绝对路径或前缀（在 `Path::components` 阶段发现）。
    NotRelative,
}

impl WorkspacePathError {
    pub fn message(&self) -> &'static str {
        match self {
            Self::Empty => "Workspace path cannot be empty",
            Self::ContainsNul => "Workspace path cannot contain NUL",
            Self::Absolute => "Workspace path must be relative",
            Self::WindowsDrivePrefix => "Workspace path cannot use a Windows drive prefix",
            Self::ParentTraversal => "Workspace path cannot contain ..",
            Self::NotRelative => "Workspace path must be relative",
        }
    }
}

impl fmt::Display for WorkspacePathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl std::error::Error for WorkspacePathError {}

/// 相对工作区路径。
///
/// `WorkspacePath` 是相对工作区根目录的安全相对路径——不包含绝对路径、目录穿越
/// (`..`)、NUL 字节或 Windows 盘符前缀。所有反斜杠 `\` 会被规范化为正斜杠 `/`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct WorkspacePath(String);

impl WorkspacePath {
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, WorkspacePathError> {
        let raw = raw.as_ref();
        if raw.is_empty() {
            return Err(WorkspacePathError::Empty);
        }
        if raw.contains('\0') {
            return Err(WorkspacePathError::ContainsNul);
        }
        if raw.starts_with('/') || raw.starts_with('\\') {
            return Err(WorkspacePathError::Absolute);
        }
        if raw.len() >= 2
            && raw.as_bytes()[1] == b':'
            && raw.as_bytes()[0].is_ascii_alphabetic()
        {
            return Err(WorkspacePathError::WindowsDrivePrefix);
        }

        let normalized = raw.replace('\\', "/");
        let path = std::path::Path::new(&normalized);
        let mut parts = Vec::new();
        for component in path.components() {
            match component {
                Component::Normal(value) => {
                    let segment = value.to_string_lossy();
                    if segment.is_empty() {
                        continue;
                    }
                    parts.push(segment.to_string());
                }
                Component::CurDir => {}
                Component::ParentDir => return Err(WorkspacePathError::ParentTraversal),
                Component::RootDir | Component::Prefix(_) => {
                    return Err(WorkspacePathError::NotRelative);
                }
            }
        }

        if parts.is_empty() {
            return Err(WorkspacePathError::Empty);
        }

        Ok(Self(parts.join("/")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}