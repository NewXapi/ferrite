//! `gateway-security` —— 内容安全（Aho-Corasick 敏感词 / 流式截断 / 第三方审核）
//!
//! 通过 `apps/gateway` 的 `extension-security` Cargo feature 条件编译链接。
//!
//! ## 文件分工
//!
//! - [`wordlist`] —— 加密词库加载
//! - [`aho_corasick`] —— 字节级 AC 自动机
//! - [`ctx_tail`] —— `CtxTail` 跨 chunk 状态机
//! - [`sanitize`] —— 输入静默脱敏
//! - [`moderation`] —— `Moderation` trait + 第三方实现
//! - [`stage`] —— `StreamingInterceptStage`

pub mod aho_corasick;
pub mod ctx_tail;
pub mod moderation;
pub mod sanitize;
pub mod stage;
pub mod wordlist;

pub use aho_corasick::AhoCorasick;
pub use ctx_tail::CtxTail;
pub use moderation::{Disabled, Moderation, ModerationResult, OpenAiOmnimod, QwenGuard};
pub use sanitize::sanitize;
pub use stage::StreamingInterceptStage;
pub use wordlist::{Category, LoadError, WordList};
