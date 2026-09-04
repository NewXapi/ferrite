//! tavern-presets — 单用户预设 JSON 文件
//!
//! 对标 SillyTavern `src/endpoints/presets.js`：每个 API 源（openai、instruct、context、
//! sysprompt、reasoning + kobold/novel/textgenerationwebui）一个子目录，每个预设一个
//! JSON 文件。文件落 `<user>/<folder>/<sanitize(name)>.json`。

pub mod http;
pub use http::{PresetsState, router};

use std::path::{Path, PathBuf};

use serde_json::Value;
use tavern_storage::{self as storage, StorageError};
#[derive(Debug, thiserror::Error)]
pub enum PresetError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unknown apiId: {0}")]
    UnknownApiId(String),
    #[error("preset not found: {0}")]
    NotFound(String),
}

/// API 源到子目录名的映射。未知返回 `None`。
pub fn folder_for(api_id: &str) -> Option<&'static str> {
    // ponytail: 静态匹配避免堆分配；调用方拿到 `&'static str` 直接 `join`。
    #[rustfmt::skip]
    const KOBOLD_DIR: &str = "KoboldAI Settings";
    #[rustfmt::skip]
    const NOVEL_DIR: &str = "NovelAI Settings";
    #[rustfmt::skip]
    const TGWEBUI_DIR: &str = "TextGen Settings";
    match api_id {
        "openai" => Some("OpenAI Settings"),
        "instruct" => Some("instruct"),
        "context" => Some("context"),
        "sysprompt" => Some("sysprompt"),
        "reasoning" => Some("reasoning"),
        "kobold" | "koboldhorde" => Some(KOBOLD_DIR),
        "novel" => Some(NOVEL_DIR),
        "textgenerationwebui" => Some(TGWEBUI_DIR),
        _ => None,
    }
}

/// 把 `apiId` 解析为子目录路径。
fn subdir(root: &Path, api_id: &str) -> Result<PathBuf, PresetError> {
    let folder = folder_for(api_id).ok_or_else(|| PresetError::UnknownApiId(api_id.to_string()))?;
    Ok(root.join(folder))
}

/// 预设文件路径：`<subdir>/<sanitize(name)>.json`。
fn preset_path(root: &Path, api_id: &str, name: &str) -> Result<PathBuf, PresetError> {
    let dir = subdir(root, api_id)?;
    let safe = storage::sanitize_name(name)?;
    let path = dir.join(format!("{safe}.json"));
    // ponytail: `sanitize_name` 已挡掉分隔符与 `..`；这里再确认路径未逃出子目录。
    if !storage::is_under(&dir, &path) {
        return Err(PresetError::Storage(StorageError::PathEscape(
            path.display().to_string(),
        )));
    }
    Ok(path)
}

/// 读单个预设。文件不存在走 `NotFound`。
pub fn load(root: &Path, api_id: &str, name: &str) -> Result<Value, PresetError> {
    let path = preset_path(root, api_id, name)?;
    match std::fs::read_to_string(&path) {
        Ok(raw) => Ok(serde_json::from_str(&raw)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(PresetError::NotFound(name.to_string()))
        }
        Err(e) => Err(e.into()),
    }
}

/// 原子覆盖写。
pub fn save(root: &Path, api_id: &str, name: &str, preset: &Value) -> Result<(), PresetError> {
    let path = preset_path(root, api_id, name)?;
    storage::write_atomic(&path, serde_json::to_vec_pretty(preset)?.as_slice())?;
    Ok(())
}

/// 删除。文件不存在走 `NotFound`。
pub fn delete(root: &Path, api_id: &str, name: &str) -> Result<(), PresetError> {
    let path = preset_path(root, api_id, name)?;
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(PresetError::NotFound(name.to_string()))
        }
        Err(e) => Err(e.into()),
    }
}

/// 子目录内所有预设名（去掉 `.json`）。
///
/// 无目录视作空列表；不可读目录项（悬空符号链接、权限不足）被跳过，
/// 单个坏项不该让整个列表失败。点开头的隐藏文件与 `write_atomic`
/// 崩溃残留的临时文件也不列出。
pub fn list(root: &Path, api_id: &str) -> Result<Vec<String>, PresetError> {
    let dir = subdir(root, api_id)?;
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(it) => it,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e.into()),
    };
    for entry in entries {
        // ponytail: 单个不可读目录项不应让整个列表 500。
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        // ponytail: `sanitize_name` 已经禁止分隔符；这里只剥后缀。
        let Some(stem) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(name) = stem.strip_suffix(".json") else {
            continue;
        };
        // 隐藏文件与写入中途残留的临时文件不是预设。
        if name.is_empty() || name.starts_with('.') {
            continue;
        }
        out.push(name.to_string());
    }
    out.sort();
    Ok(out)
}
/// 「恢复内置默认值」的结果。
///
/// 线格式与 SillyTavern `/api/presets/restore` 一致：`isDefault` + `preset`。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RestoredPreset {
    /// 是否命中了内置默认预设
    #[serde(rename = "isDefault")]
    pub is_default: bool,
    /// 命中的默认预设内容；未命中为空对象
    pub preset: Value,
}

/// 恢复「内置默认值」。本 crate 不附带内置预设 → 始终返回未命中。
///
/// 保留这个端点是为了让前端的「恢复默认」按钮有稳定契约；一旦引入内置预设，
/// 只需在此查表并返回 `is_default: true`。
pub fn restore(_root: &Path, _api_id: &str, _name: &str) -> RestoredPreset {
    RestoredPreset {
        is_default: false,
        preset: Value::Object(serde_json::Map::new()),
    }
}
