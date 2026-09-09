//! 系统配置页:卡片式网格,对齐 GroupsPage / ChannelsPage / UsersPanel 规范。
//! 包含:顶部系统状态概览、模块分区卡片网格、站点通用信息表单、用户默认注册策略。

use crate::groups::{Badge, StatCard};
use crate::state::EntityStore;
use dioxus::prelude::*;
use ui::SegmentedCapsule;

const SEC_STATS: &str = "系统概览";
const SEC_BASE: &str = "站点信息与策略";
const SEC_FILTER: &str = "功能模块与开关";
const SEC_PROXY: &str = "出口代理节点";

/// 一条出口代理节点（对应网关 `[[proxy_nodes]]` 的 UI 编辑态）。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProxyNodeRow {
    /// 节点 URL，支持 http(s)/socks5(h)/ss/trojan/vless/vmess
    pub url: String,
    /// 绑定的渠道名（逗号分隔的输入态）
    pub channels: String,
    /// 优先级，数字越大越优先
    pub priority: String,
}

/// scheme 白名单：与网关 `ProxyNode::parse_url` 一致；非法 scheme 在输入侧直接红字提示。
const PROXY_SCHEMES: &[&str] = &[
    "http", "https", "socks5", "socks5h", "ss", "trojan", "vless", "vmess",
];

/// 校验单条节点 URL：scheme 必须在白名单内；vless 的 pbk/sid 有形状约束。
/// 返回 None 表示合法；Some(原因) 会在输入框下方红字提示。
pub(crate) fn validate_proxy_url(url: &str) -> Option<&'static str> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None; // 空行允许（还没填）
    }
    let (scheme, _) = trimmed.split_once("://")?;
    if !PROXY_SCHEMES.contains(&scheme) {
        return Some("不支持的协议（可用: http/https/socks5/socks5h/ss/trojan/vless/vmess）");
    }
    if scheme == "vless" {
        for (k, bad) in [
            ("pbk=", "REALITY 公钥 pbk 需 64 个 hex 字符"),
            ("sid=", "REALITY short id sid 最多 16 个 hex 字符"),
        ] {
            if let Some(v) = trimmed.split(k).nth(1).and_then(|s| s.split('&').next())
                && (hex::decode(v).is_err()
                    || (k == "pbk=" && v.len() != 64)
                    || (k == "sid=" && v.len() > 16))
            {
                return Some(bad);
            }
        }
    }
    None
}

fn proxy_row_id(idx: usize) -> String {
    format!("proxy-node-{idx}")
}
/// 单个开关项
#[derive(Clone, PartialEq)]
struct SwitchItem {
    key: &'static str,
    title: &'static str,
    desc: &'static str,
    group: &'static str,
}

