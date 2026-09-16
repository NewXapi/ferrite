//! epay（易支付）真支付渠道 —— 跳转开单 + 回调 MD5 验签。
//!
//! 协议（彩虹易支付兼容形态）：
//! - 开单：拼 `{gateway_url}/mapi.php?m=buy&<签名参数>&sign=<MD5 大写>&sign_type=MD5`，
//!   用户浏览器跳转过去完成付款；商户侧外部单号 `out_trade_no` = 本域订单 key。
//! - 回调：渠道 POST 到 `notify_url`（`/api/topup/webhook/epay`），参数含
//!   `pid/trade_no/out_trade_no/type/name/money/trade_status/sign`。验签 = 用
//!   同一套 MD5 重算并比对（验签是唯一信任边界，见 topup::topup_webhook）。
//!
//! 签名口径（create 与 verify_callback 必须逐字一致，否则渠道验不过）：
//! 参与签名的参数按 key 升序，过滤空值与 `sign`/`sign_type`，拼成
//! `k1=v1&k2=v2…`，末尾直接追加商户密钥，MD5 十六进制**大写**。
//!
//! 金额口径：本域订单行存的是**点数**（points 货币，fiat 不进余额，见
//! [`crate::topup::TopupService::open_topup`] 的不变式守卫）。渠道收的是人民币：
//! 开单时由 `open_topup` 用 [`crate::currency::CurrencyService::convert`] 把
//! 用户要付的 ¥X 折算成点数落库，`create` 收到的 `(currency, amount)` 已经是
//! 支付口径（`("CNY", 元)`），点数不进签名——渠道只认它实际收的钱。
//!
//! # 安全边界
//!
//! `verify_callback` 只管验签与字段完整性（缺参/错签/非成功状态/非正金额一律
//! `None` → 路由层 401，不写库）。**回调金额与订单点数的绑定不由本域校验**：
//! 签名只能证明参数未被篡改，无法证明「这笔 ¥ 对应那个订单」，该绑定需要
//! 渠道侧按 out_trade_no 核对（ponytail: 若后续要在本域强校验金额，需在
//! billing_topups 加 charge 列存支付口径，当前 scope 只加 reference）。

use md5::{Digest, Md5};
use url::Url;

use crate::topup::{ProviderError, ProviderFuture, TopupProvider, TopupSession};

/// 易支付商户配置 —— 对应 `config.toml` 的 `[payment.epay]` 段。
///
/// `.example` 全为占位符；**真密钥只放本地 gitignore 的 config.toml，禁提交**。
#[derive(Clone, serde::Deserialize)]
pub struct EpayMerchant {
    /// 渠道根地址，如 `https://pay.example.com`（跳转 URL 挂 `/mapi.php`）。
    pub gateway_url: String,
    /// 商户 ID（易支付后台 pid）。
    pub pid: String,
    /// 商户密钥（MD5 签名用）。
    pub key: String,
    /// 商品名 / 订单标题，展示在支付页（如「充值」）。
    pub name: String,
    /// 异步回调地址（本域 `POST /api/topup/webhook/epay`）。
    pub notify_url: String,
}

impl std::fmt::Debug for EpayMerchant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 手写 Debug 掩埋 key：支付密钥进日志是泄漏，配置结构体随时可能被 debug 打印。
        f.debug_struct("EpayMerchant")
            .field("gateway_url", &self.gateway_url)
            .field("pid", &self.pid)
            .field("key", &"<redacted>")
            .field("name", &self.name)
            .field("notify_url", &self.notify_url)
            .finish()
    }
}

/// 易支付渠道实现（`TopupProvider`）：开单拼跳转 URL，回调 MD5 验签。
pub struct EpayProvider {
    cfg: EpayMerchant,
}

impl EpayProvider {
    /// 用商户配置构造；装配期由 `TopupService::with_epay` 注入（配置缺则不注册）。
    pub fn new(cfg: EpayMerchant) -> Self {
        Self { cfg }
    }
}

