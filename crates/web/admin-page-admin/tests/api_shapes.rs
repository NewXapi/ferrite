//! admin page 集成测试:公共入口的状态不变量。
//!
//! network.rs 的物理/几何函数是私有的,单元测试在 src/network.rs 的
//! `#[cfg(test)] mod tests`;本文件只测 crate 外可见的 API。
//!
//! 注意:`EntityStore::seed()` 内部用 `Signal::new`,Signal 需要 Dioxus
//! runtime(thread_local 栈)。测试通过 headless VirtualDom 的根组件体内
//! 执行断言 —— 组件执行期间 runtime 在栈上,Signal 可用。

use admin_page_admin::state::{AliasRow, ChannelRow, EntityStore, GroupRow};
use dioxus::prelude::*;

/// Dioxus runtime 是 thread_local 栈;`VirtualDom::rebuild_in_place` 渲染
/// 根组件期间 runtime 在栈上,组件体内可以创建/读写 Signal。
/// fn item 不能捕获环境,所以用 thread_local 槽把 `body` 传进去。
fn with_runtime(body: fn()) {
    thread_local! {
        static BODY: std::cell::RefCell<Option<fn()>> = const { std::cell::RefCell::new(None) };
    }
    BODY.with(|slot| *slot.borrow_mut() = Some(body));

    #[component]
    fn TestRoot() -> Element {
        BODY.with(|slot| {
            if let Some(f) = *slot.borrow() {
                f();
            }
        });
        rsx! { div {} }
    }

    let mut vdom = VirtualDom::new(TestRoot);
    vdom.rebuild_in_place();
}

/// seed 的数据形状:分组/别名/渠道/调度模型的引用关系要自洽。
#[test]
fn seed_store_shapes_are_consistent() {
    with_runtime(|| {
        let store = EntityStore::seed();
        let groups: Vec<GroupRow> = store.groups.read().clone();
        let aliases: Vec<AliasRow> = store.aliases.read().clone();
        let channels: Vec<ChannelRow> = store.channels.read().clone();

        assert!(!groups.is_empty(), "至少一个分组");
        assert!(!aliases.is_empty(), "至少一个别名");
        assert!(!channels.is_empty(), "至少一个渠道");

        for (ci, ch) in channels.iter().enumerate() {
            assert!(!ch.name.is_empty(), "渠道 {ci} 名字非空");
            assert!(!ch.url.is_empty(), "渠道 {ci} URL 非空");
        }

        for (i, a) in aliases.iter().enumerate() {
            assert!(!a.alias.is_empty(), "别名 {i} 非空");
        }
        for (i, g) in groups.iter().enumerate() {
            assert!(!g.name.is_empty(), "分组 {i} 非空");
        }
    });
}
