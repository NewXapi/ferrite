//! billing::topup_epay 纯逻辑单测（无 PG）：MD5 签名 roundtrip、
//! create() URL 结构、verify_callback 的接受与拒绝分支。
//!
//! 跑：`cargo test -p billing --test topup_epay`（无 DB 依赖，离线全跑）。
//!
//! 签名期望值是**手工算的**（python hashlib，见 PR 构建记录），不是用本 crate
//! 的 sign() 自己算自己——那样实现错了断言也跟着错。create 与 callback 的签名串
//! 不同（create 含 notify_url，回调不含），各自一个期望值。

use billing::topup::{TopupProvider, TopupSession};
use billing::topup_epay::{EpayMerchant, EpayProvider};
use md5::Digest;

/// 测试商户配置（占位符，非真密钥；key 仅供单测签名 roundtrip）。
fn merchant() -> EpayMerchant {
    EpayMerchant {
        gateway_url: "https://pay.example.com".into(),
        pid: "10000".into(),
        key: "TESTKEY0123456789".into(),
        name: "充值".into(),
        notify_url: "https://example.com/api/topup/webhook/epay".into(),
    }
}

/// 开单参数（create("ORDER001", "CNY", 10)）签名串对应的期望 MD5。
///
/// 串 = `money=10&name=充值&notify_url=<notify_url>&out_trade_no=ORDER001
///       &pid=10000&type=buy` + key（参数按 key 序，& 连接，末尾直接接密钥）。
const EXPECT_CREATE_SIGN: &str = "4BE50DAF365DA2B19157F2AE35D7D138";

/// 回调参数签名串的期望 MD5（create 的同一商户 + 同一订单，但参数集是回调的）。
///
/// 串 = `money=10&name=充值&out_trade_no=ORDER001&pid=10000
///       &trade_no=20260917120000001&trade_status=TRADE_SUCCESS&type=buy` + key。
const EXPECT_CALLBACK_SIGN: &str = "D11106A02CF559CEAAD02B654BDE244C";

/// 构造一份签名合法的回调 payload（out_trade_no 可覆盖，用于不同订单）。
fn callback_payload(out_trade_no: &str) -> serde_json::Value {
    serde_json::json!({
        "pid": "10000",
        "trade_no": "20260917120000001",
        "out_trade_no": out_trade_no,
        "type": "buy",
        "name": "充值",
        "money": "10",
        "trade_status": "TRADE_SUCCESS",
        "sign": EXPECT_CALLBACK_SIGN,
        "sign_type": "MD5",
    })
}

/// create() 的 URL 结构断言：端点形态、外部单号、签名落位，且签名与
/// URL 里携带的参数逐字一致（roundtrip：URL 参数重算 sign 不变）。
#[tokio::test]
async fn create_builds_signed_mapi_url() {
    let p = EpayProvider::new(merchant());
    assert_eq!(p.id(), "epay");

    let TopupSession {
        reference,
        payment_url,
    } = p
        .create("ORDER001", "CNY", 10)
        .await
        .expect("纯本地拼 URL，无外部依赖");
    // 外部单号 = 本域订单 key（回调按它回执）。
    assert_eq!(reference, "ORDER001");
    let url = payment_url.expect("epay 开单必给支付跳转 URL");
    assert!(
        url.starts_with("https://pay.example.com/mapi.php?m=buy&"),
        "经典跳转形态：{url}"
    );
    assert!(url.contains("out_trade_no=ORDER001"), "缺外部单号: {url}");
    assert!(url.contains("sign_type=MD5"), "缺签名类型: {url}");
    assert!(
        url.contains(&format!("sign={EXPECT_CREATE_SIGN}")),
        "缺签名: {url}"
    );

    // roundtrip：解析 URL query，剔除 sign/sign_type/m 后重算，必须等于 URL 里的 sign。
    let parsed = url::Url::parse(&url).expect("create 产出的必须是合法 URL");
    let pairs: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(k, _)| k != "sign" && k != "sign_type" && k != "m")
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    let mut sorted: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));
    let mut buf = String::new();
    for (k, v) in sorted {
        if !buf.is_empty() {
            buf.push('&');
        }
        buf.push_str(k);
        buf.push('=');
        buf.push_str(v);
    }
    buf.push_str("TESTKEY0123456789");
    let recomputed = hex::encode(md5::Md5::digest(buf.as_bytes())).to_uppercase();
    assert_eq!(recomputed, EXPECT_CREATE_SIGN, "URL 签名与参数不一致");
}

/// create() 的支付口径守卫：epay 只收 CNY；非 CNY 是调用方口径错误。
#[tokio::test]
async fn create_rejects_non_cny_currency() {
    let p = EpayProvider::new(merchant());
    assert!(p.create("ORDER001", "FREE", 10).await.is_err());
}

