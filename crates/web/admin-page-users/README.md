# `page-users`

## 文件

- `src/lib.rs` — 导出 UsersPanel。
- `src/api.rs` — 用户列表 / 用户编辑 / 用户创建 / 分组列表 API 薄壳。
- `src/data.rs` — 展示格式化助手(额度换算、日期截断、key 截断、百分比)。
- `src/panel.rs` — 用户管理卡片网格、筛选、编辑/新建弹窗、充值弹窗。
- `tests/api_shapes.rs` — 筛选标签约定 + 创建请求 wire 形状。
- `tests/format.rs` — 格式化助手不变量。

## 数据来源

- `GET /api/user/users?page=1&size=100` — 用户列表(items 信封)。size=100 是
  必须的:后端默认 20 会静默截断,统计卡「总用户」会少算。响应含 `groups`
  数组(迁移 0015 起),`groups[1]` 为生效分组。
- `GET /api/group` — 分组列表(items 信封)。筛选胶囊与弹窗分组 chips 共用这一份,
  不再用 `mock::users::GROUPS` 常量(那里只有 default/vip/svip/internal,与后端
  实际分组不符)。chips 与卡片徽标的展示标签取 `remark`(remark 为空才回落
  `name`),与分组管理页主标题同口径。
- `POST /api/user/users/manage` — enable/disable/set_role/adjust_quota/set_groups/
  reset_password。编辑弹窗按 tab 回写单字段;启用/禁用按钮的**文案**是中文,
  提交的 action 是 snake_case 英文(后端枚举拒中文变体)。
- `POST /api/user/users` — admin 创建用户(用户名/初始密码/邮箱/角色/额度/分组数组)。

## 编辑弹窗页签

三个真实页签,每组只渲染自己的字段(旧版「基本/额度/备注」共用同一字段区,
切换只是滚动):

- **基本信息**:用户名 / 邮箱 / 初始密码(仅新建) / 角色权限 chips(单选) / 额度
- **分组与备注**:生效分组 chips(**多选**,对齐 `auth_users.groups`,整体替换)/
  管理员备注
- **绑定**:第三方账号(后端暂无此列,只读)

角色与分组都做成 chips 面板(替代原生 `<select>`):角色单选(1/10/100),分组
多选;选中态与筛选胶囊同款高亮。分组多选的 `groups[1]` 是计费生效分组
(`currency_defs.group_rates` 取数键,token 未显式设组时的回落值)。

## 已知缺口

- 请求数:后端 `UserView` 不返回 `requestCount`(`AdminUserDto.request_count`
  缺省 0),卡片「请求数」恒为 0。要真实值需在 `/api/user/users` 响应里聚合
  `usage_logs` 计数(见 `crates/api/admin-observe/src/logs.rs` 的 stat/top)。
- 管理员备注字段前端可填,但后端 `auth_users` 无备注列,编辑态不回写
  (保存「分组与备注」页签只提交 `set_groups`)。
