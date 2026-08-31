//! 固定式锚点导航(scroll-spy,所有面板共用)
//!
//! - `fixed` 钉在面板内容区左上,手机/平板/Web 全部常驻,不随内容滚走
//!   【注意】壳层还有一个 SectionPill(顶层导航,left-2 top-1/2),本组件必须错开它的位置
//! - 整页不需要滚动时自动隐藏(此时锚点导航没有意义)
//! - 圆点等距紧凑排列,点与点之间短线相连,两端不留线头
//! - active = 视口内可见面积最大的锚点分区,滚动时自动高亮
//! - 点击圆点平滑滚动到对应分区;悬停导航上滚轮 = 滚动内容容器
//!
//! 用法:滚动容器给稳定 DOM id(`container`),分区元素带 `items` 里的 id。

use dioxus::prelude::*;

#[component]
pub fn ScrollSpyNav(
    /// 滚动容器的 DOM id
    container: &'static str,
    /// (标签, 分区元素的 DOM id),顺序即导航顺序
    items: Vec<(String, String)>,
) -> Element {
    let mut active = use_signal(|| 0usize);
    // 页面不需要滚动时隐藏导航
    let mut can_scroll = use_signal(|| true);

    use_hook({
        let items = items.clone();
        move || {
            let ids_js = items
                .iter()
                .map(|(_, id)| format!("'{id}'"))
                .collect::<Vec<_>>()
                .join(",");
            let guard = format!("__spy_{}", container.replace('-', "_"));
            let n = items.len();
            spawn(async move {
                let mut ev = document::eval(&format!(
                    r#"
                    {{
                        // 每次挂载先清旧挂载,防止重挂载后守卫还在但通道已死
                        if (window.{guard}) {{
                            clearInterval(window.{guard}.timer);
                            window.removeEventListener('scroll', window.{guard}.fn, true);
                        }}
                        const IDS = [{ids_js}];
                        const report = () => {{
                            const secs = IDS.map(id => document.getElementById(id)).filter(Boolean);
                            if (!secs.length) return;
                            const vh = window.innerHeight;
                            // 顶部线语义:最后一个顶边越过视口 35% 线的分区为 active;整页放得下时回落第一个
                            // 未滚动时恒选第一个分区;否则顶线规则
                            const atTop = (document.getElementById('{container}')?.scrollTop ?? 0) < 4 && window.scrollY < 4;
                            let active = 0;
                            if (!atTop) {{
                                secs.forEach((el, i) => {{
                                    if (el.getBoundingClientRect().top <= vh * 0.35) active = i;
                                }});
                            }}
                            const canScroll = document.documentElement.scrollHeight > window.innerHeight + 4
                                || ((document.getElementById('{container}')?.scrollHeight ?? 0) - (document.getElementById('{container}')?.clientHeight ?? 0) > 4);
                            dioxus.send([active, canScroll ? 1 : 0]);
                        }};
                        const onScroll = () => report();
                        window.addEventListener('scroll', onScroll, true);
                        const timer = setInterval(() => {{
                            if (document.getElementById('{container}')) report();
                        }}, 500);
                        window.{guard} = {{ timer, fn: onScroll }};
                        report();
                    }}
                    "#
                ));
                while let Ok(v) = ev.recv::<Vec<f64>>().await {
                    if let Some(a) = v.first() {
                        if (*a as usize) < n {
                            active.set(*a as usize);
                        }
                    }
                    if let Some(&c) = v.get(1) {
                        can_scroll.set(c > 0.5);
                    }
                }
            });
        }
    });

    rsx! {
        nav {
            class: "fixed left-9 top-[15%] z-30 flex-col items-center sm:left-11",
            class: if can_scroll() { "flex" } else { "hidden" },
            onwheel: move |e: WheelEvent| {
                use dioxus::html::geometry::WheelDelta;
                e.prevent_default();
                let dy = match e.delta() {
                    WheelDelta::Pixels(v) => v.y,
                    WheelDelta::Lines(v) => v.y * 24.0,
                    WheelDelta::Pages(v) => v.y * 400.0,
                };
                // 桌面端内容在 #container 内滚动,移动端整页滚动(window),两边都喂
                let _ = document::eval(&format!(
                    "document.getElementById('{container}')?.scrollBy({{top: {dy}}}); window.scrollBy(0, {dy});"
                ));
            },
            for (i, (label, target)) in items.iter().enumerate() {
                if i > 0 {
                    // 点间短线,两端不留线头
                    div { class: "my-1 h-2 w-px bg-zinc-700" }
                }
                {
                    let target_id = target.clone();
                    let on = i == active();
                    rsx! {
                        button {
                            key: "{label}",
                            class: "group relative flex items-center",
                            "aria-label": "{label}",
                            onclick: move |_| {
                                let _ = document::eval(&format!(
                                    "document.getElementById('{target_id}')?.scrollIntoView({{behavior:'smooth', block:'start'}});"
                                ));
                            },
                            span {
                                class: if on {
                                    "block h-6 w-1.5 rounded-full bg-zinc-100 transition-all"
                                } else {
                                    "block h-2.5 w-2.5 rounded-full bg-zinc-500 transition-all group-hover:bg-zinc-300"
                                }
                            }
                            span { class: "pointer-events-none absolute left-4 whitespace-nowrap rounded-md border border-zinc-800 bg-zinc-950 px-2 py-1 text-[11px] text-zinc-200 opacity-0 shadow-lg transition-opacity group-hover:opacity-100",
                                "{label}"
                            }
                        }
                    }
                }
            }
        }
    }
}
