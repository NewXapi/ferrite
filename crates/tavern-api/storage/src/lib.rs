//! tavern-storage — 酒馆数据根目录与文件读写底座
//!
//! 其它 tavern-api crate 的唯一落盘入口。
//! 对标 SillyTavern `src/users.js:getUserDirectories` + `USER_DIRECTORY_TEMPLATE`。

use std::io;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("invalid name: {0}")]
    InvalidName(String),
    #[error("path escapes parent: {0}")]
    PathEscape(String),
}

/// 数据根目录。默认 `data/`。
#[derive(Debug, Clone)]
pub struct DataRoot(PathBuf);

impl DataRoot {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// 某个用户的目录集合。
    pub fn user(&self, handle: &str) -> UserDirs {
        UserDirs {
            root: self.0.join(handle),
        }
    }
}

/// 单用户目录集合。MVP 只需这几个，不照抄原版 27 个。
#[derive(Debug, Clone)]
pub struct UserDirs {
    root: PathBuf,
}

impl UserDirs {
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn characters(&self) -> PathBuf {
        self.root.join("characters")
    }
    pub fn chats(&self) -> PathBuf {
        self.root.join("chats")
    }
    pub fn avatars(&self) -> PathBuf {
        self.root.join("avatars")
    }
    pub fn settings_file(&self) -> PathBuf {
        self.root.join("settings.json")
    }
    pub fn secrets_file(&self) -> PathBuf {
        self.root.join("secrets.json")
    }

    /// 建齐所有目录。启动时调一次。
    pub fn ensure(&self) -> Result<(), StorageError> {
        for d in [self.root.clone(), self.characters(), self.chats(), self.avatars()] {
            std::fs::create_dir_all(d)?;
        }
        Ok(())
    }
}

/// 清洗外部传入的文件名：去掉路径分隔符与父级引用。
pub fn sanitize_name(name: &str) -> Result<String, StorageError> {
    let bad = name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0');
    if bad {
        return Err(StorageError::InvalidName(name.to_string()));
    }
    Ok(name.to_string())
}

/// 校验 `child` 落在 `parent` 之内。对标原版 `isPathUnderParent`。
///
/// 纯词法判断，不碰文件系统：`..` 会被拒，不依赖路径是否已存在。
pub fn is_under(parent: &Path, child: &Path) -> bool {
    if child.components().any(|c| matches!(c, Component::ParentDir)) {
        return false;
    }
    child.starts_with(parent)
}

/// 拼一个位于 `parent` 内的安全路径。
pub fn join_checked(parent: &Path, name: &str) -> Result<PathBuf, StorageError> {
    let name = sanitize_name(name)?;
    let joined = parent.join(name);
    if !is_under(parent, &joined) {
        return Err(StorageError::PathEscape(joined.display().to_string()));
    }
    Ok(joined)
}

/// 原子写：同目录临时文件 + rename。避免写坏角色卡和 JSONL。
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    let parent = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(
        ".{}.tmp",
        path.file_name().and_then(|s| s.to_str()).unwrap_or("out")
    ));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// 目录内文件按 mtime 倒序列出。供最近聊天列表用。
pub fn list_by_mtime_desc(dir: &Path) -> Result<Vec<PathBuf>, StorageError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_file() {
            out.push((meta.modified()?, entry.path()));
        }
    }
    out.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(out.into_iter().map(|(_, p)| p).collect())
}
