//! tavern-state — 酒馆前端全局状态。
//!
//! 职责：
//! - 当前角色 / 聊天 / 消息列表（含 swipes）
//! - 生成中状态与流式缓冲
//! - 中止语义：置位后 [`append_delta`] 变 no-op
//! - 通过 [`init`] / [`select_character`] / [`open_chat`] / [`send`] / [`abort`] 驱动状态
//!
//! 状态只依赖 [`tavern_client`] 做落盘，不直接发 HTTP。
//! 宏替换由 [`harness_prompt`] 兜底，前端负责主替换。

use dioxus::prelude::*;
use serde_json::{Value, json};

pub use tavern_client::Character;
pub use tavern_client::CharacterSummary;
pub use tavern_client::Message;

/// 全局状态信号，所有页面共享。
pub static STATE: GlobalSignal<TavernState> = Signal::global(TavernState::default);

/// 酒馆前端的完整全局状态。
#[derive(Debug, Clone, Default)]
pub struct TavernState {
    /// 可用角色列表。
    pub characters: Vec<CharacterSummary>,
    /// 当前选中角色：(file_name, 角色卡)。
    pub character: Option<(String, Character)>,
    /// 当前聊天名。
    pub chat: Option<String>,
    /// 当前聊天消息。
    pub messages: Vec<Message>,
    /// 是否正在生成。
    pub generating: bool,
    /// 中止标志。
    aborted: bool,
    /// 用户名。
    pub user_name: String,
    /// 当前模型。
    pub model: Option<String>,
    /// 最近一次错误。
    pub last_error: Option<String>,
}

/// 从设置 JSON 中提取 user_name 和模型。
fn extract_settings(settings: &Value) -> (String, Option<String>) {
    let user_name = settings
        .get("user_name")
        .and_then(|v| v.as_str())
        .unwrap_or("User")
        .to_string();
    let model = settings
        .get("model")
        .and_then(|v| v.as_str())
        .map(String::from);
    (user_name, model)
}

/// 生成新聊天名。
///
/// ponytail: 无 js-sys 依赖，用消息数+1 生成基础名；查重递增需 recent_chats 列表，
/// 当前未接入，后续 PR 对接 recent_chats 后改为真查重。
fn new_chat_name(messages: &[Message]) -> String {
    format!("chat-{}", messages.len() + 1)
}

/// 初始化：加载设置和角色列表。
///
/// 错误写入 [`TavernState::last_error`]，不 panic。
pub async fn init() {
    STATE.with_mut(|s| {
        s.last_error = None;
        s.characters.clear();
        s.character = None;
        s.chat = None;
        s.messages.clear();
    });

    let settings = match tavern_client::load_settings().await {
        Ok(v) => v,
        Err(e) => {
            STATE.with_mut(|s| {
                s.last_error = Some(format!("加载设置失败: {e}"));
            });
            return;
        }
    };

    let (user_name, model) = extract_settings(&settings);

    match tavern_client::list_characters().await {
        Ok(list) => {
            STATE.with_mut(|s| {
                s.user_name = user_name;
                s.model = model;
                s.characters = list;
            });
        }
        Err(e) => {
            STATE.with_mut(|s| {
                s.user_name = user_name;
                s.model = model;
                s.last_error = Some(format!("加载角色列表失败: {e}"));
            });
        }
    }
}

/// 选中角色，重置聊天状态。
///
/// 若角色有 first_mes，种子首条 assistant 消息。
pub async fn select_character(file_name: String) {
    match tavern_client::get_character(file_name.clone()).await {
        Ok(card) => {
            let msgs = seed_messages(&card);
            STATE.with_mut(|s| {
                s.character = Some((file_name, card));
                s.chat = None;
                s.messages = msgs;
                s.last_error = None;
            });
        }
        Err(e) => {
            STATE.with_mut(|s| {
                s.last_error = Some(format!("加载角色失败: {e}"));
            });
        }
    }
}

/// 用角色卡生成种子消息（first_mes → assistant）。
///
/// 纯函数，便于无网络测试。
pub fn seed_messages(card: &Character) -> Vec<Message> {
    if card.first_mes.is_empty() {
        Vec::new()
    } else {
        vec![Message {
            name: card.name.clone(),
            is_user: false,
            send_date: String::new(),
            mes: card.first_mes.clone(),
            swipes: Vec::new(),
            swipe_id: None,
            extra: Default::default(),
        }]
    }
}

/// 打开指定聊天，加载历史消息。
pub async fn open_chat(chat_name: String) {
    let character_file = match &STATE.read().character {
        Some((file_name, _)) => file_name.clone(),
        None => {
            STATE.with_mut(|s| {
                s.last_error = Some("未选择角色".to_string());
            });
            return;
        }
    };

    match tavern_client::load_chat(character_file, chat_name.clone()).await {
        Ok(msgs) => {
            STATE.with_mut(|s| {
                s.chat = Some(chat_name);
                s.messages = msgs;
                s.last_error = None;
            });
        }
        Err(e) => {
            STATE.with_mut(|s| {
                s.last_error = Some(format!("加载聊天失败: {e}"));
            });
        }
    }
}

/// 中止当前生成。
///
/// 置位后 [`append_delta`] 变 no-op；send 结束后若已中止则不保存，并重置标志。
pub fn abort() {
    STATE.with_mut(|s| {
        s.aborted = true;
    });
}

/// 追加 delta 到最后一条 assistant 消息。
///
/// 中止后 no-op。
pub fn append_delta(delta: &str) {
    STATE.with_mut(|s| {
        if s.aborted {
            return;
        }
        if let Some(last) = s.messages.last_mut() {
            if !last.is_user {
                last.mes.push_str(delta);
            }
        }
    });
}