/// create() 的金额守卫：0/负金额不给渠道（哪怕调用方漏了校验）。
#[tokio::test]
async fn create_rejects_nonpositive_amount() {
    let p = EpayProvider::new(merchant());
    assert!(p.create("ORDER001", "CNY", 0).await.is_err());
    assert!(p.create("ORDER001", "CNY", -5).await.is_err());
}

/// verify_callback 接受签名合法的回执，返回 out_trade_no（= 本域订单 key）。
#[tokio::test]
async fn verify_callback_accepts_valid_signature() {
    let p = EpayProvider::new(merchant());
    let order = p
        .verify_callback(&callback_payload("ORDER001"))
        .await
        .expect("签名合法必须通过");
    assert_eq!(order, "ORDER001");
}

/// 篡改金额 → 签名失效 → None（不写库的路由层 401 就靠这条）。
#[tokio::test]
async fn verify_callback_rejects_tampered_money() {
    let p = EpayProvider::new(merchant());
    let mut payload = callback_payload("ORDER001");
    payload["money"] = "10000".into(); // 篡改金额，签名不再匹配
    assert_eq!(
        p.verify_callback(&payload).await,
        None,
        "篡改任何参与签名的字段都必须拒绝"
    );
}

/// 缺 out_trade_no → None（无法定位订单，验签通过也无意义）。
#[tokio::test]
async fn verify_callback_rejects_missing_out_trade_no() {
    let p = EpayProvider::new(merchant());
    let mut payload = callback_payload("ORDER001");
    payload.as_object_mut().unwrap().remove("out_trade_no");
    assert_eq!(p.verify_callback(&payload).await, None);
}

/// 错签 → None（密钥不对 / 伪造回调，这是 webhook 的唯一信任边界）。
#[tokio::test]
async fn verify_callback_rejects_wrong_signature() {
    let p = EpayProvider::new(merchant());
    let mut payload = callback_payload("ORDER001");
    payload["sign"] = "0123456789ABCDEF0123456789ABCDEF".into();
    assert_eq!(p.verify_callback(&payload).await, None);
}

/// 非成功交易状态 → None：只有 TRADE_SUCCESS 才该入金，失败/待查不结算。
#[tokio::test]
async fn verify_callback_rejects_non_success_status() {
    let p = EpayProvider::new(merchant());
    // 需要重算签名才过得了验签这关（trade_status 参与签名）。
    let pairs = [
        ("money", "10"),
        ("name", "充值"),
        ("out_trade_no", "ORDER001"),
        ("pid", "10000"),
        ("trade_no", "20260917120000001"),
        ("trade_status", "WAIT_BUYER_PAY"),
        ("type", "buy"),
    ];
    let mut buf = String::new();
    let mut sorted: Vec<(&str, &str)> = pairs.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(b.0));
    for (k, v) in sorted {
        if !buf.is_empty() {
            buf.push('&');
        }
        buf.push_str(k);
        buf.push('=');
        buf.push_str(v);
    }
    buf.push_str("TESTKEY0123456789");
    let sig = hex::encode(md5::Md5::digest(buf.as_bytes())).to_uppercase();
    let payload = serde_json::json!({
        "pid": "10000",
        "trade_no": "20260917120000001",
        "out_trade_no": "ORDER001",
        "type": "buy",
        "name": "充值",
        "money": "10",
        "trade_status": "WAIT_BUYER_PAY",
        "sign": sig,
        "sign_type": "MD5",
    });
    assert_eq!(
        p.verify_callback(&payload).await,
        None,
        "非 TRADE_SUCCESS 的回执绝不能触发入金"
    );
}

/// 非对象 payload / 非正金额 → None（防御畸形回调）。
#[tokio::test]
async fn verify_callback_rejects_malformed() {
    let p = EpayProvider::new(merchant());
    assert_eq!(p.verify_callback(&serde_json::json!([1, 2])).await, None);
    let mut zero = callback_payload("ORDER001");
    zero["money"] = "0".into();
    // 金额变了但签名没重算 → 验签先拒；再构造一个 0 元但签名合法的：
    let pairs = [
        ("money", "0"),
        ("name", "充值"),
        ("out_trade_no", "ORDER001"),
        ("pid", "10000"),
        ("trade_no", "20260917120000001"),
        ("trade_status", "TRADE_SUCCESS"),
        ("type", "buy"),
    ];
    let mut buf = String::new();
    let mut sorted: Vec<(&str, &str)> = pairs.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(b.0));
    for (k, v) in sorted {
        if !buf.is_empty() {
            buf.push('&');
        }
        buf.push_str(k);
        buf.push('=');
        buf.push_str(v);
    }
    buf.push_str("TESTKEY0123456789");
    let sig = hex::encode(md5::Md5::digest(buf.as_bytes())).to_uppercase();
    let mut zero_ok = callback_payload("ORDER001");
    zero_ok["money"] = "0".into();
    zero_ok["sign"] = sig.into();
    assert_eq!(
        p.verify_callback(&zero_ok).await,
        None,
        "0 元回执不能入金（哪怕签名合法）"
    );
    assert_eq!(p.verify_callback(&zero).await, None);
}
