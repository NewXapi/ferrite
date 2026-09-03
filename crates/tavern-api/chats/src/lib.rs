//! tavern-chats — 聊天记录存取，JSONL 一行一条消息
//!
//! 对标 SillyTavern `src/endpoints/chats.js`。
//! 存储形态：`<user>/chats/<character>/<chat>.jsonl`。

pub mod http;
pub use http::{ChatsState, router};

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tavern_storage::{self as storage, StorageError};

#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

/// 一条消息。字段名与 SillyTavern JSONL 对齐，便于导入现有数据。
///
/// `is_system` 区分 system 消息，agent prompt 组装时据此过滤。
/// `extra: MessageExtra` 平铺到顶层，已知字段（如 `api` / `model` / `reasoning` /
/// `token_count`）直接出现，老 JSONL 的未知顶层字段也透传到 `extra.additional`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub name: String,
    pub is_user: bool,
    #[serde(default)]
    pub is_system: bool,
    pub send_date: String,
    pub mes: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub swipes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swipe_id: Option<usize>,
    #[serde(default, flatten)]
    pub extra: MessageExtra,
}

/// 消息附带的 Agent 元数据，平铺在 Message 顶层。
///
/// `additional` 透传未知字段，老 JSONL 里出现过的顶层自定义字段不会因为
/// 后端不认识就丢掉。已知字段（`api` / `model` / `reasoning` 等）以
/// 顶层 key 出现，避免破坏 SillyTavern 兼容。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageExtra {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_duration: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gen_started: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gen_finished: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force_avatar: Option<String>,
    /// 未知字段透传保留，不因后端不认识就丢掉。
    #[serde(flatten)]
    pub additional: serde_json::Map<String, serde_json::Value>,
}

/// 最近聊天列表的一行。
#[derive(Debug, Clone, Serialize)]
pub struct ChatSummary {
    pub file_name: String,
    pub preview: String,
}

fn chat_path(chats_dir: &Path, character: &str, chat: &str) -> Result<PathBuf, ChatError> {
    let dir = storage::join_checked(chats_dir, character)?;
    Ok(storage::join_checked(&dir, &format!("{chat}.jsonl"))?)
}

/// 整份覆盖保存。
///
/// 不做增量追加：swipe 与消息编辑会改写历史行，追加会写出错误历史。
pub fn save(
    chats_dir: &Path,
    character: &str,
    chat: &str,
    messages: &[Message],
) -> Result<(), ChatError> {
    let path = chat_path(chats_dir, character, chat)?;
    let mut buf = String::new();
    for m in messages {
        buf.push_str(&serde_json::to_string(m)?);
        buf.push('\n');
    }
    storage::write_atomic(&path, buf.as_bytes())?;
    Ok(())
}

/// 逐行读取。单行坏了跳过，不让整个聊天报废。
pub fn load(chats_dir: &Path, character: &str, chat: &str) -> Result<Vec<Message>, ChatError> {
    let path = chat_path(chats_dir, character, chat)?;
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    Ok(raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Message>(l).ok())
        .collect())
}

pub fn delete(chats_dir: &Path, character: &str, chat: &str) -> Result<(), ChatError> {
    let path = chat_path(chats_dir, character, chat)?;
    if path.exists() {
        std::fs::remove_file(path).map_err(StorageError::Io)?;
    }
    Ok(())
}

pub fn rename(chats_dir: &Path, character: &str, from: &str, to: &str) -> Result<(), ChatError> {
    let src = chat_path(chats_dir, character, from)?;
    let dst = chat_path(chats_dir, character, to)?;
    std::fs::rename(src, dst).map_err(StorageError::Io)?;
    Ok(())
}

/// 最近聊天：按 mtime 倒序 + 首行预览。
pub fn recent(chats_dir: &Path, character: &str) -> Result<Vec<ChatSummary>, ChatError> {
    let dir = storage::join_checked(chats_dir, character)?;
    let mut out = Vec::new();
    for path in storage::list_by_mtime_desc(&dir)? {
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }
        let file_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let preview = std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| {
                raw.lines()
                    .find(|l| !l.trim().is_empty())
                    .and_then(|l| serde_json::from_str::<Message>(l).ok())
                    .map(|m| m.mes.chars().take(80).collect::<String>())
            })
            .unwrap_or_default();
        out.push(ChatSummary { file_name, preview });
    }
    Ok(out)
}
