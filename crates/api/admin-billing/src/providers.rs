//! 支付渠道抽象 — epay / stripe / creem / waffo 各一实现。
//!
//! 参考: new-api topup_{stripe,creem,waffo,waffo_pancake}.go +
//! payment_webhook_availability.go (回调可用性开关) +
//! payment_compliance.go (合规验收门)。

/// 支付渠道 trait — console 注册进注册表, webhook 路由按 provider id 分发。
pub trait PaymentProvider: Send + Sync {
    /// 渠道标识 ("epay" | "stripe" | "creem")。
    fn id(&self) -> &'static str;

    /// 渠道当前是否可用 (配置完整性/合规开关)。
    /// console 下单前检查; 不可用渠道不在前端展示。
    fn available(&self) -> bool;

    /// 创建渠道侧支付会话, 返回支付 URL / 跳转参数。
    /// TODO(#605): 签名/金额格式/币种按渠道实现。
    fn create_checkout(
        &self,
        order: &contract::records::PaymentOrderRecord,
    ) -> impl Future<Output = Result<String, store::StoreError>> + Send;

    /// 验签并提取标准化事件 (txn_id, 状态, 金额)。
    /// TODO(#605): 各渠道验签实现; 验签失败 → StoreError::Conflict。
    fn verify_webhook(
        &self,
        payload: &serde_json::Value,
    ) -> impl Future<Output = Result<WebhookEvent, store::StoreError>> + Send;
}

/// 标准化回调事件 — 各渠道差异被 providers 吸收, orders 只看这个。
pub struct WebhookEvent {
    pub provider_txn_id: String,
    /// "paid" | "failed"。
    pub outcome: &'static str,
    pub amount: String,
}
