//! 配置接线测试：证明 config.toml 的值真的到达运行期组件，不只是能解析。
//!
//! 单纯断言 serde 默认值只能证明结构体字段存在；这里断言的是配置驱动了
//! 行为差异（冷却阈值改变熔断时机、价格表有无决定是否计费）。

use dispatch::health::{FailureClass, HealthTable, MemoryHealthTable};
use gateway::config::GatewayConfig;
use std::io::Write;

/// 把 TOML 写到临时文件再 load，走真实的 `GatewayConfig::load` 路径。
///
/// 文件名带原子计数器：测试并行跑，同名文件会互相覆盖。
fn load_toml(body: &str) -> GatewayConfig {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("ferrite-cfg-{}-{n}.toml", std::process::id()));
    let mut f = std::fs::File::create(&path).expect("create temp config");
    f.write_all(body.as_bytes()).expect("write temp config");
    let cfg = GatewayConfig::load(&path).expect("load config");
    let _ = std::fs::remove_file(&path);
    cfg
}
/// 空配置走全套默认值。整段 `[dispatch]` / `[retry]` / `[metering]` 可省。
#[test]
fn empty_config_uses_defaults() {
    let cfg = load_toml("");
    assert_eq!(cfg.listen, "0.0.0.0:3000");
    assert_eq!(cfg.log_level, "info");
    assert_eq!(cfg.dispatch.cooldown_threshold, 5);
    assert_eq!(cfg.dispatch.cooldown_base_seconds, 10);
    assert_eq!(cfg.dispatch.cooldown_max_seconds, 60);
    assert_eq!(cfg.retry.max_attempts, 3);
    assert!(cfg.metering.prices.is_empty());
}

/// `[dispatch].cooldown_threshold` 必须真的改变熔断时机。
///
/// 这是接线测试的核心：配置能解析不等于配置起作用。阈值设为 2 时第 2 次
/// 连续失败就该进冷却；默认 5 时同样两次失败仍可选。
#[test]
fn dispatch_cooldown_threshold_changes_circuit_timing() {
    let cfg = load_toml(
        r#"
[dispatch]
cooldown_threshold = 2
cooldown_base_seconds = 30
cooldown_max_seconds = 30
"#,
    );
    let tuned = MemoryHealthTable::with_config(dispatch::health::HealthSetting {
        cooldown_threshold: cfg.dispatch.cooldown_threshold,
        cooldown_base_seconds: cfg.dispatch.cooldown_base_seconds,
        cooldown_max_seconds: cfg.dispatch.cooldown_max_seconds,
        ..Default::default()
    });
    let now = 1_000_000;
    // 两次传输层失败 → 阈值 2 已达，进冷却。
    tuned.record("ch1", Err(FailureClass::Retryable));
    tuned.record("ch1", Err(FailureClass::Retryable));
    assert!(
        !tuned.is_selectable("ch1", now),
        "cooldown_threshold=2 时两次失败应进冷却"
    );

    // 对照：默认阈值 5，同样两次失败仍可选。证明差异来自配置而非固定行为。
    let default_table = MemoryHealthTable::new();
    default_table.record("ch1", Err(FailureClass::Retryable));
    default_table.record("ch1", Err(FailureClass::Retryable));
    assert!(
        default_table.is_selectable("ch1", now),
        "默认阈值 5 时两次失败不该进冷却"
    );
}

/// `[metering.prices]` 为空 → 无价格表（本地单机不计费）；非空 → 可查到价格。
#[test]
fn metering_prices_drive_price_table_presence() {
    let bare = load_toml("");
    assert!(
        gateway::build_price_table(&bare).is_none(),
        "prices 为空时不该装配价格表，否则本地单机会被计费"
    );

    let priced = load_toml(
        r#"
[metering.prices."gpt-4o"]
input = 2.5
output = 10.0
cache = 1.25
"#,
    );
    let table = gateway::build_price_table(&priced).expect("prices 非空应装配价格表");
    let p = table.lookup("gpt-4o").expect("配置里的模型应查得到");
    assert_eq!(p.input, 2.5);
    assert_eq!(p.output, 10.0);
    assert_eq!(p.cache, 1.25);
    // group_multiplier 省略时取默认 1.0，不是 f64::default() 的 0.0
    // ——否则所有费用会被乘成 0。
    assert_eq!(p.group_multiplier, 1.0);
    assert!(table.lookup("unknown-model").is_none());
}

/// `[retry].max_attempts` 传到 `RetryPolicy`。
#[test]
fn retry_max_attempts_reaches_policy() {
    let cfg = load_toml("[retry]\nmax_attempts = 7\n");
    assert_eq!(gateway::build_retry_policy(&cfg).max_attempts, 7);

    let cfg = load_toml("");
    assert_eq!(gateway::build_retry_policy(&cfg).max_attempts, 3);
}

/// `build_app` 接受配置并成功组装（不 panic），冷却参数走配置路径。
#[test]
fn build_app_accepts_config() {
    let cfg = load_toml("[dispatch]\ncooldown_threshold = 1\n");
    let _router = gateway::build_app(&cfg);
}

