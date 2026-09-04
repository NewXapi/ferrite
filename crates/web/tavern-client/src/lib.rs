//! tavern-client — 调 tavern-api 的 HTTP 客户端与 SSE 消费。
//!
//! 职责：
//! - 角色 / 聊天 / 设置 / 密钥接口的 HTTP 封装（gloo-net，WASM 友好）
//! - 生成接口的请求发送与 SSE 逐帧解析（[`parse_sse_line`]）
//! - 统一错误形状 [`ApiError`]
//!
//! DTO 字段与 tavern-api 各 crate 对齐，未知字段容忍（serde default），
//! 避免后端加字段导致前端解码失败。SSE 解析是纯函数，可在无网络环境测试。

use gloo_net::http::{Request, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

/// 统一错误形状
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("网络错误: {0}")]
    Network(#[from] gloo_net::Error),
    #[error("HTTP 状态 {status}: {body}")]
    Http { status: u16, body: String },
    #[error("JSON 解码失败: {0}")]
    Json(#[from] serde_json::Error),
}

/// 发送 HTTP 请求并返回响应文本
async fn send_request(request: Request) -> Result<String, ApiError> {
    let response: Response = request.send().await?;
    if !response.ok() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(ApiError::Http { status, body });
    }
    response.text().await.map_err(ApiError::from)
}

/// 角色 DTO，与 tavern-api/characters 字段完全对齐
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
    pub extra: Map<String, serde_json::Value>,
}

/// 角色列表 DTO，与 tavern-api/characters 字段完全对齐
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSummary {
    pub file_name: String,
    pub name: String,
    pub description: String,
}

/// 消息 DTO，与 tavern-api/chats 字段完全对齐
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub name: String,
    pub is_user: bool,
    pub send_date: String,
    pub mes: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub swipes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swipe_id: Option<usize>,
    /// 未知字段透传保留，不因后端不认识就丢掉。
    #[serde(flatten)]
    pub extra: Map<String, serde_json::Value>,
}

/// SSE 事件类型
#[derive(Debug, Clone, PartialEq)]
pub enum SseEvent {
    /// 生成内容片段
    Message(String),
    /// SSE 结束标记
    Done,
}

/// 解析 SSE 行，返回事件或 None
/// 支持 tavern-api/generate 接口的 SSE 格式
pub fn parse_sse_line(line: &str) -> Option<SseEvent> {
    let line = line.trim();
    if let Some(data) = line.strip_prefix("data: ") {
        // 只剥 `data: ` 前缀，不 trim 载荷：流式拼接时尾随空格是有意义的
        // （例如英文单词间的空格），剥掉会把下一个 delta 粘到前一个词上。
        let data = data.trim().to_string();
        if data == "[DONE]" {
            return Some(SseEvent::Done);
        }

        // 尝试提取 choices[0].delta.content
        if let Ok(json) = serde_json::from_str::<Value>(&data) {
            if let Some(choices) = json.get("choices") {
                if let Some(first_choice) = choices.get(0) {
                    if let Some(delta) = first_choice.get("delta") {
                        if let Some(content) = delta.get("content") {
                            if let Some(c) = content.as_str() {
                                return Some(SseEvent::Message(c.to_string()));
                            }
                        }
                    }
                }
            }
        }

        // 如果 JSON 解析失败或缺少 content 字段，直接返回文本
        return Some(SseEvent::Message(data));
    }
    None
}

/// 角色 CRUD 接口

/// 获取角色列表
pub async fn list_characters() -> Result<Vec<CharacterSummary>, ApiError> {
    let request = Request::get("/tavern/characters")
        .build()
        .map_err(ApiError::from)?;
    let text = send_request(request).await?;
    let summaries: Vec<CharacterSummary> = serde_json::from_str(&text)?;
    Ok(summaries)
}

/// 获取单个角色
pub async fn get_character(name: String) -> Result<Character, ApiError> {
    let url = format!("/tavern/characters/{name}");
    let request = Request::get(&url).build().map_err(ApiError::from)?;
    let text = send_request(request).await?;
    let character: Character = serde_json::from_str(&text)?;
    Ok(character)
}

/// 创建角色
pub async fn create_character(file_name: String, character: Character) -> Result<(), ApiError> {
    let body_str = serde_json::to_string(&serde_json::json!({
        "file_name": file_name,
        "character": character
    }))?;
    let request = Request::post("/tavern/characters")
        .header("Content-Type", "application/json")
        .body(body_str)
        .map_err(ApiError::from)?;
    send_request(request).await?;
    Ok(())
}

/// 更新角色
pub async fn update_character(name: String, character: Character) -> Result<(), ApiError> {
    let url = format!("/tavern/characters/{name}");
    let body_str = serde_json::to_string(&character)?;
    let request = Request::put(&url)
        .header("Content-Type", "application/json")
        .body(body_str)
        .map_err(ApiError::from)?;
    send_request(request).await?;
    Ok(())
}

