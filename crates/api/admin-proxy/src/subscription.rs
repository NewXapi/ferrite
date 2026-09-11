//! `subscription` —— 订阅批量导入与分享链接批量导入。
//!
//! 订阅拉取与 YAML 解析**全部委托** `meow_config::subscription`：它已处理 HTTP
//! 拉取（带 UA 与体积上限）、`<<: *anchor` merge 键展开、`proxies` 序列提取。
//! 本模块只做 clash proxy map → `proxy_nodes` 行的映射。
//!
//! `SubscriptionData` 的 `proxy_groups` / `rules` 解析后直接丢弃：我们的选点语义是
//! 「渠道 → 节点集合 + priority」（见 `gateway_proxy::pool`），不需要 clash 的
//! url-test / fallback 组，也不需要域名规则路由。
//!
//! # `clash_proxy_to_url` 是有损映射
//!
//! `proxy_nodes.url` 是分享链接形态的 TEXT，表达力比 clash map 窄：`grpc-opts`、
//! `mux`、`smux`、`ech-opts`、`plugin`、非空 `alpn`（单值也带不上）在我们的 URL
//! query 里**没有对应键**。
//!
//! 约定：遇到无法表达的键 → 返回 `Err`，调用方记为 [`ImportFailure`]。宁可导入失败
//! 让用户知道，不要静默丢配置——一个丢了传输层配置的节点会拨号成功但走错传输，
//! 排查成本远高于导入时报错。
//!
//! 真出现大量此类节点时的升级路径：给 `proxy_nodes` 加 `clash_config JSONB` 列，
//! `load_proxy_snapshot` 优先用它，`adapter_for` 加 `adapter_from_clash(map)` 入口。
//! 代价是 `ProxyNode` 不再是唯一真相，多一条装配路径要同步维护。

use std::collections::HashMap;

use gateway_proxy::sharelink::{mask_link, parse_share_link};
use meow_config::subscription::{SubscriptionData, fetch_subscription, parse_subscription_yaml};
use serde::{Deserialize, Serialize};
use serde_yaml::Value as Yaml;

use crate::{ProxyNodeService, ServiceError};

/// 导入请求：订阅 URL 或直接粘贴的文本，二选一。
///
/// `url` 与 `text` 同时为 `None` 或同时有值都是 400（歧义请求不猜意图）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRequest {
    /// 订阅 URL（clash YAML 订阅）。
    #[serde(default)]
    pub url: Option<String>,
    /// 直接粘贴的内容：订阅 YAML 原文，或多行分享链接（取决于调用的端点）。
    #[serde(default)]
    pub text: Option<String>,
    /// 导入的节点统一绑定到这些渠道。空数组是 400——空绑定的节点永远不会被选中。
    pub channel_keys: Vec<String>,
    /// 导入的节点统一优先级。
    #[serde(default)]
    pub priority: i32,
}

/// 单条导入失败。
///
/// `source` 必须是**掩码后**的形态：订阅与分享链接里都带密码 / UUID，
/// 原样回显等于把凭据写进前端和日志。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFailure {
    /// 掩码后的来源标识（`vless://***@host:port`，或订阅里的节点名）。
    pub source: String,
    /// 失败原因：不支持的协议 / 有损映射拒绝 / 校验失败 / DB 冲突。
    pub reason: String,
}

/// 导入结果逐条报告。
///
/// 为什么不只回计数：用户需要知道**哪个**节点**为什么**没进去。
/// 「导入 40 个成功 31 个」这种回复无法排查——剩下 9 个是协议不支持、
/// 渠道名写错、还是有损映射拒绝？
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    /// 成功入库条数。
    pub created: usize,
    /// 跳过条数。`proxy_nodes` 对 `url` 没有唯一约束（同 URL 可重复入库），
    /// 当前恒为 0；字段留给 M3 的订阅 diff 去重用。
    pub skipped: usize,
    /// 失败明细。
    pub failures: Vec<ImportFailure>,
}