const ALL_SWITCHES: &[SwitchItem] = &[
    SwitchItem {
        key: "站点公告",
        title: "站点公告横幅",
        desc: "在控制台顶端展示全局系统通知",
        group: "站点",
    },
    SwitchItem {
        key: "页头导航",
        title: "自定义页头导航",
        desc: "启用顶部自定义跳转链接与外部文档菜单",
        group: "站点",
    },
    SwitchItem {
        key: "货币与展示",
        title: "多币种与汇率展示",
        desc: "自动按人民币与美元双币种折算用量与套餐",
        group: "站点",
    },
    SwitchItem {
        key: "基础认证",
        title: "邮箱密码注册与登录",
        desc: "允许常规账密形式注册和 Argon2 鉴权",
        group: "认证",
    },
    SwitchItem {
        key: "OAuth 集成",
        title: "GitHub & LinuxDO 登录",
        desc: "一键关联第三方开放平台单点登录",
        group: "认证",
    },
    SwitchItem {
        key: "自定义 OAuth",
        title: "OIDC / CAS 协议接入",
        desc: "对接企业内部私有身份服务提供商",
        group: "认证",
    },
    SwitchItem {
        key: "Passkey",
        title: "WebAuthn 生物密钥",
        desc: "支持指纹/面容/FIDO2 硬件安全验证",
        group: "认证",
    },
    SwitchItem {
        key: "支付网关",
        title: "聚合在线收银台",
        desc: "支持微信、支付宝、Stripe 自动化上分",
        group: "计费",
    },
    SwitchItem {
        key: "签到奖励",
        title: "每日签到赠金",
        desc: "用户每日登录控制台领取随机额度奖励",
        group: "计费",
    },
    SwitchItem {
        key: "路由单位",
        title: "精细化 Token 结算",
        desc: "支持按请求次数或每 1k Token 独立计价",
        group: "计费",
    },
    SwitchItem {
        key: "机器人防护",
        title: "Cloudflare Turnstile",
        desc: "注册与登录行为验证码人机校验",
        group: "安全",
    },
    SwitchItem {
        key: "频率限制",
        title: "高频 IP 智能限速",
        desc: "防范恶意刷接口与暴力破解攻击",
        group: "安全",
    },
    SwitchItem {
        key: "敏感词",
        title: "敏感词实时过滤审计",
        desc: "多轮输入输出安全词典流式阻断",
        group: "安全",
    },
    SwitchItem {
        key: "SSRF 防护",
        title: "内网穿透与出口拦截",
        desc: "禁止渠道目标转发至私有网段",
        group: "安全",
    },
    SwitchItem {
        key: "公告",
        title: "系统全局广播系统",
        desc: "支持 Markdown 富文本弹窗与常驻通告",
        group: "内容",
    },
    SwitchItem {
        key: "FAQ",
        title: "常见问题知识库",
        desc: "在控制台前台公开常用调用与排障指引",
        group: "内容",
    },
    SwitchItem {
        key: "绘画",
        title: "Midjourney / DALL-E",
        desc: "启用文生图接口与操作代理面板",
        group: "内容",
    },
    SwitchItem {
        key: "侧边栏模块",
        title: "酒馆与工作台扩展",
        desc: "开启内置角色扮演与工作流集成",
        group: "内容",
    },
    SwitchItem {
        key: "日志维护",
        title: "调用详情链路日志",
        desc: "记录完整请求响应 Token 消耗追溯",
        group: "运维",
    },
    SwitchItem {
        key: "监控告警",
        title: "渠道熔断与宕机通知",
        desc: "通过 Telegram / 飞书即时通报警报",
        group: "运维",
    },
    SwitchItem {
        key: "性能",
        title: "缓存与内存预热",
        desc: "高频模型别名与路由就近命中优化",
        group: "运维",
    },
    SwitchItem {
        key: "Worker 代理",
        title: "CF Workers 边缘中继",
        desc: "使用无服务器节点分流上游流量",
        group: "运维",
    },
];

