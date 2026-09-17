//! subscriptions 纯逻辑层断言 — DTO 映射 + 口径换算（不依赖 PG）。
//!
//! 为什么不跑 DB 集成：CI 无 postgres service（`.github/workflows/ci.yml`
//! 只有 fmt/clippy/rust 作业），同域 redeem.rs / currency_display.rs 的
//! DB 测试在 CI 上就是 skip 态；本文件的全部断言都是纯函数/纯映射，
//! 无 PG 也能在 CI 真跑（AGENTS.md：测试尽可能覆盖完整场景，放 CI）。
//!
//! DB 侧行为（upsert 的 ON CONFLICT 幂等、delete 的 NotFound、迁移建表）
//! 由 admin-router 装配 + 0017 迁移在带 PG 的 smoke 环境覆盖。
//!
//! 覆盖场景：
//! - contract 的 `SubscriptionPlanRecord → SubscriptionDto` 字段映射
//!   （period_val/period_unit/group/max_per_user/enabled/quota/price）
//! - upsert 入库前校验：price 非法值 → BadRequest（不静默落库）
//! - quota f64 展示口径 → i64 内部单位换算（×500_000 四舍五入、负值/溢出拒绝）

use billing::subscriptions::{parse_price, quota_display_to_internal};
use contract::api::billing::SubscriptionDto;
use contract::records::SyncMeta;
use contract::records::billing::SubscriptionPlanRecord;

/// 构造一条记录（时间戳走 sqlx re-export 的 chrono，测试不单独引 chrono 依赖）。
fn sample_plan() -> SubscriptionPlanRecord {
    SubscriptionPlanRecord {
        meta: SyncMeta {
            key: "plan-monthly".into(),
            schema_version: 1,
            logical_version: 1,
            origin: "center".into(),
            updated_at: sqlx::types::chrono::Utc::now(),
        },
        name: "月度会员".into(),
        price: "9.9".into(),
        currency: "CNY".into(),
        duration_days: 30,
        quota: 4_950_000,
        upgrade_group: Some("vip".into()),
        max_purchases: Some(3),
        enabled: true,
    }
}

/// contract 的 record → DTO 映射必须逐字段对齐两套口径。
///
/// 为什么逐字段钉：这是 ferrite 形状 ↔ new-api 形状的唯一映射点，任一字段
/// 漂移都会让前端拿到错的套餐信息（时长/分组/限购次数）。period_val/period_unit
/// /group/max_per_user 是两套口径的转接字段，最容易错，单独断言。
#[test]
fn record_to_dto_field_mapping() {
    let rec = sample_plan();
    let dto = SubscriptionDto::from(&rec);

    // 转接字段（ferrite → new-api 口径）。
    assert_eq!(dto.period_val, Some(30), "periodVal <- duration_days");
    assert_eq!(
        dto.period_unit.as_deref(),
        Some("days"),
        "periodUnit 固定 days"
    );
    assert_eq!(dto.group.as_deref(), Some("vip"), "group <- upgrade_group");
    assert_eq!(dto.max_per_user, Some(3), "maxPerUser <- max_purchases");

    // 直映字段。
    assert_eq!(dto.name, "月度会员");
    assert_eq!(dto.price, Some(9.9), "price NUMERIC 字符串 → f64");
    // quota 记录层是内部单位（4_950_000），DTO 是展示口径（÷500_000 = 9.9）。
    assert_eq!(dto.quota, Some(9.9), "quota 内部单位 → 展示口径 ÷500_000");
    assert_eq!(dto.enabled, Some(true));

    // new-api 独有、ferrite 侧未实现的字段必须为 None（前端按可选处理）。
    assert_eq!(dto.description, None);
    assert_eq!(dto.downgrade_group, None);
    assert_eq!(dto.reset_cycle, None);
    assert_eq!(dto.stripe_price_id, None);
}

/// quota 换算：展示值 ×500_000 四舍五入成内部单位。
///
/// 为什么钉 0.5 与 1.0000001 两个点：四舍五入是口径里最容易写错成 floor
/// 或截断的地方（floor 会让运营配的零碎额度系统性缩水）。
#[test]
fn quota_conversion_multiplies_and_rounds() {
    assert_eq!(quota_display_to_internal(1.0).unwrap(), 500_000);
    assert_eq!(quota_display_to_internal(0.5).unwrap(), 250_000);
    assert_eq!(quota_display_to_internal(0.0).unwrap(), 0);
    // 1.0000001 × 500_000 = 500_000.05 → 四舍五入 500_000（floor 也同值，
    // 换 1.0000011 → 500_000.55 → round 500_001 才真能区分 round vs floor）。
    assert_eq!(quota_display_to_internal(1.0000011).unwrap(), 500_001);
}

/// quota 非法值拒绝：负值/NaN/Infinity/溢出。
///
/// 为什么必须拒绝而非钳到 0：运营表单脏输入（手滑打负号或非数值）静默成 0
/// 会造出「免费无限套餐」；溢出截断会静默改写额度。BadRequest 才能把问题
/// 暴露给调用方（同 currency.rs 的口径护栏语义）。
#[test]
fn quota_conversion_rejects_invalid() {
    assert!(quota_display_to_internal(-1.0).is_err(), "negative quota");
    assert!(quota_display_to_internal(f64::NAN).is_err(), "NaN quota");
    assert!(
        quota_display_to_internal(f64::INFINITY).is_err(),
        "infinite quota"
    );
    assert!(
        quota_display_to_internal(1e18).is_err(),
        "quota overflowing i64 internal range must be rejected, not truncated"
    );
}

/// upsert 的 price 校验：NUMERIC 语义字符串非法值 → BadRequest。
///
/// 这是 [`SubscriptionService::upsert`] 的入口校验（price TEXT 列的护栏）：
/// 非法字符串写进库后，读侧 `price.parse::<f64>()` 会静默得到 None，前端
/// 显示空价格——所以在写入前用同一个 [`parse_price`] 拦下。
#[test]
fn price_validation_rejects_non_numeric() {
    assert!(parse_price("9.9").is_ok());
    assert!(parse_price("0").is_ok());
    assert!(parse_price(" 19.9 ").is_ok(), "前后空白 trim 后合法");
    assert!(parse_price("abc").is_err(), "非数值字符串");
    assert!(parse_price("").is_err(), "空串");
    assert!(parse_price("-1").is_err(), "负价格");
}

/// quota 往返自洽：写入侧 ×500_000 入库，读回侧 ÷500_000 还原，不漂移。
///
/// 为什么单钉这条：两侧换算不对称会让运营改一次套餐、列表显示值就翻
/// 500_000 倍（提交 1 → 显示 500_000），是肉眼可见的数据错乱。写侧
/// [`quota_display_to_internal`] 与读侧 `From<SubscriptionRow> for
/// SubscriptionDto` 必须互为逆运算。
#[test]
fn quota_roundtrip_is_symmetric() {
    let displayed = 9.9;
    let internal = quota_display_to_internal(displayed).unwrap();
    // 读回侧换算（与 From<SubscriptionRow> 的 quota: r.quota as f64 / UNITS_PER_DOLLAR 同式）
    let read_back = internal as f64 / 500_000.0;
    assert_eq!(
        read_back, displayed,
        "quota 往返必须还原：写 {displayed} → 内部 {internal} → 读 {read_back}"
    );

    // 整数美元的往返（无舍入误差的常见运营取值）。
    let internal_1 = quota_display_to_internal(1.0).unwrap();
    assert_eq!(internal_1 as f64 / 500_000.0, 1.0);
}
