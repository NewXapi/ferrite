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
    /// 邀请人用户名;None = 无邀请人
    pub inviter: Option<&'static str>,
    /// 邀请分成收入(quota)
    pub aff_income: i64,
    /// 创建时间(完整,hover 展示)
    pub created_at: &'static str,
    /// 最后登录(完整,hover 展示)
    pub last_login_at: &'static str,
    /// 管理员备注(仅管理员可见)
    pub remark: &'static str,
    /// 第三方账号绑定(只读,用户在个人资料里管理)
    pub bindings: Bindings,
}

/// 第三方绑定;None 显示为 "-"
#[derive(Clone, Copy, PartialEq)]
pub struct Bindings {
    pub github: Option<&'static str>,
    pub discord: Option<&'static str>,
    pub oidc: Option<&'static str>,
    pub wechat: Option<&'static str>,
    pub telegram: Option<&'static str>,
}

/// 空绑定(新用户)
pub const BINDINGS_NONE: Bindings = Bindings {
    github: None,
    discord: None,
    oidc: None,
    wechat: None,
    telegram: None,
};

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
        inviter: None,
        aff_income: 85_000,
        created_at: "2024-11-03 09:12:44",
        last_login_at: "2026-09-01 08:41:02",
        remark: "内部主力测试号",
        bindings: Bindings {
            github: Some("hathaway"),
            ..BINDINGS_NONE
        },
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
        inviter: Some("hathaway"),
        aff_income: 0,
        created_at: "2025-01-18 22:05:11",
        last_login_at: "2026-08-31 21:17:40",
        remark: "",
        bindings: BINDINGS_NONE,
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
        inviter: Some("hathaway"),
        aff_income: 0,
        created_at: "2025-03-22 14:33:09",
        last_login_at: "2026-08-15 11:02:37",
        remark: "",
        bindings: BINDINGS_NONE,
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
        inviter: None,
        aff_income: 0,
        created_at: "2025-05-09 16:44:52",
        last_login_at: "2026-07-30 09:55:12",
        remark: "连续 30 天无登录,待清退",
        bindings: Bindings {
            wechat: Some("chenjing_wx"),
            ..BINDINGS_NONE
        },
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
        inviter: None,
        aff_income: 0,
        created_at: "2025-06-14 08:20:31",
        last_login_at: "2026-08-30 19:44:25",
        remark: "",
        bindings: Bindings {
            telegram: Some("@wangfang"),
            ..BINDINGS_NONE
        },
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
        inviter: Some("sunqi"),
        aff_income: 0,
        created_at: "2025-08-02 13:27:58",
        last_login_at: "2026-05-12 17:06:33",
        remark: "",
        bindings: BINDINGS_NONE,
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
        inviter: None,
        aff_income: 30_000,
        created_at: "2026-08-11 10:31:47",
        last_login_at: "2026-09-01 07:52:19",
        remark: "",
        bindings: Bindings {
            github: Some("sunqi-dev"),
            telegram: Some("@sunqi"),
            ..BINDINGS_NONE
        },
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
        inviter: Some("sunqi"),
        aff_income: 0,
        created_at: "2026-08-24 23:58:04",
        last_login_at: "2026-08-31 23:12:51",
        remark: "",
        bindings: BINDINGS_NONE,
    },
];