/// 发送用户消息并触发生成。
pub async fn send(text: String) {
    let (character, character_name, user_name, model) = {
        let s = STATE.read();
        let (character_name, card) = match &s.character {
            Some((name, card)) => (name.clone(), card.clone()),
            None => {
                STATE.with_mut(|st| {
                    st.last_error = Some("请先选择角色".to_string());
                });
                return;
            }
        };
        let model = match &s.model {
            Some(m) => m.clone(),
            None => {
                STATE.with_mut(|st| {
                    st.last_error = Some("未设置模型".to_string());
                });
                return;
            }
        };
        (card, character_name, s.user_name.clone(), model)
    };

    STATE.with_mut(|s| {
        s.messages.push(Message {
            name: s.user_name.clone(),
            is_user: true,
            send_date: String::new(),
            mes: text,
            swipes: Vec::new(),
            swipe_id: None,
            extra: Default::default(),
        });
        s.generating = true;
        s.aborted = false;
        s.last_error = None;
    });

    let body = build_generate_body(
        &character,
        &character_name,
        &user_name,
        &STATE.read().messages,
        &model,
    );

    let assistant_name = character.name.clone();
    STATE.with_mut(|s| {
        s.messages.push(Message {
            name: assistant_name,
            is_user: false,
            send_date: String::new(),
            mes: String::new(),
            swipes: Vec::new(),
            swipe_id: None,
            extra: Default::default(),
        });
    });

    let result = tavern_client::generate(body, |delta| append_delta(&delta)).await;

    STATE.with_mut(|s| {
        s.generating = false;
    });

    if let Err(e) = result {
        STATE.with_mut(|s| {
            s.last_error = Some(format!("生成失败: {e}"));
        });
        return;
    }

    let was_aborted = STATE.read().aborted;
    if was_aborted {
        STATE.with_mut(|s| {
            s.aborted = false;
        });
        return;
    }

    let (character_name_for_save, chat_name, msgs_to_save) = {
        let s = STATE.read();
        let char_name = match &s.character {
            Some((name, _)) => name.clone(),
            None => return,
        };
        let chat = s.chat.clone().unwrap_or_else(|| new_chat_name(&s.messages));
        (char_name, chat, s.messages.clone())
    };

    if let Err(e) =
        tavern_client::save_chat(character_name_for_save, chat_name.clone(), msgs_to_save).await
    {
        STATE.with_mut(|s| {
            s.last_error = Some(format!("保存聊天失败: {e}"));
        });
        return;
    }

    STATE.with_mut(|s| {
        s.chat = Some(chat_name);
    });
}

/// 构建生成请求体。
///
/// 1. 用 PromptInput 渲染（展开 {{char}}/{{user}}）
/// 2. truncate_history 按 4096 token budget 裁剪
/// 3. 组装 OpenAI-compatible JSON，首条 system 置前
pub fn build_generate_body(
    character: &Character,
    character_name: &str,
    user_name: &str,
    messages: &[Message],
    model: &str,
) -> Value {
    let system = build_system_prompt(character);

    let mut prompt = harness_prompt::PromptInput::new().with_system(system);
    prompt.character_name = Some(character_name.to_string());
    prompt.user_name = Some(user_name.to_string());

    for msg in messages {
        let role = if msg.is_user {
            harness_prompt::AgentModelRole::User
        } else {
            harness_prompt::AgentModelRole::Assistant
        };
        let text = resolve_swipe(msg);
        prompt = prompt.push_message(harness_prompt::AgentModelMessage::text(role, text));
    }

    let request =
        harness_prompt::render(&prompt).unwrap_or_else(|_| harness_prompt::AgentModelRequest {
            system: None,
            messages: Vec::new(),
            tools: Vec::new(),
            metadata: None,
        });

    // ponytail: len/4 粗估 token，真 tokenizer 后续接 harness-tokenizer
    let truncated = harness_prompt::truncate_history(&request.messages, 4096, |m| {
        (m.text_payload().chars().count() / 4 + 1) as u32
    });

    let mut out_messages: Vec<Value> = Vec::new();
    if let Some(ref sys) = request.system {
        out_messages.push(json!({"role": "system", "content": sys}));
    }
    for msg in &truncated {
        let role_str = match msg.role {
            harness_prompt::AgentModelRole::System => "system",
            harness_prompt::AgentModelRole::User => "user",
            harness_prompt::AgentModelRole::Assistant => "assistant",
            harness_prompt::AgentModelRole::Tool => "tool",
        };
        out_messages.push(json!({"role": role_str, "content": msg.text_payload()}));
    }

    json!({
        "model": model,
        "messages": out_messages,
        "stream": true,
        "_ferrite_agent_prompt_marker": ""
    })
}

/// 构建 system prompt。
///
/// 顺序：description → Personality → Scenario → Example dialogue，跳过空字段，用 "\n\n" 连接。
pub fn build_system_prompt(character: &Character) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !character.description.is_empty() {
        parts.push(character.description.clone());
    }
    if !character.personality.is_empty() {
        parts.push(format!("Personality: {}", character.personality));
    }
    if !character.scenario.is_empty() {
        parts.push(format!("Scenario: {}", character.scenario));
    }
    if !character.mes_example.is_empty() {
        parts.push(format!("Example dialogue:\n{}", character.mes_example));
    }
    parts.join("\n\n")
}

/// 解析 swipe：选中 swipe_id 对应项，越界回退 mes。
pub fn resolve_swipe(msg: &Message) -> String {
    msg.swipes
        .get(msg.swipe_id.unwrap_or(0))
        .cloned()
        .unwrap_or_else(|| msg.mes.clone())
}