/// 删除角色
pub async fn delete_character(name: String) -> Result<(), ApiError> {
    let url = format!("/tavern/characters/{name}");
    let request = Request::delete(&url).build().map_err(ApiError::from)?;
    send_request(request).await?;
    Ok(())
}

/// 聊天操作接口

/// 获取最近的聊天列表
pub async fn recent_chats(character: String) -> Result<Vec<CharacterSummary>, ApiError> {
    let url = format!("/tavern/chats/{character}");
    let request = Request::get(&url).build().map_err(ApiError::from)?;
    let text = send_request(request).await?;
    let summaries: Vec<CharacterSummary> = serde_json::from_str(&text)?;
    Ok(summaries)
}

/// 加载聊天内容
pub async fn load_chat(character: String, chat: String) -> Result<Vec<Message>, ApiError> {
    let url = format!("/tavern/chats/{character}/{chat}");
    let request = Request::get(&url).build().map_err(ApiError::from)?;
    let text = send_request(request).await?;
    let messages: Vec<Message> = serde_json::from_str(&text)?;
    Ok(messages)
}

/// 保存聊天内容
pub async fn save_chat(
    character: String,
    chat: String,
    messages: Vec<Message>,
) -> Result<(), ApiError> {
    let url = format!("/tavern/chats/{character}/{chat}");
    let body_str = serde_json::to_string(&messages)?;
    let request = Request::put(&url)
        .header("Content-Type", "application/json")
        .body(body_str)
        .map_err(ApiError::from)?;
    send_request(request).await?;
    Ok(())
}

/// 删除聊天
pub async fn delete_chat(character: String, chat: String) -> Result<(), ApiError> {
    let url = format!("/tavern/chats/{character}/{chat}");
    let request = Request::delete(&url).build().map_err(ApiError::from)?;
    send_request(request).await?;
    Ok(())
}

/// 重命名聊天
pub async fn rename_chat(character: String, from: String, to: String) -> Result<(), ApiError> {
    let url = format!("/tavern/chats/{character}/{from}/rename/{to}");
    let request = Request::put(&url).build().map_err(ApiError::from)?;
    send_request(request).await?;
    Ok(())
}

/// 设置接口

/// 加载设置
pub async fn load_settings() -> Result<Value, ApiError> {
    let request = Request::get("/tavern/settings")
        .build()
        .map_err(ApiError::from)?;
    let text = send_request(request).await?;
    let settings: Value = serde_json::from_str(&text)?;
    Ok(settings)
}

/// 保存设置
pub async fn save_settings(settings: Value) -> Result<(), ApiError> {
    let body_str = serde_json::to_string(&settings)?;
    let request = Request::put("/tavern/settings")
        .header("Content-Type", "application/json")
        .body(body_str)
        .map_err(ApiError::from)?;
    send_request(request).await?;
    Ok(())
}

/// 密钥接口

/// 获取密钥状态
pub async fn secrets_state() -> Result<Value, ApiError> {
    let request = Request::get("/tavern/secrets")
        .build()
        .map_err(ApiError::from)?;
    let text = send_request(request).await?;
    let secrets: Value = serde_json::from_str(&text)?;
    Ok(secrets)
}

/// 设置密钥
pub async fn put_secret(key: String, value: String) -> Result<(), ApiError> {
    let url = format!("/tavern/secrets/{key}");
    let body_str = serde_json::to_string(&serde_json::json!({ "key": key, "value": value }))?;
    let request = Request::put(&url)
        .header("Content-Type", "application/json")
        .body(body_str)
        .map_err(ApiError::from)?;
    send_request(request).await?;
    Ok(())
}

/// 删除密钥
pub async fn delete_secret(key: String) -> Result<(), ApiError> {
    let url = format!("/tavern/secrets/{key}");
    let request = Request::delete(&url).build().map_err(ApiError::from)?;
    send_request(request).await?;
    Ok(())
}

/// 生成接口，支持 SSE 逐帧消费

/// 生成内容，支持 SSE 流式处理
///
/// # 参数
/// * `prompt` - 生成请求参数
/// * `on_delta` - 每当收到新的 Delta 内容时调用，参数为 Delta 字符串
pub async fn generate<F>(prompt: Value, mut on_delta: F) -> Result<(), ApiError>
where
    F: FnMut(String),
{
    let body_str = serde_json::to_string(&prompt)?;
    let request = Request::post("/tavern/generate")
        .header("Content-Type", "application/json")
        .body(body_str)
        .map_err(ApiError::from)?;
    let text = send_request(request).await?;

    // SSE 解析：读取 body 并逐行解析
    for line in text.lines() {
        if let Some(event) = parse_sse_line(line) {
            match event {
                SseEvent::Message(delta) => {
                    on_delta(delta);
                }
                SseEvent::Done => {
                    break;
                }
            }
        }
    }
    Ok(())
}