/// clash proxy map → 分享链接形态 URL（有损，见模块头）。
///
/// # 参数
/// `proxy` 是 `meow_config::subscription::SubscriptionData::proxies` 的一个元素，
/// 键名遵循 clash/mihomo 约定：`type` / `server` / `port` / `uuid` / `password` /
/// `cipher` / `ws-opts` / `reality-opts` / `client-fingerprint` / `sni` / `flow`。
///
/// # 错误
/// - 缺 `type` / `server` / `port` / 对应协议的凭据字段：无法构造节点
/// - `type` 不在 7 协议白名单内（如 tuic / wireguard）
/// - URL query 无法表达的键（`grpc-opts` / `mux` / `smux` / `ech-opts` /
///   `plugin` / 非 ws 的 `network` / 非空 `alpn`）：**拒绝而非丢弃**
///
/// # 映射约定（与 `gateway_proxy::adapter::clash_config` 反向一致）
/// ss → `ss://cipher:password@`（两段）；vless / vmess → `scheme://uuid@`；
/// trojan / hysteria2 / anytls / snell → `scheme://password@`（单段）。
/// 搞错段落位置会导致装配后认证失败，#98 已在 trojan 上踩过一次。
pub fn clash_proxy_to_url(proxy: &HashMap<String, Yaml>) -> Result<String, String> {
    let get_str = |k: &str| {
        proxy.get(k).and_then(|v| match v {
            Yaml::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
            _ => None,
        })
    };

    let ty = get_str("type").ok_or("缺少 type")?;
    let server = get_str("server").ok_or("缺少 server")?;
    let port: u16 = match proxy.get("port") {
        Some(Yaml::Number(n)) => n
            .as_u64()
            .and_then(|v| u16::try_from(v).ok())
            .ok_or("port 非法")?,
        Some(Yaml::String(s)) => s.trim().parse().map_err(|_| "port 非法")?,
        _ => return Err("缺少 port".to_string()),
    };

    if !matches!(
        ty.as_str(),
        "ss" | "trojan" | "vless" | "vmess" | "hysteria2" | "anytls" | "snell"
    ) {
        return Err(format!("协议不支持: {ty}"));
    }

    // 有损拒绝（模块头约定）。原因里点名键，用户才知道删哪个能救回来。
    for bad in [
        "grpc-opts",
        "mux",
        "smux",
        "ech-opts",
        "plugin",
        "plugin-opts",
    ] {
        if proxy.contains_key(bad) {
            return Err(format!(
                "URL 无法表达 `{bad}`，拒绝导入（静默丢弃会走错配置）"
            ));
        }
    }
    match proxy.get("network").and_then(|v| v.as_str()) {
        Some("ws") | None => {}
        Some(other) => return Err(format!("network={other} 无法用 URL 表达")),
    }
    // alpn 只要非空就拒（我们的 URL query 没有 alpn 键，单值也带不上）。
    // 非序列形态（订阅写错成字符串）同样视为非空拒掉。
    let alpn_empty = match proxy.get("alpn") {
        None | Some(Yaml::Null) => true,
        Some(Yaml::Sequence(s)) => s.is_empty(),
        Some(_) => false,
    };
    if !alpn_empty {
        return Err("URL 无法表达 `alpn`（非空，单值也带不上），拒绝导入".to_string());
    }
    // network=ws 时 path 缺省为 "/"（clash/mihomo 默认，与 sharelink vmess 方言一致）。
    // parse_url 靠 `path=` 识别 ws，只要发出去就行。
    let ws_path = if proxy.get("network").and_then(|v| v.as_str()) == Some("ws") {
        proxy
            .get("ws-opts")
            .and_then(|v| v.get("path"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("/")
            .to_string()
    } else {
        String::new()
    };
    // userinfo 按协议段落（见函数 rustdoc）。url crate 的 set_username /
    // set_password 会 percent-encode，凭据里的 `@ : / ?` 都能活过 round-trip。
    let mut url = url::Url::parse(&format!("{ty}://{server}:{port}"))
        .map_err(|e| format!("构造 URL 失败: {e}"))?;
    let set_creds = |url: &mut url::Url| -> Result<(), String> {
        match ty.as_str() {
            "ss" => {
                let cipher = get_str("cipher").ok_or("ss 缺少 cipher")?;
                let password = get_str("password").ok_or("ss 缺少 password")?;
                url.set_username(&cipher).map_err(|_| "cipher 编码失败")?;
                url.set_password(Some(&password))
                    .map_err(|_| "password 编码失败")?;
            }
            "vless" | "vmess" => {
                let uuid = get_str("uuid").ok_or("vless/vmess 缺少 uuid")?;
                url.set_username(&uuid).map_err(|_| "uuid 编码失败")?;
                // vmess 的加密方式放 auth.pass（与 parse_url 的 `uuid:cipher@` 对齐）；
                // auto / 缺省留空让 meow-config 取缺省。
                let cipher = get_str("cipher").unwrap_or_default();
                if !cipher.is_empty() && !cipher.eq_ignore_ascii_case("auto") {
                    url.set_password(Some(&cipher))
                        .map_err(|_| "cipher 编码失败")?;
                }
            }
            "snell" => {
                let psk = get_str("psk").ok_or("snell 缺少 psk")?;
                url.set_username(&psk).map_err(|_| "psk 编码失败")?;
            }
            _ => {
                // trojan / hysteria2 / anytls：密码是单段 userinfo。
                let password = get_str("password").ok_or("缺少 password")?;
                url.set_username(&password)
                    .map_err(|_| "password 编码失败")?;
            }
        }
        Ok(())
    };
    set_creds(&mut url)?;

    // query 用 query_pairs_mut 写入：值里的 `& = %` 会被正确编码，
    // 手拼字符串会在含特殊字符的 path / sni 上断掉。
    {
        let mut q = url.query_pairs_mut();
        let sni = get_str("sni").or_else(|| get_str("servername"));
        if let Some(sni) = sni {
            q.append_pair("sni", &sni);
        }
        if proxy.get("skip-cert-verify") == Some(&Yaml::Bool(true)) {
            q.append_pair("insecure", "1");
        }
        if let Some(Yaml::Mapping(m)) = proxy.get("reality-opts") {
            let key = Yaml::String("public-key".into());
            if let Some(Yaml::String(pbk)) = m.get(&key) {
                q.append_pair("pbk", pbk);
            }
            let key = Yaml::String("short-id".into());
            if let Some(Yaml::String(sid)) = m.get(&key) {
                q.append_pair("sid", sid);
            }
        }
        if let Some(fp) = get_str("client-fingerprint") {
            q.append_pair("fp", &fp);
        }
        if let Some(flow) = get_str("flow") {
            q.append_pair("flow", &flow);
        }
        if !ws_path.is_empty() {
            q.append_pair("path", &ws_path);
            if let Some(Yaml::Mapping(hs)) = proxy.get("ws-opts") {
                let key = Yaml::String("headers".into());
                if let Some(Yaml::Mapping(headers)) = hs.get(&key) {
                    let host = Yaml::String("Host".into());
                    if let Some(Yaml::String(h)) = headers.get(&host) {
                        q.append_pair("host", h);
                    }
                }
            }
        }
        if let Some(Yaml::Number(v)) = proxy.get("version") {
            q.append_pair("version", &v.to_string());
        }
        if let Some(Yaml::Mapping(o)) = proxy.get("obfs-opts") {
            let mode = Yaml::String("mode".into());
            if let Some(Yaml::String(m)) = o.get(&mode) {
                q.append_pair("obfs", m);
            }
            let host = Yaml::String("host".into());
            if let Some(Yaml::String(h)) = o.get(&host) {
                q.append_pair("obfs-uri", h);
            }
        }
    }
    Ok(url.to_string())
}

/// 请求体合法性：url 与 text 必须恰好给一个，channel_keys 不能为空。
///
/// 放在循环外做一次：40 个节点共用一个坏请求，报 40 条一样的 failure 不如直接 400。
fn validate_request(req: &ImportRequest) -> Result<(), ServiceError> {
    if req.url.is_some() == req.text.is_some() {
        return Err(ServiceError::BadRequest(
            "url 与 text 必须恰好给一个（都不给或都给都是歧义请求）".into(),
        ));
    }
    if req.channel_keys.is_empty() {
        return Err(ServiceError::BadRequest(
            "channel_keys 不能为空（空 = 永远不会被任何渠道选中）".into(),
        ));
    }
    Ok(())
}

/// 导入订阅：拉取 / 解析 → 逐条 [`clash_proxy_to_url`] → 复用
/// [`ProxyNodeService::create`]。
///
/// 复用 `create` 而不新写 INSERT：URL 校验、渠道引用完整性都在里面，
/// 绕过它等于把两处逻辑抄第二遍。
///
/// # 错误
/// 整体失败（返回 `Err`）只有三种情况：请求歧义、channel_keys 为空、
/// 订阅拉取或 YAML 解析失败。**单个节点的失败不算整体失败**——进
/// [`ImportReport::failures`]，其余节点继续导入。
pub async fn import_subscription(
    svc: &ProxyNodeService,
    req: &ImportRequest,
) -> Result<ImportReport, ServiceError> {
    validate_request(req)?;

    let data: SubscriptionData = match (req.url.as_deref(), req.text.as_deref()) {
        (Some(url), _) => fetch_subscription(url)
            .await
            .map_err(|e| ServiceError::BadRequest(format!("拉取订阅失败: {e}")))?,
        (_, Some(text)) => parse_subscription_yaml(text)
            .map_err(|e| ServiceError::BadRequest(format!("解析订阅文本失败: {e}")))?,
        (None, None) => return Err(ServiceError::BadRequest("url 与 text 必须给一个".into())),
    };

    let mut report = ImportReport::default();
    for proxy in &data.proxies {
        // 订阅里的节点名不是凭据，失败时原样回显便于定位。
        let name = proxy
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("未命名节点")
            .to_string();
        match clash_proxy_to_url(proxy) {
            Ok(url) => match svc
                .create(&name, &url, &req.channel_keys, req.priority, "")
                .await
            {
                Ok(_) => report.created += 1,
                Err(e) => report.failures.push(ImportFailure {
                    source: name,
                    reason: e.to_string(),
                }),
            },
            Err(reason) => report.failures.push(ImportFailure {
                source: name,
                reason,
            }),
        }
    }
    Ok(report)
}

/// 导入粘贴的多行分享链接。
///
/// 逐行调 `parse_share_link` 而不是用批量入口 `parse_share_links`：入库要用
/// **原始行**——`ProxyNode` 是解析产物，反向拼 URL 会丢 query 里的未识别键
/// （骨架期就写明的约定），原始行本身就是合法分享链接，存它即可。
///
/// 节点名取链接的 `#备注`（v2rayN / SIP002 惯例），没有备注就用掩码形态。
///
/// # 错误
/// 与 [`import_subscription`] 同：整体失败仅限请求歧义 / channel_keys 为空；
/// 单行失败进 report。
pub async fn import_share_links(
    svc: &ProxyNodeService,
    req: &ImportRequest,
) -> Result<ImportReport, ServiceError> {
    validate_request(req)?;
    let text = req.text.as_deref().ok_or_else(|| {
        ServiceError::BadRequest("分享链接导入需要 text（订阅 URL 请走 /subscription）".into())
    })?;

    let mut report = ImportReport::default();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let masked = mask_link(line);
        match parse_share_link(line) {
            Ok(_node) => {
                let name = link_name(line);
                match svc
                    .create(&name, line, &req.channel_keys, req.priority, "")
                    .await
                {
                    Ok(_) => report.created += 1,
                    Err(e) => report.failures.push(ImportFailure {
                        source: masked,
                        reason: e.to_string(),
                    }),
                }
            }
            Err(e) => report.failures.push(ImportFailure {
                source: masked,
                reason: e.to_string(),
            }),
        }
    }
    Ok(report)
}

/// 从分享链接提取人读名字：`#` 后的备注（percent-decode），没有就用掩码形态。
fn link_name(line: &str) -> String {
    let remark = line
        .split_once('#')
        .map(|(_, r)| {
            percent_encoding::percent_decode_str(r)
                .decode_utf8_lossy()
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty());
    remark.unwrap_or_else(|| mask_link(line))
}
