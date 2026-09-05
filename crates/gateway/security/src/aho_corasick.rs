//! `aho_corasick` —— 字节级 AC 自动机
//!
//! 固定大小 `[256]int32` 转移表，零分配扫描。

use crate::wordlist::WordList;

/// 匹配命中位置
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchHit {
    pub start: usize,
    pub end: usize,
    pub word_index: usize,
}

#[allow(dead_code)] // ponytail: 桩 — build/scan 实现后读取这些表
pub struct AhoCorasick {
    goto: Vec<[i32; 256]>,
    fail: Vec<i32>,
    output: Vec<Vec<usize>>,
    state_count: usize,
}

impl AhoCorasick {
    /// 从词库构建 AC 自动机
    pub fn build(words: &WordList) -> Self {
        // TODO: 构建 goto / fail / output 三张表
        let _ = words;
        unimplemented!("AhoCorasick::build")
    }

    /// 扫描输入字节，返回所有命中
    pub fn scan(&self, input: &[u8]) -> Vec<MatchHit> {
        // TODO: 状态机扫描
        let _ = input;
        unimplemented!("AhoCorasick::scan")
    }
}