#[component]
pub fn SystemPage() -> Element {
    let store = use_context::<EntityStore>();
    let groups = store.groups;

    let mut search = use_signal(String::new);
    let mut filter_group = use_signal(|| 0usize);
    let mut save_flash = use_signal(|| false);

    // 站点基本配置
    let mut site_name = use_signal(|| "Ferrite New-API".to_string());
    let mut announcement = use_signal(|| "欢迎使用 Ferrite 统一大模型分发控制台".to_string());
    let mut def_group = use_signal(|| "default".to_string());
    let mut topup_rate = use_signal(|| "1.0".to_string());
    let mut contact_info = use_signal(|| "admin@ferrite.dev".to_string());

    // 出口代理节点（[[proxy_nodes]] 的 UI 编辑态；API 落地前先在本页保存态）
    let mut proxy_nodes = use_signal(|| {
        vec![ProxyNodeRow {
            url: "socks5://127.0.0.1:7890".to_string(),
            channels: "default".to_string(),
            priority: "10".to_string(),
        }]
    });
    let add_proxy_node = move |_| {
        proxy_nodes.write().push(ProxyNodeRow {
            url: String::new(),
            channels: String::new(),
            priority: "0".to_string(),
        });
    };
    let mut remove_proxy_node = move |idx: usize| {
        if proxy_nodes.read().len() > 1 {
            proxy_nodes.write().remove(idx);
        }
    };

    // 开关状态存储 (HashMap)
    let toggles = use_signal(|| {
        let mut m = std::collections::HashMap::<&'static str, bool>::new();
        m.insert("站点公告", true);
        m.insert("基础认证", true);
        m.insert("货币与展示", true);
        m.insert("签到奖励", true);
        m.insert("敏感词", true);
        m.insert("频率限制", true);
        m.insert("SSRF 防护", true);
        m.insert("日志维护", true);
        m.insert("监控告警", true);
        m
    });

    let group_categories = ["全部", "站点", "认证", "计费", "安全", "内容", "运维"];

    let total_switches = ALL_SWITCHES.len();
    let enabled_switches = ALL_SWITCHES
        .iter()
        .filter(|s| toggles.read().get(s.key).copied().unwrap_or(false))
        .count();
    let security_count = ALL_SWITCHES
        .iter()
        .filter(|s| s.group == "安全" && toggles.read().get(s.key).copied().unwrap_or(false))
        .count();
    let billing_count = ALL_SWITCHES
        .iter()
        .filter(|s| s.group == "计费" && toggles.read().get(s.key).copied().unwrap_or(false))
        .count();

    let stats: [(String, &str); 5] = [
        (total_switches.to_string(), "功能模块数"),
        (enabled_switches.to_string(), "已启用模块"),
        (format!("{security_count}/4"), "安全防护项"),
        (format!("{billing_count}/3"), "支付计费项"),
        (def_group(), "新用户默认分组"),
    ];

    let filter_options: Vec<String> = group_categories
        .iter()
        .map(|c| {
            if *c == "全部" {
                format!("全部 ({total_switches})")
            } else {
                let cnt = ALL_SWITCHES.iter().filter(|s| s.group == *c).count();
                format!("{c} ({cnt})")
            }
        })
        .collect();

    let filtered_switches: Vec<&'static SwitchItem> = {
        let q = search().trim().to_lowercase();
        let cat = group_categories[filter_group()];
        ALL_SWITCHES
            .iter()
            .filter(|s| {
                if !q.is_empty()
                    && !s.title.to_lowercase().contains(&q)
                    && !s.desc.to_lowercase().contains(&q)
                    && !s.group.to_lowercase().contains(&q)
                {
                    return false;
                }
                if cat != "全部" && s.group != cat {
                    return false;
                }
                true
            })
            .collect()
    };

    let save_base_info = move |_| {
        save_flash.set(true);
        spawn(async move {
            gloo_timers::future::TimeoutFuture::new(1200).await;
            save_flash.set(false);
        });
    };

    rsx! {
            div { class: "flex flex-col gap-6",
                // 1. 概览统计区
                section { id: "system-sec-stats", class: "scroll-mt-8 space-y-3",
                    h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
                    div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                        for (value, label) in stats {
                            StatCard { value, label }
                        }
                    }
                }

                // 2. 站点核心设置卡片 (替代原空荡荡的面板 2 与面板 3)
                section {
                    id: "system-sec-base",
                    class: "scroll-mt-8 rounded-xl border border-zinc-800 bg-zinc-900/60 p-5 space-y-4",
                    div { class: "flex items-center justify-between",
                        div {
                            h2 { class: "text-sm font-medium text-zinc-200", "{SEC_BASE}" }
                            p { class: "text-xs text-zinc-500", "配置平台全局展示名称、公告信息及注册落点策略" }
                        }
                        div { class: "flex items-center gap-3",
                            if save_flash() {
                                span { class: "text-xs text-emerald-400 animate-pulse", "✓ 配置已保存生效" }
                            }
                            button {
                                class: "rounded-xl bg-white px-4 py-2 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                                onclick: save_base_info,
                                "保存基础设置"
                            }
                        }
                    }

                    div { class: "grid grid-cols-1 md:grid-cols-3 gap-4 pt-1",
                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "站点名称" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                value: "{site_name}",
                                oninput: move |e| site_name.set(e.value()),
                            }
                        }
                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "默认用户分组" }
                            select {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                value: "{def_group}",
                                onchange: move |e| def_group.set(e.value()),
                                for g in groups.read().iter() {
                                    option { value: "{g.name}", "{g.name} ({g.display})" }
                                }
                            }
                        }
                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "充值汇率折算 (¥/USD)" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                                value: "{topup_rate}",
                                oninput: move |e| topup_rate.set(e.value()),
                            }
                        }
                    }

                    div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "全局顶部公告" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                value: "{announcement}",
                                oninput: move |e| announcement.set(e.value()),
                            }
                        }
                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "管理员联系邮箱" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                                value: "{contact_info}",
                                oninput: move |e| contact_info.set(e.value()),
                            }
                        }
                    }
                }

                // 3. 出口代理节点面板（[[proxy_nodes]]）
                section {
                    id: "system-sec-proxy",
                    class: "scroll-mt-8 rounded-xl border border-zinc-800 bg-zinc-900/60 p-5 space-y-4",
                    div { class: "flex items-center justify-between",
                        div {
                            h2 { class: "text-sm font-medium text-zinc-200", "data-testid": "heading-proxy-nodes", "{SEC_PROXY}" }
                            p { class: "text-xs text-zinc-500", "每行一个节点；渠道绑定用逗号分隔；vless 支持 ?flow=&sni=&pbk=&sid=（REALITY）。" }
                        }
                        button {
                            name: "btn-add-proxy-node",
                            class: "rounded-xl border border-zinc-600 px-3 py-1.5 text-xs text-zinc-200 transition-colors hover:border-zinc-400 hover:text-white",
                            onclick: add_proxy_node,
                            "＋ 添加节点"
                        }
                    }

                    div { class: "space-y-2",
                        for (idx, row) in proxy_nodes.read().iter().enumerate() {
                            {
                                let err = validate_proxy_url(&row.url);
                                let idx_capture = idx;
                                let row_url = row.url.clone();
                                let row_channels = row.channels.clone();
                                let row_priority = row.priority.clone();
                                rsx! {
                                    div {
                                        key: "{proxy_row_id(idx)}",
                                        class: "rounded-xl border border-zinc-800 bg-zinc-950/60 p-3 space-y-2",
                                        div { class: "grid grid-cols-1 gap-2 md:grid-cols-[1fr_180px_90px_auto]",
                                            div {
                                                label { class: "mb-1 block text-[11px] text-zinc-500", "节点 URL" }
                                                input {
                                                    name: "proxy-node-url-{idx}",
                                                    "data-testid": "proxy-node-url",
                                                    class: if err.is_some() {
                                                        "w-full rounded-lg border border-red-600/70 bg-zinc-950 px-3 py-1.5 text-xs text-zinc-100 font-mono focus:outline-none"
                                                    } else {
                                                        "w-full rounded-lg border border-zinc-700 bg-zinc-950 px-3 py-1.5 text-xs text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none"
                                                    },
                                                    placeholder: "vless://uuid@host:443?flow=xtls-rprx-vision&pbk=…&sni=…",
                                                    value: "{row_url}",
                                                    oninput: move |e| {
                                                        let mut rows = proxy_nodes.write();
                                                        rows[idx_capture].url = e.value();
                                                    },
                                                }
                                                if let Some(msg) = err {
                                                    p { class: "mt-1 text-[11px] text-red-400", "{msg}" }
                                                }
                                            }
                                            div {
                                                label { class: "mb-1 block text-[11px] text-zinc-500", "绑定渠道 (逗号分隔)" }
                                                input {
                                                    name: "proxy-node-channels-{idx}",
                                                    "data-testid": "proxy-node-channels",
                                                    class: "w-full rounded-lg border border-zinc-700 bg-zinc-950 px-3 py-1.5 text-xs text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                                    placeholder: "openai, claude",
                                                    value: "{row_channels}",
                                                    oninput: move |e| {
                                                        let mut rows = proxy_nodes.write();
                                                        rows[idx_capture].channels = e.value();
                                                    },
                                                }
                                            }
                                            div {
                                                label { class: "mb-1 block text-[11px] text-zinc-500", "优先级" }
                                                input {
                                                    name: "proxy-node-priority-{idx}",
                                                    "data-testid": "proxy-node-priority",
                                                    r#type: "number",
                                                    class: "w-full rounded-lg border border-zinc-700 bg-zinc-950 px-3 py-1.5 text-xs text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                                                    value: "{row_priority}",
                                                    oninput: move |e| {
                                                        let mut rows = proxy_nodes.write();
                                                        rows[idx_capture].priority = e.value();
                                                    },
                                                }
                                            }
                                            div { class: "flex items-end",
                                                button {
                                                    name: "btn-remove-proxy-node-{idx}",
                                                    "data-testid": "btn-remove-proxy-node",
                                                    class: "rounded-lg border border-zinc-700 px-2.5 py-1.5 text-xs text-zinc-400 transition-colors hover:border-red-500 hover:text-red-400",
                                                    onclick: move |_| remove_proxy_node(idx_capture),
                                                    "删除"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 4. 模块开关筛选与卡片区
                section {
                    id: "system-sec-filter",
                    class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
                    div { class: "flex items-center justify-between gap-3",
                        div { class: "flex items-center gap-2",
                            h2 { class: "text-sm font-medium text-zinc-300", "{SEC_FILTER}" }
                            span { class: "text-xs text-zinc-500", "点击开关即时切换各功能与服务子域" }
                        }
                        span { class: "text-xs text-zinc-400",
                            "共 {filtered_switches.len()} 项"
                        }
                    }

                    input {
                        class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 outline-none transition focus:border-zinc-500",
                        r#type: "text",
                        placeholder: "搜索模块标题、关键词或描述 (如 安全, 验证码, SSRF, 支付)...",
                        value: "{search}",
                        oninput: move |e| search.set(e.value()),
                    }

                    div { class: "flex flex-wrap gap-2",
                        SegmentedCapsule {
                            items: filter_options,
                            active: filter_group(),
                            on_select: move |i: usize| filter_group.set(i),
                        }
                    }
                }

                // 4. 卡片网格展示
                section { class: "space-y-4",
                    if filtered_switches.is_empty() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "没有匹配的功能模块" }
                        }
                    } else {
                        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                            for sw in filtered_switches {
                                {
                                    let is_on = toggles.read().get(sw.key).copied().unwrap_or(false);
                                    let k = sw.key;
                                    rsx! {
                                        SwitchCard {
                                            key: "{sw.key}",
                                            item: sw,
                                            is_on: is_on,
                                            on_toggle: move |_| {
                                                let mut t = toggles;
                                                let cur = t.read().get(k).copied().unwrap_or(false);
                                                t.write().insert(k, !cur);
                                            },
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
    }
}

/// 功能开关卡片 (严格对齐统一卡片规范)
#[component]
fn SwitchCard(item: &'static SwitchItem, is_on: bool, on_toggle: EventHandler<()>) -> Element {
    let initial = item
        .group
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

    let (status_text, status_tone, group_tone, bar_tone) = if is_on {
        (
            "已开启",
            "border-emerald-500/30 bg-emerald-500/20 text-emerald-400",
            "border-zinc-700 bg-zinc-800/80 text-zinc-300",
            "bg-emerald-500",
        )
    } else {
        (
            "未启用",
            "border-zinc-700 bg-zinc-800/80 text-zinc-500",
            "border-zinc-800 bg-zinc-900 text-zinc-500",
            "bg-zinc-700",
        )
    };

    rsx! {
        div { class: "group flex flex-col justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            div { class: "space-y-3",
                // 头部
                div { class: "flex items-start gap-3",
                    div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-zinc-700 bg-zinc-800 text-sm font-semibold text-zinc-200 group-hover:border-zinc-500 transition-colors",
                        "{initial}"
                    }
                    div { class: "min-w-0 flex-1",
                        div { class: "flex items-center justify-between gap-2",
                            h3 { class: "truncate text-sm font-medium text-zinc-100", "{item.title}" }
                            span { class: "shrink-0 rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-mono text-zinc-400 border border-zinc-700/60",
                                "{item.group}"
                            }
                        }
                        p { class: "mt-0.5 truncate text-[11px] text-zinc-400", "{item.key}" }
                    }
                }

                // 徽标行
                div { class: "flex flex-wrap gap-1.5",
                    Badge { text: status_text.to_string(), tone: status_tone }
                    Badge { text: format!("{}域", item.group), tone: group_tone }
                }

                // 状态指示条
                div { class: "space-y-1.5",
                    div { class: "flex justify-between gap-2 text-[11px]",
                        span { class: "text-zinc-400", "运行状态" }
                        span { class: if is_on { "font-medium text-emerald-400" } else { "font-medium text-zinc-500" },
                            "{status_text}"
                        }
                    }
                    div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                        div { class: "h-full rounded-full {bar_tone} transition-all duration-300", style: if is_on { "width: 100%" } else { "width: 8%" } }
                    }
                }

                // 说明文本行
                div { class: "text-xs pt-1",
                    p { class: "text-zinc-400 leading-relaxed min-h-[36px]", "{item.desc}" }
                }
            }

            // 底部操作区 (开关按钮)
            div { class: "mt-4 border-t border-zinc-800 pt-3",
                button {
                    class: if is_on {
                        "w-full rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-2 text-xs text-amber-400 transition-colors hover:bg-zinc-700 hover:text-amber-300 font-medium"
                    } else {
                        "w-full rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-2 text-xs text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white"
                    },
                    onclick: move |_| on_toggle.call(()),
                    if is_on { "点击停用模块" } else { "点击启用模块" }
                }
            }
        }
    }
}
