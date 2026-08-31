//! 用户管理面板 mock 数据。字段名对标 new-api features/users。

/// 一个用户。
#[derive(Clone, PartialEq)]
pub struct User {
    pub id: u32,
    pub username: &'static str,
    pub display_name: &'static str,
    pub email: &'static str,
    pub quota: i64,
    pub used_quota: i64,
    pub request_count: u32,
    pub group: &'static str,
    pub aff_count: u32,
    /// 1 启用 / 2 禁用
    pub status: u8,
    /// 1 用户 / 10 管理员 / 100 root
    pub role: u16,
    pub created: &'static str,
}

/// 本月新增判定:mock 数据以 created 前缀比对当月
pub const THIS_MONTH: &str = "2026-08";

/// (标签, group 值);空值表示不过滤
pub const GROUPS: &[(&str, &str)] = &[
    ("全部", ""),
    ("默认", "default"),
    ("VIP", "vip"),
    ("SVIP", "svip"),
    ("内部", "internal"),
];

/// (标签, status 值);0 表示不过滤
pub const STATUSES: &[(&str, u8)] = &[("全部", 0), ("启用", 1), ("禁用", 2)];

/// (标签, role 值);0 表示不过滤
pub const ROLES: &[(&str, u16)] = &[("全部", 0), ("普通用户", 1), ("管理员", 10), ("Root", 100)];

pub const USERS: &[User] = &[
    User {
        id: 1,
        username: "hathaway",
        display_name: "海瑟薇",
        email: "hathaway@wildtoken.com",
        quota: 50_000_000,
        used_quota: 12_400_000,
        request_count: 18420,
        group: "svip",
        aff_count: 17,
        status: 1,
        role: 100,
        created: "2024-11-03",
    },
    User {
        id: 2,
        username: "linwei",
        display_name: "林伟",
        email: "linwei@example.com",
        quota: 20_000_000,
        used_quota: 18_920_000,
        request_count: 9312,
        group: "vip",
        aff_count: 4,
        status: 1,
        role: 10,
        created: "2025-01-18",
    },
    User {
        id: 3,
        username: "zhangna",
        display_name: "张娜",
        email: "zhangna@example.com",
        quota: 5_000_000,
        used_quota: 1_240_000,
        request_count: 1842,
        group: "default",
        aff_count: 0,
        status: 1,
        role: 1,
        created: "2025-03-22",
    },
    User {
        id: 4,
        username: "chenjing",
        display_name: "陈静",
        email: "chenjing@example.com",
        quota: 2_000_000,
        used_quota: 1_980_000,
        request_count: 764,
        group: "default",
        aff_count: 2,
        status: 2,
        role: 1,
        created: "2025-05-09",
    },
    User {
        id: 5,
        username: "wangfang",
        display_name: "王芳",
        email: "wangfang@wildtoken.com",
        quota: 30_000_000,
        used_quota: 4_120_000,
        request_count: 5231,
        group: "internal",
        aff_count: 9,
        status: 1,
        role: 10,
        created: "2025-06-14",
    },
    User {
        id: 6,
        username: "liuyang",
        display_name: "刘洋",
        email: "liuyang@example.com",
        quota: 1_000_000,
        used_quota: 0,
        request_count: 0,
        group: "default",
        aff_count: 0,
        status: 2,
        role: 1,
        created: "2025-08-02",
    },
    User {
        id: 7,
        username: "sunqi",
        display_name: "孙琪",
        email: "sunqi@example.com",
        quota: 10_000_000,
        used_quota: 3_450_000,
        request_count: 2610,
        group: "vip",
        aff_count: 6,
        status: 1,
        role: 1,
        created: "2026-08-11",
    },
    User {
        id: 8,
        username: "zhaolei",
        display_name: "赵磊",
        email: "zhaolei@example.com",
        quota: 8_000_000,
        used_quota: 620_000,
        request_count: 431,
        group: "default",
        aff_count: 1,
        status: 1,
        role: 1,
        created: "2026-08-24",
    },
];
