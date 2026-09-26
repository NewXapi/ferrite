//! 拓扑页 drawer（节点 / 设置 / 导入 tab）的写路径。
//!
//! #183 之后拓扑画布不再由 `EntityStore` 驱动（读侧走
//! `network::load_network_data` 三端点并发拉真实数据），drawer 里的编辑 /
//! 删除 / 导入因此不能只写本地 store 行——那部分改动在刷新后即被丢弃。
//! 本模块把写操作直接接到 `crate::api` 的真实端点（#175/#182 建好）：
//! - 分组展示名（remark 列）→ `update_group_api`（分组名锁读：后端
//!   `UpdateGroupRequest` 只有 ratio/model_whitelist/remark/status，改名
//!   无更新路径，UI 侧诚实锁读不静默丢写）；
//! - 渠道 name/url/keys → `update_channel_api` 最小 diff 体
//!   （`api::UpdateChannelBody`：keys 缺席 = 后端 COALESCE 保持现有密钥）；
//! - 渠道 / 分组删除 → `delete_channel_api` / `delete_group_api`，
//!   调用方须先经 `ui::dialog::Dialog` 确认弹窗；
//! - 导入新渠道 → `create_channel_api`（全量 `ChannelUpsertRequest`）。
//!
//! 掩码不回传（硬约定）：`/api/channel` 列表端点不携带 keys，抽屉里能显示
//! 的 keys 要么是用户自己输入的明文，要么是单查回显的掩码值。所有
//! 渠道写函数只接受用户本次输入的明文 keys；未编辑时 keys 字段整体缺席
//! 请求体，绝不用旧值 / 掩码值充当新值。

use client::ApiClient;
use contract::api::admin::{ChannelDto, ChannelUpsertRequest, GroupDto, GroupUpsertRequest};
use dioxus::prelude::*;

use crate::api::{
    UpdateChannelBody, create_channel_api, create_group_api, delete_channel_api, delete_group_api,
    get_channel_api, list_channels_api, list_groups_api, set_channel_status_api,
    update_channel_api, update_group_api,
};
use crate::state::CHANNEL_TYPES;

/// 一次写操作的结果：错误摘要（`ApiError` 的 Display）由调用方在抽屉内
/// 渲染 `role=alert` 红边卡。
type WriteResult<T> = Result<T, String>;

/// `ApiError` → 抽屉错误摘要（统一 `?` 转换，避免逐处 `map_err`）。
fn api_err_to_string<E: core::fmt::Display>(e: E) -> String {
    e.to_string()
}

// ---------------------------------------------------------------------------
// 分组
// ---------------------------------------------------------------------------

/// 按分组名找真实分组（列表端点无 name 过滤，本地匹配），取回
/// 服务端 key / 现值，供改名透传与删除定位。
///
/// `Ok(None)` = 列表拉取成功但无此名（本地草稿行 / 已被他人删除）；
/// `Err` = API 失败（401 / 网络 / 5xx）。两者必须区分：前者才允许
/// 「按新建处理」或「删本地行」，后者一律报错，绝不静默降级成新建
/// （否则一次瞬时网络故障就会复制出重复分组）。
pub async fn find_group_by_name(name: &str) -> WriteResult<Option<GroupDto>> {
    let client = ApiClient::shared().clone();
    let groups = list_groups_api(&client).await.map_err(api_err_to_string)?;
    Ok(groups.into_iter().find(|g| g.name == name))
}

/// 新建一个真实分组（名称 + 展示名/备注 + 倍率）。
/// 设置页「分组」卡片的新增入口：此前只写本地 store，刷新即丢。
pub async fn create_group_write(
    name: &str,
    display: &str,
    multiplier: f64,
) -> WriteResult<GroupDto> {
    let client = ApiClient::shared().clone();
    let req = GroupUpsertRequest {
        name: name.to_string(),
        ratio: multiplier,
        model_whitelist: serde_json::json!([]),
        remark: display.to_string(),
    };
    create_group_api(&client, &req)
        .await
        .map_err(api_err_to_string)
}

