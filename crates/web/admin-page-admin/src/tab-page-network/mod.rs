//! 网络拓扑 tab:三层交互式路由编辑器。拆分约定:
//! - `data`:拓扑数据源(三端点并发拉取)、`GraphView` 快照与从 DTO 推导
//!   节点/边的纯函数、布局与几何计算(初始位置/贝塞尔/拟合视图/焦点锥)
//! - `physics`:每帧力学积分(`physics_step`)与节点标题/主色调派生
//! - `ui`:页面入口 `NetworkPanel`(画布 SVG + 拖拽/缩放/连线的全部交互)
//! - `drawer`:抽屉页签栏、抽屉头与渠道导入表单
//! - `inspector`:节点检视器(按节点类型分流)与底层输入控件
//! - `shared`:该 tab 的全部用户可见文案常量
//!
//! 边界:网络写路径(`crate::drawer_write`)不在此目录;本目录只做读侧拉取
//! `GET /api/group` `/api/channel` `/api/models` 与画布渲染。

#[path = "data.rs"]
pub mod data;
#[path = "drawer.rs"]
pub mod drawer;
#[path = "inspector.rs"]
pub mod inspector;
#[path = "physics.rs"]
pub mod physics;
#[path = "shared.rs"]
pub mod shared;
#[path = "ui.rs"]
pub mod ui;

pub use data::*;
pub use ui::NetworkPanel;
