//! tavern-characters — 角色卡 CRUD
//!
//! 对标 SillyTavern `src/endpoints/characters.js` + `src/character-card-parser.js`。
//! 存储形态与原版一致：`<user>/characters/<name>.png`，角色 JSON 放在 PNG 的
//! `chara` tEXt chunk 里（base64），便于直接导入现有卡。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tavern_storage::{self as storage, StorageError};

pub mod http;
pub mod png;

pub use http::{router, CharactersState};

#[derive(Debug, thiserror::Error)]
pub enum CharacterError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("png: {0}")]
    Png(#[from] png::PngError),
    #[error("not found: {0}")]
    NotFound(String),
}

/// Character Card V2 字段集。未知字段透传保留。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub personality: String,
    #[serde(default)]
    pub scenario: String,
    #[serde(default)]
    pub first_mes: String,
    #[serde(default)]
    pub mes_example: String,
    #[serde(default)]
    pub creator_notes: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// 列表行：只要展示需要的字段，不为了列表把每张卡全解析出来。
#[derive(Debug, Clone, Serialize)]
pub struct CharacterSummary {
    pub file_name: String,
    pub name: String,
    pub description: String,
}

fn card_path(dir: &Path, name: &str) -> Result<PathBuf, CharacterError> {
    Ok(storage::join_checked(dir, &format!("{name}.png"))?)
}

/// 读一张卡。
pub fn get(dir: &Path, file_name: &str) -> Result<Character, CharacterError> {
    let path = card_path(dir, file_name)?;
    let bytes = std::fs::read(&path).map_err(StorageError::Io)?;
    let json = png::read_chara(&bytes)?;
    Ok(serde_json::from_slice(&json)?)
}

/// 写一张卡。`avatar_png` 是底图；为空则复用已有底图。
pub fn save(
    dir: &Path,
    file_name: &str,
    character: &Character,
    avatar_png: Option<&[u8]>,
) -> Result<(), CharacterError> {
    let path = card_path(dir, file_name)?;
    let base: Vec<u8> = match avatar_png {
        Some(b) => b.to_vec(),
        None => std::fs::read(&path).map_err(StorageError::Io)?,
    };
    let json = serde_json::to_vec(character)?;
    let out = png::write_chara(&base, &json)?;
    storage::write_atomic(&path, &out)?;
    Ok(())
}

pub fn delete(dir: &Path, file_name: &str) -> Result<(), CharacterError> {
    let path = card_path(dir, file_name)?;
    if path.exists() {
        std::fs::remove_file(path).map_err(StorageError::Io)?;
    }
    Ok(())
}

pub fn rename(dir: &Path, from: &str, to: &str) -> Result<(), CharacterError> {
    let src = card_path(dir, from)?;
    let dst = card_path(dir, to)?;
    std::fs::rename(src, dst).map_err(StorageError::Io)?;
    Ok(())
}

/// 列表。单张卡解析失败跳过，不让整个列表报废。
pub fn list(dir: &Path) -> Result<Vec<CharacterSummary>, CharacterError> {
    let mut out = Vec::new();
    for path in storage::list_by_mtime_desc(dir)? {
        if path.extension().and_then(|s| s.to_str()) != Some("png") {
            continue;
        }
        let file_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let Ok(json) = png::read_chara(&bytes) else { continue };
        let Ok(c) = serde_json::from_slice::<Character>(&json) else { continue };
        out.push(CharacterSummary {
            file_name,
            name: c.name,
            description: c.description.chars().take(120).collect(),
        });
    }
    Ok(out)
}
