//! 通知 — 渠道熔断/余额不足/系统事件的用户与管理员触达。
//!
//! 参考: new-api notify_user.go (per-user 类型限流) + dispatch_webhook.go
//! (HMAC 签名 + SSRF 控制) + limit_notify.go。
//!
//! V1 事件源: 渠道自动熔断 (dispatch 健康事件聚合后触发) → 通知 root。
//! TODO(#802): email (SMTP) 渠道; webhook HMAC; bark/gotify; 限流规则。

use store::StoreError;

/// 通知类型 (限流的维度)。
#[derive(Debug, Clone, Copy)]
pub enum Topic {
    /// 渠道熔断/恢复 (收件人: admin+)。
    ChannelHealth,
    /// 用户余额不足 (收件人: 用户本人)。
    LowBalance,
}

pub trait Notifier: Send + Sync {
    fn topic(&self) -> Topic;
    /// 投递 (实现自带 per-recipient 限流, 避免风暴)。
    fn send(
        &self,
        recipient: &str,
        title: &str,
        body: &str,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;
}

/// 触发入口 — 由 observe/dispatch 的健康事件下游调用。
pub async fn dispatch(_topic: Topic, _recipient: &str, _payload: &serde_json::Value) -> Result<(), StoreError> {
    todo!("TODO(#802): 路由到注册的 Notifier + 限流")
}
