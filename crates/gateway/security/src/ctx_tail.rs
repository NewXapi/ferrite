//! `ctx_tail` —— `CtxTail` 跨 chunk 状态机
//!
//! SSE 流可能被网络层切成任意长度 chunk，敏感词可能被截断在 chunk 边界。
//! CtxTail 持有最后 N 字节（max(词长, 域名长)），新 chunk 到达时拼接扫描。

pub struct CtxTail {
    tail: Vec<u8>,
    offset: usize,
}

impl CtxTail {
    /// 创建指定 tail 长度的状态机
    pub fn new(tail_len: usize) -> Self {
        Self {
            tail: vec![0; tail_len],
            offset: 0,
        }
    }

    /// 推入新 chunk，返回**已确认安全可输出**的字节
    pub fn push(&mut self, chunk: &[u8]) -> &[u8] {
        // TODO: 拼接 tail + chunk，扫描，输出安全部分，更新 tail
        let _ = chunk;
        unimplemented!("CtxTail::push")
    }

    /// 流结束时 flush 残留字节
    pub fn flush(&mut self) -> &[u8] {
        &self.tail[..self.offset]
    }
}
