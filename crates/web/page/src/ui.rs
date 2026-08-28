//! 通用滚动侦察导航（scroll-spy timeline）。
//!
//! 钉在滚动容器左缘的一条竖线 + 一串圆点：
//! - 圆点位置 = 对应分区在文档流里的真实纵向比例
//! - active = 视口内可见面积最大的分区（跟随滚动自动高亮）
//! - 点击圆点平滑滚动到对应分区；标签只在 hover 时浮现
//!
//! 用法：给滚动容器一个稳定 DOM id（`container`），分区元素用 `items`
//! 里的 id 标记，然后把组件放进一个 `relative` 的父级里即可。

use dioxus::prelude::*;

#[component]
pub fn ScrollSpyNav(
    /// 滚动容器的 DOM id
    container: &'static str,
    /// (标签, 分区元素的 DOM id)，顺序即导航顺序
    items: Vec<(String, String)>,
) -> Element {
    let mut active = use_signal(|| 0usize);
    let mut fracs = use_signal(Vec::<f64>::new);

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
                        // 每次挂载都先清旧挂载：否则第二次打开抽屉时守卫还在，
                        // 但旧 interval 引用的 eval 通道已断，导航永远死。
                        if (window.{guard}) {{
                            clearInterval(window.{guard}.timer);
                            window.removeEventListener('scroll', window.{guard}.fn, true);
                        }}
                        const IDS = [{ids_js}];
                        const report = () => {{
                            const cont = document.getElementById('{container}');
                            if (!cont) return;
                            const secs = IDS.map(id => document.getElementById(id)).filter(Boolean);
                            if (!secs.length) return;
                            const cr = cont.getBoundingClientRect();
                            let active = 0, best = -1;
                            secs.forEach((el, i) => {{
                                const r = el.getBoundingClientRect();
                                const ov = Math.min(r.bottom, cr.bottom) - Math.max(r.top, cr.top);
                                if (ov > best) {{ best = ov; active = i; }}
                            }});
                            const fr = secs.map(el =>
                                (el.getBoundingClientRect().top - cr.top + cont.scrollTop) / cont.scrollHeight
                            );
                            dioxus.send([active, ...fr]);
                        }};
                        const onScroll = (e) => {{
                            if (e.target === document.getElementById('{container}')) report();
                        }};
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
                    if v.len() == n + 1 {
                        active.set(v[0] as usize);
                        fracs.set(v[1..].to_vec());
                    }
                }
            });
        }
    });

    let n = items.len().max(1) as f64;
    rsx! {
        div { class: "pointer-events-none absolute left-2 top-3 z-20 h-[38%]",
            div { class: "relative h-full",
                div { class: "absolute bottom-0 left-[7px] top-0 w-px bg-zinc-800" }
                for (i, (label, target)) in items.iter().enumerate() {
                    {
                        let target_id = target.clone();
                        rsx! {
                    button {
                        class: "group absolute left-0 flex items-center gap-2 text-left",
                        style: "top: calc({fracs.read().get(i).copied().unwrap_or((i as f64 + 0.5) / n).clamp(0.02, 0.98) * 100.0}% - 7px)",
                        onclick: move |_| {
                            let _ = document::eval(&format!(r#"
                                document.getElementById("{target_id}")
                                    ?.scrollIntoView({{behavior:"smooth", block:"start"}});
                            "#));
                        },
                        {
                            let on = i == active();
                            rsx! {
                                span {
                                    class: if on {
                                        "pointer-events-auto relative z-10 flex h-3.5 w-3.5 items-center justify-center rounded-full border-2 border-zinc-200 bg-zinc-800"
                                    } else {
                                        "pointer-events-auto relative z-10 flex h-3.5 w-3.5 items-center justify-center rounded-full border border-zinc-700 bg-zinc-950 transition-colors group-hover:border-zinc-300 group-hover:bg-zinc-800"
                                    },
                                    span { class: if on { "h-1.5 w-1.5 rounded-full bg-zinc-100" } else { "h-1.5 w-1.5 rounded-full bg-zinc-500 group-hover:bg-zinc-200" } }
                                }
                                span {
                                    class: "pointer-events-none translate-x-1 whitespace-nowrap rounded-md border border-zinc-800 bg-zinc-950 px-2 py-1 text-[11px] text-zinc-200 opacity-0 shadow-lg transition-all group-hover:translate-x-0 group-hover:opacity-100",
                                    "{label}"
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
}
