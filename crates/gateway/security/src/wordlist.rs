//! `wordlist` —— 加密词库加载
//!
//! 词库二进制用 `include_bytes!` 嵌入到 crate，启动时通过 argon2id 派生 key 解密。
//! 替换走 `ArcSwap<WordList>`，由 `service::sync` 推送。

use std::sync::Arc;
use arc_swap::ArcSwap;
use thiserror::Error;
use std::collections::HashMap;

/// 词条分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Political,
    Porn,
    Violence,
    Fraud,
    Custom,
}

/// 词库
#[derive(Default)]
pub struct WordList {
    /// 明文词条
    words: Vec<String>,
    /// 词 → 分类
    classification: HashMap<String, Category>,
}

impl WordList {
    /// 启动时加载（include_bytes! 嵌入 + argon2id 派生 key + AES-GCM 解密）
    pub fn load() -> Result<Arc<Self>, LoadError> {
        // TODO: include_bytes! + decrypt
        unimplemented!("WordList::load")
    }

    /// 替换快照（service::sync 推送）
    pub fn install_into(swap: &ArcSwap<Self>, new: Arc<Self>) {
        swap.store(new);
    }

    /// 查询所有词条
    pub fn words(&self) -> &[String] {
        &self.words
    }

    /// 查询词条分类
    pub fn classify(&self, word: &str) -> Option<Category> {
        self.classification.get(word).copied()
    }
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("decrypt failed")]
    Decrypt,
    #[error("invalid format")]
    Format,
}
