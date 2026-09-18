//! 渠道管理共享类型与纯函数:弹窗状态、写操作种类、筛选与输入解析。
//! page / modal 复用;筛选与两个解析函数保持 `pub`,供 `tests/` 无 runtime 单测。

use contract::api::admin::ChannelDto;

/// 弹窗状态
#[derive(Clone, PartialEq)]
pub enum ChannelModalState {
    Closed,
    New,
    Edit(String),
}

/// 写操作种类:在写回工厂里区分启停与删除
#[derive(Clone, Copy)]
pub enum WriteOp {
    Toggle(i16),
    Delete,
}

/// 渠道列表筛选纯函数:按关键词(名称/类型/地址/分组,大小写不敏感)+
/// 状态档位(0=全部, 1=启用中, 2=已停用)过滤。
///
/// 抽成模块级 `pub` 纯函数,便于在同层 `tests/` 做无 runtime 的纯函数单测。
pub fn filter_channels(list: &[ChannelDto], query: &str, tier: usize) -> Vec<ChannelDto> {
    let q = query.trim().to_lowercase();
    list.iter()
        .filter(|c| {
            if !q.is_empty()
                && !c.name.to_lowercase().contains(&q)
                && !c.channel_type.to_lowercase().contains(&q)
                && !c.base_url.to_lowercase().contains(&q)
                && !c.groups.iter().any(|g| g.to_lowercase().contains(&q))
            {
                return false;
            }
            match tier {
                1 => c.status == 1,
                2 => c.status != 1,
                _ => true,
            }
        })
        .cloned()
        .collect()
}

/// 弹窗「绑定分组」输入解析:逗号分隔,trim,去空项(保持输入顺序)。
pub fn parse_group_input(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 弹窗「API Key」输入解析:换行分隔多 Key,trim,去空行(保持输入顺序)。
pub fn parse_keys_input(raw: &str) -> Vec<String> {
    raw.lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