/// 按分组 key 更新展示名（`remark` 列，后端 COALESCE 可写列）。
///
/// `g` 为调用方 [`find_group_by_name`] 取回的服务端现值，原样透传
/// `name` / `ratio` / `model_whitelist`（后端 `UpdateGroupRequest` 无
/// `name` 列——分组改名无更新路径，锁读；透传现值不置空不猜默认）。
pub async fn update_group_display(g: &GroupDto, display: &str) -> WriteResult<GroupDto> {
    let client = ApiClient::shared().clone();
    let req = GroupUpsertRequest {
        name: g.name.clone(),
        ratio: g.ratio,
        model_whitelist: g.model_whitelist.clone(),
        remark: display.trim().to_string(),
    };
    update_group_api(&client, &g.key, &req)
        .await
        .map_err(api_err_to_string)
}

/// 删除分组（调用方须先经 `ui::dialog::Dialog` 确认弹窗）。
pub async fn delete_group(key: &str) -> WriteResult<()> {
    let client = ApiClient::shared().clone();
    delete_group_api(&client, key)
        .await
        .map_err(api_err_to_string)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 渠道
// ---------------------------------------------------------------------------

/// 按渠道名找真实渠道（端点无 name 过滤，本地匹配），取回
/// 服务端 key 与现值，供编辑 / 删除定位。
///
/// `Ok(None)` = 列表拉取成功但无此名（本地草稿行 / 已被他人删除）；
/// `Err` = API 失败。语义约定见 [`find_group_by_name`]：只有 `Ok(None)`
/// 才允许按新建兜底，`Err` 一律报错不静默降级。
pub async fn find_channel_by_name(name: &str) -> WriteResult<Option<ChannelDto>> {
    let client = ApiClient::shared().clone();
    let channels = list_channels_api(&client)
        .await
        .map_err(api_err_to_string)?;
    Ok(channels.into_iter().find(|c| c.name == name))
}

/// 按渠道 key 读回单渠道（列表端点不携带 keys，编辑前单查取现值）。
pub async fn read_channel(key: &str) -> WriteResult<ChannelDto> {
    let client = ApiClient::shared().clone();
    get_channel_api(&client, key)
        .await
        .map_err(api_err_to_string)
}

/// 渠道编辑（最小 diff 体，语义见 [`UpdateChannelBody`]）。
///
/// - `new_name` / `new_url`：输入新值，恒发；
/// - `new_keys`：用户本次输入的明文 key 列表（未编辑传 `None`）；
///   `None` = 请求体整体缺席 `keys` 字段 = 后端 COALESCE 保持现有密钥。
///   旧值 / 掩码值绝不作为 `new_keys` 传入——这是最小 diff 的核心约定。
pub async fn update_channel(
    key: &str,
    new_name: &str,
    new_url: &str,
    new_keys: Option<Vec<String>>,
) -> WriteResult<ChannelDto> {
    let client = ApiClient::shared().clone();
    // 读现值透传弹窗不管理的列：channel_type / groups / remark / test_model。
    // test_model 恒带（该列无 COALESCE，缺席即清 NULL，带现值 = 不变）。
    let ch = get_channel_api(&client, key)
        .await
        .map_err(api_err_to_string)?;
    let body = UpdateChannelBody {
        name: new_name.to_string(),
        channel_type: ch.channel_type.clone(),
        base_url: new_url.to_string(),
        groups: ch.groups.clone(),
        remark: ch.remark.clone(),
        test_model: ch.test_model.clone(),
        keys: new_keys,
        // 拓扑 drawer 不管理 models 列：字段缺席 = 后端 COALESCE 保持现值
        // （models 通路属渠道管理页弹窗的「拉取模型」面板）。
        models: None,
    };
    update_channel_api(&client, key, &body)
        .await
        .map_err(api_err_to_string)
}

/// 删除渠道（调用方须先经 `ui::dialog::Dialog` 确认弹窗）。
pub async fn delete_channel(key: &str) -> WriteResult<()> {
    let client = ApiClient::shared().clone();
    delete_channel_api(&client, key)
        .await
        .map_err(api_err_to_string)?;
    Ok(())
}

/// 导入新渠道（全量 [`ChannelUpsertRequest`]，创建语义要求字段给全）。
///
/// - `groups`：绑定分组名（后端要求至少一个非空分组）；
/// - `keys`：明文 key 列表（至少一个非空，后端 validate 会拦）；
/// - `models` 发空数组——导入的渠道尚未加入任何调度模型，由用户后续
///   在渠道页「加入调度」，不在此造数据；
/// - `ctype`：渠道类型，缺省 `"openai"`（与 `CHANNEL_TYPES` 首项一致）。
pub async fn create_channel_import(
    name: &str,
    url: &str,
    ctype: &str,
    groups: &[String],
    keys: &[String],
) -> WriteResult<ChannelDto> {
    let ctype = if ctype.is_empty() {
        CHANNEL_TYPES[0]
    } else {
        ctype
    };
    let client = ApiClient::shared().clone();
    let req = ChannelUpsertRequest {
        name: name.to_string(),
        channel_type: ctype.to_string(),
        base_url: url.to_string(),
        keys: keys.to_vec(),
        models: serde_json::json!([]),
        groups: groups.to_vec(),
        priority: 0,
        weight: 0,
        test_model: None,
        remark: String::new(),
    };
    create_channel_api(&client, &req)
        .await
        .map_err(api_err_to_string)
}

/// 渠道「启用 / 停用」快捷写（`POST /api/channel/{key}/status`）。
pub async fn set_channel_status(key: &str, status: i16) -> WriteResult<()> {
    let client = ApiClient::shared().clone();
    set_channel_status_api(&client, key, status)
        .await
        .map_err(api_err_to_string)?;
    Ok(())
}

/// 抽屉写操作的通知条渲染：`idle` 隐藏；进行中灰字；成功绿字；
/// 失败为页面三态约定的 error 面板（柔和红边卡 + `role=alert`）。
/// 调用方持有 `Signal<Option<DrawerNotice>>`，写操作前后 set/清除。
#[derive(Clone, PartialEq)]
pub enum DrawerNotice {
    Idle,
    Busy,
    Ok,
    Err(String),
}

/// 通知条组件：与网络页 `ent-blocked` 顶部提示同一视觉约定。
#[component]
pub fn DrawerNoticeBar(notice: Signal<DrawerNotice>, on_clear: EventHandler<()>) -> Element {
    match notice() {
        DrawerNotice::Idle => rsx! { Fragment {} },
        DrawerNotice::Busy => rsx! {
            div {
                class: "px-4 py-2",
                "data-testid": "ent-write-notice",
                p { class: "text-[11px] text-zinc-400", "处理中…" }
            }
        },
        DrawerNotice::Ok => rsx! {
            div {
                class: "px-4 py-2",
                "data-testid": "ent-write-notice",
                p {
                    class: "text-[11px] text-emerald-300/80",
                    onclick: move |_| on_clear.call(()),
                    "已保存"
                }
            }
        },
        DrawerNotice::Err(msg) => {
            let m = msg.clone();
            rsx! {
                div {
                    role: "alert",
                    class: "mx-3 my-2 rounded-xl border-red-500/30 bg-red-950/30 px-3 py-2",
                    "data-testid": "ent-write-error",
                    p { class: "text-[11px] {ui::STATE_DANGER_TEXT}", "{m}" }
                    button {
                        class: "ml-2 text-[11px] text-red-300/70 hover:text-red-200",
                        onclick: move |_| on_clear.call(()),
                        "✕"
                    }
                }
            }
        }
    }
}