/// `[[channels]]` 必须真的变成可选中的路由：dispatch 用 `(group, model)` 查候选，
/// 并且 `upstream_models` 的映射要生效——上游收到的是真名而非公开别名。
#[test]
fn channels_config_drives_selectable_routes() {
    use dispatch::Dispatch;

    let cfg = load_toml(
        r#"
[[channels]]
name = "primary"
base_url = "https://primary.example"
api_key = "sk-primary"
models = ["gpt-4o", "gpt-4"]
upstream_models = { "gpt-4" = "gpt-4-0613" }

[[channels]]
name = "claude-ch"
provider_type = "claude"
base_url = "https://claude.example"
api_key = "sk-claude"
models = ["claude-sonnet"]
"#,
    );
    let snapshot = gateway::load_snapshot(&cfg);

    // 行为断言：不关心内部 unit/channel 精确数量，只关心关键模型可选且正确。
    assert!(snapshot.units.len() > 0, "配置了渠道应产出路由单元");
    assert!(snapshot.channels.len() > 0, "配置了渠道应产出渠道记录");

    let dispatcher = dispatch::Dispatcher::new(
        Some(snapshot),
        std::sync::Arc::new(dispatch::MemoryHealthTable::new()),
    );

    // 无映射的模型：上游真名等于公开名，凭据取自所属渠道。
    let picked = dispatcher
        .select("default", "gpt-4o", &[])
        .expect("应选中 primary");
    assert_eq!(picked.base_url, "https://primary.example");
    assert_eq!(picked.secret, "sk-primary");
    assert_eq!(picked.upstream_model, "gpt-4o");
    assert_eq!(
        picked.provider_type, "openai",
        "未指定 provider_type 应缺省 openai"
    );

    // 有映射的模型：发往上游的是真名，客户端看到的仍是公开别名。
    let mapped = dispatcher
        .select("default", "gpt-4", &[])
        .expect("应选中 primary");
    assert_eq!(mapped.upstream_model, "gpt-4-0613");

    // provider_type 随渠道走，决定 forward 的鉴权头与路径模板。
    let claude = dispatcher
        .select("default", "claude-sonnet", &[])
        .expect("应选中 claude-ch");
    assert_eq!(claude.provider_type, "claude");
    assert_eq!(claude.secret, "sk-claude");

    // 配置里没有的模型选不出候选。
    assert!(dispatcher.select("default", "not-configured", &[]).is_err());
}

/// `[[keys]]` 必须能被 `AuthGate` 的 sha256 查表命中，且 `allowed_models`
/// 为空时存 `None`（= 不限制），非空时原样保留供 `ModelGate` 比对。
#[test]
fn keys_config_drives_token_lookup() {
    let cfg = load_toml(
        r#"
[[keys]]
key = "sk-local-alpha"
name = "alpha"

[[keys]]
key = "sk-local-beta"
group = "vip"
allowed_models = ["gpt-4*"]
"#,
    );

    let (tokens, users) = gateway::config::build_token_snapshot(&cfg.keys);

    // 查表口径就是 gate 用的 sha256(明文)。
    let alpha = tokens
        .lookup(&gateway_gate::sha256("sk-local-alpha"))
        .expect("明文 key 的 sha256 应命中");
    assert!(
        alpha.allowed_models.is_none(),
        "空 allowed_models 应为 None = 不限制"
    );

    let beta = tokens
        .lookup(&gateway_gate::sha256("sk-local-beta"))
        .expect("第二把 key 也应命中");
    assert_eq!(
        beta.allowed_models.as_deref(),
        Some(["gpt-4*".to_string()].as_slice())
    );
    assert_eq!(beta.record.group.as_deref(), Some("vip"));

    // 未配置的 key 查不到 → AuthGate 返回 401。
    assert!(
        tokens
            .lookup(&gateway_gate::sha256("sk-not-configured"))
            .is_none()
    );

    // StateGate 要用 token.user_key 查到启用的用户，否则恒 401 UserNotFound。
    let user = users
        .lookup(&alpha.record.user_key)
        .expect("token 指向的用户必须存在");
    assert_eq!(user.status, 1, "本地用户必须是启用状态");
}

/// 空配置不 panic，产出空快照：转发请求 404 no_route、鉴权 401，而不是崩溃。
#[test]
fn empty_channels_and_keys_yield_empty_snapshots() {
    let cfg = load_toml("");
    let snapshot = gateway::load_snapshot(&cfg);
    assert!(snapshot.units.is_empty());
    assert!(snapshot.channels.is_empty());

    let (tokens, _users) = gateway::config::build_token_snapshot(&cfg.keys);
    assert!(tokens.lookup(&gateway_gate::sha256("anything")).is_none());
}

/// 额度闸只在计费模式挂上。`QuotaSnapshot::remaining` 对未登记 token 返回 0，
/// 单机不计费时若挂了 `QuotaGate`，每个请求都会被判 402。
#[test]
fn quota_gate_mounts_only_when_priced() {
    let bare = load_toml("");
    let priced =
        load_toml("[metering.prices.\"gpt-4o\"]\ninput = 1.0\noutput = 2.0\ncache = 0.5\n");
    assert_eq!(
        gateway::build_gates(&priced).len(),
        gateway::build_gates(&bare).len() + 1,
        "计费模式应多挂一道额度闸"
    );
}
