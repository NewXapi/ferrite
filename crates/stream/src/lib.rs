//! 流式回传：SSE 单写者、心跳、断开取消。
//! 见 docs/08-mvp.md §1.7 §4.6
pub mod cancel;
pub mod heartbeat;
pub mod nonstream;
pub mod sse;