impl TopupProvider for EpayProvider {
    fn id(&self) -> &'static str {
        "epay"
    }

    /// 开单：拼易支付跳转 URL。`(currency, amount)` 是**支付口径**——
    /// `open_topup` 已折算好，本域点数不进签名。
    fn create<'a>(
        &'a self,
        order_key: &'a str,
        currency: &'a str,
        amount: i64,
    ) -> ProviderFuture<'a, Result<TopupSession, ProviderError>> {
        Box::pin(async move {
            // 渠道只收 CNY：口径不符是调用方编程错误（开单层已按 epay 分流），
            // 在这层挡住，不让错位金额进签名。
            if currency != "CNY" {
                return Err(ProviderError(format!(
                    "epay charges CNY only, got {currency}"
                )));
            }
            // 0/负金额订单不给开（调用方已校验 >0，这是信任边界上的二次守卫）。
            if amount <= 0 {
                return Err(ProviderError("epay money must be positive".into()));
            }
            // 易支付 money 是元级整数（前端输入正整数元，见 rewards.rs）。
            let money = amount.to_string();
            let pairs = vec![
                ("pid".into(), self.cfg.pid.clone()),
                ("type".into(), "buy".into()),
                ("out_trade_no".into(), order_key.to_string()),
                ("notify_url".into(), self.cfg.notify_url.clone()),
                ("name".into(), self.cfg.name.clone()),
                ("money".into(), money),
            ];
            let sig = sign(&pairs, &self.cfg.key);

            let base = Url::parse(&self.cfg.gateway_url)
                .map_err(|e| ProviderError(format!("bad gateway_url: {e}")))?;
            let mut url = base
                .join("mapi.php")
                .map_err(|e| ProviderError(format!("bad gateway_url: {e}")))?;
            // m=buy 是端点形态（经典跳转形态），不参与签名。
            url.query_pairs_mut().append_pair("m", "buy");
            for (k, v) in &pairs {
                url.query_pairs_mut().append_pair(k, v);
            }
            url.query_pairs_mut()
                .append_pair("sign", &sig)
                .append_pair("sign_type", "MD5");

            Ok(TopupSession {
                reference: order_key.to_string(),
                payment_url: Some(url.to_string()),
            })
        })
    }

    /// 回调验签：重算 MD5 比对，通过则返回 `out_trade_no`（= 本域订单 key）。
    ///
    /// 拒绝（→ None → 路由层 401，不写库）条件：非对象 / 缺 out_trade_no /
    /// 缺 sign / trade_status 非 TRADE_SUCCESS / money 非正数 / 签名不符。
    fn verify_callback<'a>(
        &'a self,
        payload: &'a serde_json::Value,
    ) -> ProviderFuture<'a, Option<String>> {
        Box::pin(async move {
            let obj = payload.as_object()?;
            // 签名口径：收到的全部非空参数（除 sign/sign_type 自身）按 key 序重算。
            let pairs: Vec<(String, String)> = obj
                .iter()
                .filter_map(|(k, v)| {
                    if k == "sign" || k == "sign_type" {
                        return None;
                    }
                    let s = v.as_str()?;
                    (!s.is_empty()).then(|| (k.clone(), s.to_string()))
                })
                .collect();
            let expect = sign(&pairs, &self.cfg.key);
            let got = obj.get("sign").and_then(|v| v.as_str())?;
            if got != expect {
                return None;
            }
            // 验签通过后的语义校验：成功状态 + 外部单号 + 正金额。
            let out_trade_no = obj.get("out_trade_no").and_then(|v| v.as_str())?;
            if out_trade_no.is_empty() {
                return None;
            }
            let trade_status = obj.get("trade_status").and_then(|v| v.as_str())?;
            if trade_status != "TRADE_SUCCESS" {
                return None;
            }
            let money = obj.get("money").and_then(|v| v.as_str())?;
            // 金额必须是有限正数（0 元订单不入金；解析失败说明回调格式坏了）。
            let money: f64 = money.parse().ok()?;
            if !money.is_finite() || money <= 0.0 {
                return None;
            }
            Some(out_trade_no.to_string())
        })
    }
}

/// 易支付签名：参数按 key 升序、滤掉空值与 `sign`/`sign_type`，拼成
/// `k1=v1&k2=v2…`，末尾追加商户密钥，MD5 → 十六进制大写。
///
/// 纯函数（无 IO），create 与 verify_callback 共用，单测可离线断言 roundtrip。
fn sign(pairs: &[(String, String)], key: &str) -> String {
    // 转 (&str,&str) 再排序：直接对 Vec<&(String,String)> 排序会陷入双重引用,
    // a.0/b.0 的借用形式让 clippy::needless_borrow 与类型检查互相矛盾。
    let mut sorted: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));
    let mut buf = String::new();
    for (k, v) in sorted {
        if k == "sign" || k == "sign_type" || v.is_empty() {
            continue;
        }
        if !buf.is_empty() {
            buf.push('&');
        }
        buf.push_str(k);
        buf.push('=');
        buf.push_str(v);
    }
    buf.push_str(key);
    hex::encode(Md5::digest(buf.as_bytes())).to_uppercase()
}
