use dioxus::prelude::*;

/// 状态/备忘录卡片 (任务系统、档案、备忘录)
#[component]
pub fn StatusCard(
    title: String,
    #[props(default = "emerald".to_string())] color: String,
    children: Element,
) -> Element {
    let border_color = match color.as_str() {
        "purple" => "border-purple-500/40 bg-purple-950/20 text-purple-200",
        "amber" => "border-amber-500/40 bg-amber-950/20 text-amber-200",
        "rose" => "border-rose-500/40 bg-rose-950/20 text-rose-200",
        "cyan" => "border-cyan-500/40 bg-cyan-950/20 text-cyan-200",
        _ => "border-emerald-500/40 bg-emerald-950/20 text-emerald-200",
    };
    let badge_color = match color.as_str() {
        "purple" => "bg-purple-500/20 text-purple-300 border-purple-500/30",
        "amber" => "bg-amber-500/20 text-amber-300 border-amber-500/30",
        "rose" => "bg-rose-500/20 text-rose-300 border-rose-500/30",
        "cyan" => "bg-cyan-500/20 text-cyan-300 border-cyan-500/30",
        _ => "bg-emerald-500/20 text-emerald-300 border-emerald-500/30",
    };
    rsx! {
        div { class: "flex flex-col gap-2 rounded-2xl border p-3.5 shadow-md {border_color}",
            div { class: "flex items-center gap-2",
                span { class: "rounded-lg border px-2 py-0.5 text-xs font-semibold {badge_color}",
                    "{title}"
                }
            }
            div { class: "text-xs leading-5 text-zinc-300",
                {children}
            }
        }
    }
}
