# `page-users`

## 文件

- `src/lib.rs` — 导出 UsersPanel;一个 tab = 一个 `tab-page-*` 目录。
- `src/api.rs` — 用户列表 / 用户编辑 / 用户创建 / 分组列表 API 薄壳。
- `src/data.rs` — 展示格式化助手(额度换算、日期截断、key 截断、百分比)。
- `src/tab-page-users/` — 用户管理 tab,目录内文件不带前缀:
  - `page.rs` — 页面层:状态 + 拉取 effect + 统计 / 筛选 / 卡片网格三区组合(spec §1.4 统一页面层文件名)。
  - `user_card.rs` — 单张用户卡的页面适配层：分组名 → remark 展示标签的 context 映射 + 编辑 / 充值 / 启停操作插槽；卡片展示（AdminCard 三页签、额度换算、用量配色）复用 ui-components 的共享 `ui::UserCard`，本文件不保留整套渲染树。
  - `badge.rs` — 分组 / 角色 / 状态共用的胶囊徽标。
  - `modal.rs` — 弹窗外壳与输入框样式(表单弹窗与充值弹窗共用)。
  - `user_form.rs` — 新建 / 编辑弹窗,含弹窗内三个真实页签。
  - `group_chips.rs` — 生效分组多选 chips(列表经 context 注入)。
  - `role_chips.rs` — 角色权限单选 chips。
  - `topup_form.rs` — 额度充值弹窗。
  - `shared.rs` — 本 tab 独占的共享文案常量(spec §3.2 前缀)。
- `tests/api_shapes.rs` — 筛选标签约定 + 创建请求 wire 形状。
- `tests/format.rs` — 格式化助手不变量。

## 数据来源

- `GET /api/user/users?page=1&size=100` — 用户列表(items 信封)。size=100 是
  必须的:后端默认 20 会静默截断,统计卡「总用户」会少算。响应含 `groups`
  数组(迁移 0016 起),`groups[1]` 为生效分组。
  列表仅在挂载、手动刷新/重试或写操作成功后重新拉取;响应写回不触发新请求。
  刷新期间保留已有卡片,首次加载才显示占位符。
- `GET /api/group` — 分组列表(items 信封)。筛选胶囊与弹窗分组 chips 共用这一份,
  不再用 `mock::users::GROUPS` 常量(那里只有 default/vip/svip/internal,与后端
  实际分组不符)。chips 与卡片徽标的展示标签取 `remark`(remark 为空才回落
  `name`),与分组管理页主标题同口径。
- `POST /api/user/users/manage` — enable/disable/set_role/adjust_quota/set_groups/
  reset_password。编辑弹窗按 tab 回写单字段;启用/禁用按钮的**文案**是中文,
  提交的 action 是 snake_case 英文(后端枚举拒中文变体)。
- `POST /api/user/users` — admin 创建用户(用户名/初始密码/邮箱/角色/额度/分组数组)。

## 用户卡片

列表网格用 `ui::CardGrid`（`role="list"`，testid `users-list`，1/3/5 列），
分页是页面本地 UI 状态（`ui::Pager`，testid `users-pager`，每页 15 张），不
跨组件、不改拉取逻辑。单卡是共享 `ui::UserCard`（`AdminCard` 外壳，无头像字母
圈），三个页签：

- **基本信息**：用户名 / 邮箱 / 角色 / 状态 / 分组徽标（标签取分组列表 remark，
  回落裸名，与弹窗 chips 同口径）+ 底部操作按钮组
- **额度**：已用 / 总额（`500_000` 内部单位 = `¥1`）/ 进度条（用量 ≥70% 琥珀、
  ≥90% 红，否则绿）/ 请求数
- **系统**：截断 key / 创建时间

测试标识：卡根 `user-card`，操作按钮 `user-edit` / `user-topup` /
`user-toggle`。启停按钮提交的 action 是 snake_case 英文（`enable` / `disable`，
后端枚举拒中文变体），按钮文案仍是中文；编辑 / 充值分别开 `UserForm` 编辑态与
`TopUpForm`。卡片只经 props 收数据、经回调抛事件，不发起网络请求。

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
