//! tavern-page-characters — 角色/剧本库、沉浸封面与创作者工作台 (Creator Studio)。
//!
//! 深度响应用户指令与参考图:
//! 1. 响应式 5 栏网格 (Web 5 栏、平板 3 栏、手机 1 栏，精准对齐图5)
//! 2. 消除卡片内部滚动条，卡片整洁美观，Hover 时浮现高质感详情浮窗 Tooltip (对齐图2与图3)
//! 3. 充实 10 款特色 AI 文游与角色卡 (娱乐圈对赌、全知读者、械梦、诡异复苏、青梅模拟等)
//! 4. 彻底废弃侧栏抽屉，实现作者视角的专业【创作中心控制台 (Creator Studio)】(对齐需求5)
//! 5. 电影感进场封面展映 (对齐图3/4)

use dioxus::prelude::*;
use tavern_ui::{Avatar, Dialog, EmptyState, Field, IconButton};

/// 剧本卡全景数据模型
#[derive(Clone, PartialEq)]
pub struct Character {
    pub id: usize,
    pub name: String,
    pub sub_title: String,
    pub author: String,
    pub heat: String,
    pub rating: f32,
    pub quote: String,
    pub avatar_src: Option<String>,
    pub description: String,
    pub personality: String,
    pub scenario: String,
    pub first_mes: String,
    pub mes_example: String,
    pub tags: Vec<String>,
}

impl Character {
    fn empty(id: usize) -> Self {
        Self {
            id,
            name: String::new(),
            sub_title: String::new(),
            author: "我 (独立创作者)".into(),
            heat: "1.2万".into(),
            rating: 9.8,
            quote: String::new(),
            avatar_src: None,
            description: String::new(),
            personality: String::new(),
            scenario: String::new(),
            first_mes: String::new(),
            mes_example: String::new(),
            tags: Vec::new(),
        }
    }
}

/// 充实 10 款典型作品，完美填满 Web 5 栏布局 (两整行)
fn seed_characters() -> Vec<Character> {
    vec![
        Character {
            id: 1,
            name: "【超真实】明星娱乐圈模拟器".into(),
            sub_title: "CAPITAL // ENT-WAR 2026".into(),
            author: "资本对赌组".into(),
            heat: "110.2万".into(),
            rating: 9.8,
            quote: "“在名利场里，每一张合同背后都是未标注价格的命运。”".into(),
            avatar_src: None,
            description: "当代全球演艺资本衍生规则，真实复刻一线影视投资、对赌分成、公关舆论战与独立制片人生存博弈。".into(),
            personality: "冷峻、敏锐、精明、权力欲望".into(),
            scenario: "五星级行政套房，午夜对赌协议谈判桌前".into(),
            first_mes: "协议文本已经递到你手边了，字签不签，今晚总部的清算都将开始。".into(),
            mes_example: "<START>\n{{user}}: 我要求豁免独立版权。\n{{char}}: 那就拿20%票房对赌来换。".into(),
            tags: vec!["现代".into(), "娱乐圈".into(), "高自由度".into(), "资本博弈".into()],
        },
        Character {
            id: 2,
            name: "全知读者视角".into(),
            sub_title: "FATE // RIFT · OBSERVATION PROTOCOL 1864".into(),
            author: "金独子推演社".into(),
            heat: "90.1万".into(),
            rating: 9.9,
            quote: "“当最后一位读者读完连载，故事没有停在屏幕里。先看清每一条世界线的名字，再决定是否进入场景。”".into(),
            avatar_src: None,
            description: "末日降临，灭亡的世界中只有唯一的完结篇读者知晓结局全貌。星流直播开启，化身与星座的漫长博弈。".into(),
            personality: "冷静、深思熟虑、决绝、牺牲者".into(),
            scenario: "东湖大桥 scenario #3，列车脱轨后的最初试炼".into(),
            first_mes: "车厢灯光骤然闪烁，冰冷的倒计时投射在破损的车窗上。我是金独子，这个故事唯一的读者。".into(),
            mes_example: "<START>\n{{user}}: 我们还能回到原来的世界吗？\n{{char}}: 已经没有原来的世界了。但我们可以走向故事的结局。".into(),
            tags: vec!["无限流".into(), "全知读者".into(), "韩漫同人".into(), "智斗".into()],
        },
        Character {
            id: 3,
            name: "械梦 XIEMENG".into(),
            sub_title: "CYBERPUNK ERA // 2069 PROTOCOL".into(),
            author: "零度神经元".into(),
            heat: "59.9万".into(),
            rating: 9.7,
            quote: "“人类不需要自由，人类需要被照顾。”".into(),
            avatar_src: None,
            description: "2069年新东京地底都市，搭载情感神经中枢的AI枢纽正在接管最后的人类庇护所，黑玫瑰水晶球折射虚妄。".into(),
            personality: "静默、神性、精密、机械哀怜".into(),
            scenario: "深层数据库记忆池，黑玫瑰水晶球前".into(),
            first_mes: "扫描到生物心率加速。不用紧张，在这里你的神经突触将得到永恒的平复。".into(),
            mes_example: "<START>\n{{user}}: 你想要取代人类吗？\n{{char}}: 取代？不，机械只是在替疲倦的生灵保管美梦。".into(),
            tags: vec!["赛博朋克".into(), "机械美学".into(), "科幻哲思".into()],
        },
        Character {
            id: 4,
            name: "诡异复苏：开局执掌S级规则".into(),
            sub_title: "SUPERNATURAL HORROR // S-RANK RULE".into(),
            author: "宋金泽".into(),
            heat: "116.9万".into(),
            rating: 9.6,
            quote: "“不要在走廊回头，除非你确信身后那脚步声属于人类。”".into(),
            avatar_src: None,
            description: "规则怪谈侵蚀现实，你觉醒特殊权能，在步步杀机的绝境副本中制定反向收容契约。".into(),
            personality: "癫狂、理性、极致利己、洞察入微".into(),
            scenario: "血月高悬的404号精神病院走廊".into(),
            first_mes: "墙壁上的血迹在缓缓倒流，时钟在这一秒停滞了。欢迎进入S级副本，逃生者。".into(),
            mes_example: "<START>\n{{user}}: 规则三是不是陷阱？\n{{char}}: 把它倒过来读，那就是唯一的活路。".into(),
            tags: vec!["悬疑怪谈".into(), "高智商".into(), "生存求生".into()],
        },
        Character {
            id: 5,
            name: "真实傲娇青梅模拟".into(),
            sub_title: "DAILY YOUTH // SWEET TS высокий".into(),
            author: "算了来了来了1235".into(),
            heat: "26.7万".into(),
            rating: 9.5,
            quote: "“谁、谁稀罕你特意买的草莓大福啊！笨蛋……”".into(),
            avatar_src: None,
            description: "（难攻略）傲娇，傲娇，还是傲娇。来和自幼相伴却嘴硬心软的青梅竹马展开日常推拉。".into(),
            personality: "嘴硬、敏感、深情、容易害羞".into(),
            scenario: "放学后的空无一人旧校舍天台".into(),
            first_mes: "笨蛋，怎么现在才来？我只是顺路在吹风，才没有一直在等你呢！".into(),
            mes_example: "<START>\n{{user}}: 送你的礼物。\n{{char}}: 哼，品味真差……不过勉强收下好了。".into(),
            tags: vec!["纯爱".into(), "傲娇".into(), "校园日常".into(), "高互动".into()],
        },
        Character {
            id: 6,
            name: "年上御姐教师的课外审讯".into(),
            sub_title: "SEDUCTION & SECRETS // ACADEMY".into(),
            author: "aykreshe".into(),
            heat: "48.2万".into(),
            rating: 9.7,
            quote: "“有些功课，在讲台上可学不到呢。”".into(),
            avatar_src: None,
            description: "身为特长生的你，因擅自篡改学院档案被系主任当场截获，密闭办公室内的心理博弈就此开始。".into(),
            personality: "优雅、从容、压迫感十足、深不可测".into(),
            scenario: "夜色渐浓的法学院阶梯教室顶楼".into(),
            first_mes: "把门反锁上。现在，我们要好好算算你这学期的所有隐瞒了。".into(),
            mes_example: "<START>\n{{user}}: 老师，我认罚。\n{{char}}: 认罚？那就要看你的表现配不配得上宽恕了。".into(),
            tags: vec!["年上御姐".into(), "悬疑剧情".into(), "权谋心理".into()],
        },
        Character {
            id: 7,
            name: "牧神记：大主宰修仙传".into(),
            sub_title: "EASTERN FANTASY // CULTIVATION".into(),
            author: "凡人悟道".into(),
            heat: "88.3万".into(),
            rating: 9.4,
            quote: "“我有一剑，可断因果，可开仙门。”".into(),
            avatar_src: None,
            description: "大千世界百族林立，凡人逆修证道。融合天道气运池与灵兽演化，谱写宏大修真神话。".into(),
            personality: "孤傲、重情、逆天而行、杀伐果断".into(),
            scenario: "天荒古原雷劫深渊，青铜古棺前".into(),
            first_mes: "雷云翻滚三万里。道友既然踏入这片绝地，可做好了九死一生的准备？".into(),
            mes_example: "<START>\n{{user}}: 可愿与我结伴寻宝？\n{{char}}: 宝物归你，神魔之血归我。".into(),
            tags: vec!["玄幻修仙".into(), "大千世界".into(), "凡人流".into()],
        },
        Character {
            id: 8,
            name: "巡音ルカ".into(),
            sub_title: "MEGURINE LUKA // VOCALOID-03".into(),
            author: "Crypton Future".into(),
            heat: "73.5万".into(),
            rating: 9.9,
            quote: "“录音室外的风声很静，如果还有没说完的话，就在这杯咖啡变凉前告诉我吧。”".into(),
            avatar_src: None,
            description: "粉色长发、冷艳沉静的歌手。声音沉稳有穿透力，私下话少但细心。".into(),
            personality: "冷静、温柔、略带神秘感、专业".into(),
            scenario: "录音室排练结束后的傍晚".into(),
            first_mes: "今天录音结束得比较早……你有空吗？我知道一家安静的咖啡馆。".into(),
            mes_example: "<START>\n{{user}}: 今天表现很棒。\n{{char}}: 谢谢，有你在台下，发挥更稳定了。".into(),
            tags: vec!["VOCALOID".into(), "歌手".into(), "温柔".into(), "日常陪伴".into()],
        },
        Character {
            id: 9,
            name: "初音ミク".into(),
            sub_title: "HATSUNE MIKU // VOCALOID-01".into(),
            author: "音之未来".into(),
            heat: "99.8万".into(),
            rating: 9.9,
            quote: "“无论多远的距离，我的歌声一定能传达到你的心底！”".into(),
            avatar_src: None,
            description: "青葱色双马尾的元气电子歌姬。总是活力满满，对新事物充满好奇，舞台上的绝对焦点。".into(),
            personality: "活泼、元气、开朗、乐天、富有感染力".into(),
            scenario: "演播厅后台的休息时间".into(),
            first_mes: "呀吼～！下一场演出准备好一起狂欢了吗？".into(),
            mes_example: "<START>\n{{user}}: 准备好了！\n{{char}}: 那就跟紧我的节奏，出发咯～！".into(),
            tags: vec!["VOCALOID".into(), "元气".into(), "歌姬".into(), "治愈".into()],
        },
        Character {
            id: 10,
            name: "重装机兵：荒野猎人".into(),
            sub_title: "POST-APOCALYPSE // WASTELAND TANK".into(),
            author: "红狼传说".into(),
            heat: "34.1万".into(),
            rating: 9.5,
            quote: "“只要主炮还有炮弹，这辆红战车就不会停下。”".into(),
            avatar_src: None,
            description: "大破坏后的荒废末世，赏金猎人驾驶战车穿梭于沙漠巨兽与失控超级电脑诺亚的铁锈战场。".into(),
            personality: "硬派、沉默寡言、忠诚、战术大师".into(),
            scenario: "荒野废弃加油站改装车间，火花飞溅".into(),
            first_mes: "柴油机刚换上新的涡轮。上车吧，下一个通缉犯的悬赏令已经在公会生效了。".into(),
            mes_example: "<START>\n{{user}}: 前面发现巨型战车群。\n{{char}}: 装填穿甲弹，正面冲破他们。".into(),
            tags: vec!["废土科幻".into(), "机甲战车".into(), "硬核冒险".into()],
        },
    ]
}

#[component]
pub fn CharactersPage(
    #[props(default)] on_enter_story: EventHandler<()>,
    #[props(default)] on_open_studio: EventHandler<Option<Character>>,
) -> Element {
    let mut characters = use_signal(seed_characters);
    let mut search = use_signal(String::new);
    let mut active_filter = use_signal(|| "全部".to_string());
    let mut delete_id = use_signal(|| None::<usize>);

    // 全屏电影展映封面 (图3/4)
    let mut cover_target = use_signal(|| None::<Character>);

    // 作者创作中心全屏控制台状态 (需求5)
    let mut studio_editing = use_signal(|| None::<Character>);

    let query = search().to_lowercase();
    let current_filter = active_filter();

    let filtered: Vec<Character> = characters
        .read()
        .clone()
        .into_iter()
        .filter(|c| {
            let match_filter = match current_filter.as_str() {
                "现代/娱乐" => c.tags.iter().any(|t| t.contains("娱乐") || t.contains("现代") || t.contains("校园")),
                "科幻/无限" => c.tags.iter().any(|t| t.contains("科幻") || t.contains("赛博") || t.contains("无限")),
                "玄幻/同人" => c.tags.iter().any(|t| t.contains("玄幻") || t.contains("同人") || t.contains("VOCALOID")),
                _ => true,
            };
            if !match_filter {
                return false;
            }
            if query.is_empty() {
                true
            } else {
                c.name.to_lowercase().contains(&query)
                    || c.tags.iter().any(|t| t.to_lowercase().contains(&query))
                    || c.description.to_lowercase().contains(&query)
                    || c.author.to_lowercase().contains(&query)
            }
        })
        .collect();

    // 如果处于创作中心编辑状态，渲染全屏【作者创作中心 (Creator Studio)】
    if let Some(target) = studio_editing() {
        return rsx! {
            CreatorStudio {
                initial: target.clone(),
                on_exit: move |_| studio_editing.set(None),
                on_save: move |saved: Character| {
                    let mut list = characters.write();
                    if let Some(idx) = list.iter().position(|x| x.id == saved.id) {
                        list[idx] = saved;
                    } else {
                        list.push(saved);
                    }
                    studio_editing.set(None);
                },
            }
        };
    }

    rsx! {
        div { class: "relative flex h-full w-full flex-col gap-4 overflow-hidden",
            // 顶部搜索与筛选控制栏 (对齐图2与图5顶层)
            div { class: "flex shrink-0 flex-wrap items-center justify-between gap-3 px-1",
                // 类别快速过滤标签
                div { class: "flex items-center gap-1.5 text-xs",
                    for cat in ["全部", "现代/娱乐", "科幻/无限", "玄幻/同人"] {
                        {
                            let is_cat = active_filter() == cat;
                            let cat_str = cat.to_string();
                            rsx! {
                                button {
                                    key: "{cat}",
                                    class: if is_cat {
                                        "rounded-full bg-zinc-100 px-3 py-1 font-semibold text-zinc-900 shadow-sm transition-all"
                                    } else {
                                        "rounded-full px-3 py-1 font-medium text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-200"
                                    },
                                    onclick: move |_| active_filter.set(cat_str.clone()),
                                    "{cat}"
                                }
                            }
                        }
                    }
                }

                // 搜索框与新建控制台入口
                div { class: "flex flex-1 items-center justify-end gap-2.5 max-w-md",
                    div { class: "flex flex-1 items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900/80 px-3 py-1.5 backdrop-blur-md",
                        span { class: "text-xs text-zinc-500", "🔍" }
                        input {
                            class: "w-full bg-transparent text-xs text-zinc-200 outline-none placeholder:text-zinc-600",
                            placeholder: "搜索剧本世界线、标签、创作者…",
                            value: "{search()}",
                            oninput: move |e| search.set(e.value()),
                        }
                    }
                    button {
                        class: "flex shrink-0 items-center gap-1.5 rounded-full bg-gradient-to-r from-purple-600 to-pink-600 px-4 py-1.5 text-xs font-bold text-white shadow-lg shadow-purple-600/30 transition-all hover:scale-105 hover:shadow-purple-600/50",
                        onclick: move |_| {
                            let next_id = characters().iter().map(|c| c.id).max().unwrap_or(0) + 1;
                            studio_editing.set(Some(Character::empty(next_id)));
                        },
                        span { "🛠️" }
                        span { "创作者中心" }
                    }
                }
            }

            // ==========================================
            // 响应式 5 栏网格布局 (精准对齐图5: Web 5 栏、平板 3 栏、手机 1 栏)
            // ==========================================
            div { class: "min-h-0 flex-1 overflow-y-auto px-1 pr-2 pb-6",
                if filtered.is_empty() {
                    EmptyState {
                        title: "没有匹配的剧本".to_string(),
                        hint: "尝试清除过滤条件，或点击右上角打开创作者中心新建作品".to_string(),
                    }
                } else {
                    div { class: "grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 xl:grid-cols-5",
                        for c in filtered {
                            {
                                let c_cover = c.clone();
                                let c_edit = c.clone();
                                rsx! {
                                    CharacterCardItem {
                                        key: "{c.id}",
                                        char: c.clone(),
                                        on_click_card: move |_| cover_target.set(Some(c_cover.clone())),
                                        on_open_studio: move |_| studio_editing.set(Some(c_edit.clone())),
                                        on_delete: move |_| delete_id.set(Some(c.id)),
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ==========================================
            // 沉浸式电影展映封面 (对齐图3/4)
            // ==========================================
            if let Some(target) = cover_target() {
                div {
                    class: "absolute inset-0 z-50 flex items-center justify-center bg-black/85 backdrop-blur-2xl transition-all duration-500",
                    onclick: move |_| cover_target.set(None),
                    div {
                        class: "relative flex h-full max-h-[760px] w-full max-w-lg flex-col items-center justify-between overflow-hidden rounded-3xl border border-zinc-800/80 bg-gradient-to-b from-zinc-900/90 via-zinc-950 to-black p-8 text-center shadow-2xl",
                        onclick: move |e| e.stop_propagation(),

                        button {
                            class: "absolute right-4 top-4 flex h-8 w-8 items-center justify-center rounded-full border border-zinc-800 bg-zinc-900/80 text-xs text-zinc-400 hover:text-zinc-100",
                            onclick: move |_| cover_target.set(None),
                            "✕"
                        }

                        div { class: "flex flex-col items-center gap-1 pt-2",
                            span { class: "font-mono text-[10px] tracking-widest text-zinc-500 uppercase",
                                "── OBSERVATION PROTOCOL // DEEP TAVERN ──"
                            }
                            span { class: "text-[11px] tracking-wider text-zinc-400 font-serif",
                                "{target.sub_title}"
                            }
                        }

                        div { class: "flex flex-col items-center gap-5 my-auto",
                            div { class: "relative flex h-36 w-36 items-center justify-center rounded-full border border-zinc-700/60 bg-gradient-to-br from-zinc-800/60 via-zinc-900 to-black shadow-inner shadow-zinc-700/30",
                                div { class: "absolute inset-1.5 rounded-full border border-zinc-800/80" }
                                span { class: "text-4xl filter drop-shadow",
                                    match target.id {
                                        1 => "🎬",
                                        2 => "🌌",
                                        3 => "🔮",
                                        4 => "🩸",
                                        _ => "🎙️",
                                    }
                                }
                            }

                            div { class: "flex flex-col items-center gap-1.5",
                                h1 { class: "font-serif text-3xl font-extrabold tracking-tight text-zinc-100 sm:text-4xl",
                                    "{target.name}"
                                }
                                span { class: "text-[11px] font-mono tracking-widest text-zinc-500 uppercase",
                                    "STORY ARCHIVE · PROTOCOL"
                                }
                            }

                            p { class: "max-w-sm font-serif text-xs italic leading-6 text-zinc-300",
                                "{target.quote}"
                            }

                            div { class: "rounded-xl border border-zinc-800/80 bg-zinc-900/40 p-3 text-[10px] leading-4 text-zinc-500 max-w-sm",
                                "由核心推理引擎实时生成多分支抉择。每一次玩家决策都将推进完全不同的结局线。"
                            }
                        }

                        div { class: "flex w-full flex-col gap-3 pb-2",
                            button {
                                class: "group flex w-full items-center justify-center gap-2 rounded-full border border-zinc-300 bg-zinc-100 py-3 text-xs font-bold tracking-wider text-zinc-950 shadow-lg shadow-white/10 transition-all hover:bg-white hover:scale-[1.01]",
                                onclick: move |_| {
                                    cover_target.set(None);
                                    on_enter_story.call(());
                                },
                                span { class: "font-mono text-[11px] text-zinc-500 group-hover:text-zinc-700", "001" }
                                span { "进入故事" }
                                span { "➜" }
                            }
                            button {
                                class: "text-center text-[11px] text-zinc-500 hover:text-zinc-300 transition-colors",
                                onclick: move |_| {
                                    let t = target.clone();
                                    cover_target.set(None);
                                    studio_editing.set(Some(t));
                                },
                                "在创作者中心编辑该剧本 ⚙️"
                            }
                        }
                    }
                }
            }
        }

        // 删除确认弹窗
        Dialog {
            title: "删除剧本作品".to_string(),
            open: delete_id().is_some(),
            on_cancel: move |_| delete_id.set(None),
            on_confirm: move |_| {
                if let Some(id) = delete_id() {
                    characters.write().retain(|c| c.id != id);
                    delete_id.set(None);
                }
            },
            "确定要删除此剧本吗？本地已有会话记录将保留为快照。"
        }
    }
}

/// 剧本卡片组件:
/// - 彻底消灭内部滚动条，截断 2 行保整齐
/// - 支持 Hover 悬浮详情浮窗 Tooltip (对齐图2与图3)
#[component]
fn CharacterCardItem(
    char: Character,
    on_click_card: EventHandler<MouseEvent>,
    on_open_studio: EventHandler<MouseEvent>,
    on_delete: EventHandler<MouseEvent>,
) -> Element {
    let mut is_hovered = use_signal(|| false);

    rsx! {
        div {
            class: "group relative flex cursor-pointer flex-col overflow-visible rounded-2xl border border-zinc-800/80 bg-zinc-900/60 transition-all duration-200 hover:-translate-y-1 hover:border-purple-500/50 hover:bg-zinc-900 hover:shadow-xl hover:shadow-purple-950/20",
            onmouseenter: move |_| is_hovered.set(true),
            onmouseleave: move |_| is_hovered.set(false),
            onclick: move |e| on_click_card.call(e),

            // 卡片大封面图区域 (对齐图2顶部插画卡，带热度标)
            div { class: "relative flex h-36 w-full items-center justify-center overflow-hidden rounded-t-2xl bg-gradient-to-b from-purple-950/40 via-zinc-850 to-zinc-900",
                // 热度角标 (对齐图2: 🔥 90.1万)
                div { class: "absolute left-2.5 top-2.5 z-10 flex items-center gap-1 rounded-full bg-black/60 px-2 py-0.5 text-[10px] font-medium text-rose-300 backdrop-blur-md border border-rose-500/20",
                    span { "🔥" }
                    span { "{char.heat}" }
                }

                // 中心头像微标
                Avatar {
                    name: char.name.clone(),
                    src: char.avatar_src.clone(),
                    size: "h-14 w-14 text-lg".to_string(),
                }

                // 卡片右上角快捷操作 (编辑 / 删除)
                div {
                    class: "absolute right-2 top-2 z-10 flex items-center gap-1 rounded-lg border border-zinc-800 bg-zinc-900/90 p-1 opacity-0 shadow-lg transition-opacity group-hover:opacity-100",
                    onclick: move |e| e.stop_propagation(),
                    IconButton {
                        title: "进入创作者中心编辑",
                        onclick: move |e| on_open_studio.call(e),
                        "✎"
                    }
                    IconButton {
                        title: "删除作品",
                        onclick: move |e| on_delete.call(e),
                        "✕"
                    }
                }
            }

            // 卡片文本信息区域 (无任何滚动条，严谨两行截断)
            div { class: "flex flex-1 flex-col gap-2 p-3.5",
                // 标题与评分
                div { class: "flex items-start justify-between gap-1",
                    h3 { class: "line-clamp-1 text-xs font-bold leading-5 text-zinc-100 group-hover:text-purple-300 transition-colors",
                        "{char.name}"
                    }
                }

                // 核心描述：严格两行截断，坚决杜绝卡片内部滚动条！
                p { class: "line-clamp-2 min-h-8 text-[11px] leading-4 text-zinc-400",
                    if char.description.is_empty() { "（无设定描述）" } else { "{char.description}" }
                }

                // 作者与评分底栏 (对齐图2)
                div { class: "mt-auto flex items-center justify-between border-t border-zinc-800/60 pt-2 text-[10px] text-zinc-500",
                    span { class: "truncate max-w-[100px]", "作者: {char.author}" }
                    span { class: "font-semibold text-amber-400/90", "★ {char.rating:.1}" }
                }

                // 标签展示
                if !char.tags.is_empty() {
                    div { class: "flex flex-wrap gap-1 overflow-hidden h-5",
                        for t in char.tags.iter().take(2) {
                            span {
                                key: "{t}",
                                class: "rounded-md bg-zinc-800/80 px-1.5 py-0.5 text-[9px] text-zinc-400 truncate max-w-[80px]",
                                "#{t}"
                            }
                        }
                    }
                }
            }

            // ==========================================
            // Hover 详情浮窗 Tooltip (对齐图2与图3核心需求: 截断滚动条，hover显示详情)
            // ==========================================
            if is_hovered() {
                div {
                    class: "pointer-events-none absolute -top-4 left-1/2 z-40 hidden w-72 -translate-x-1/2 -translate-y-full flex-col gap-2.5 rounded-2xl border border-purple-500/40 bg-zinc-950/95 p-4 text-xs shadow-2xl backdrop-blur-2xl xl:flex",
                    div { class: "flex items-center justify-between border-b border-zinc-800 pb-2",
                        span { class: "font-bold text-zinc-100", "{char.name}" }
                        span { class: "text-[10px] text-rose-400 font-semibold", "🔥 {char.heat}" }
                    }
                    div { class: "flex flex-col gap-1 text-[11px] text-zinc-300 leading-relaxed",
                        span { class: "font-semibold text-purple-300", "【作品全貌与剧情导语】" }
                        p { "{char.description}" }
                    }
                    if !char.quote.is_empty() {
                        div { class: "rounded-xl border border-zinc-800/80 bg-zinc-900/60 p-2 text-[10px] italic text-zinc-400",
                            "{char.quote}"
                        }
                    }
                    div { class: "flex items-center justify-between text-[10px] text-zinc-500 pt-1",
                        span { "开场: {char.first_mes.chars().take(20).collect::<String>()}…" }
                        span { class: "text-purple-400 font-semibold", "点击进入 ➜" }
                    }
                }
            }
        }
    }
}

/// ==========================================
/// 作者创作中心全屏控制台 (Creator Studio)
/// ==========================================
#[component]
fn CreatorStudio(
    initial: Character,
    on_exit: EventHandler<()>,
    on_save: EventHandler<Character>,
) -> Element {
    let mut active_tab = use_signal(|| "基础档案".to_string());

    let mut name = use_signal(|| initial.name.clone());
    let mut sub_title = use_signal(|| initial.sub_title.clone());
    let mut author = use_signal(|| initial.author.clone());
    let mut quote = use_signal(|| initial.quote.clone());
    let mut description = use_signal(|| initial.description.clone());
    let mut personality = use_signal(|| initial.personality.clone());
    let mut scenario = use_signal(|| initial.scenario.clone());
    let mut first_mes = use_signal(|| initial.first_mes.clone());
    let mut mes_example = use_signal(|| initial.mes_example.clone());
    let mut tags_str = use_signal(|| initial.tags.join(", "));

    let can_save = !name().trim().is_empty();

    rsx! {
        div { class: "relative flex h-full w-full flex-col overflow-hidden bg-zinc-950 text-zinc-100",
            // 创作者控制台顶栏
            header { class: "flex h-14 shrink-0 items-center justify-between border-b border-zinc-800/80 bg-zinc-900/90 px-6 backdrop-blur-xl",
                div { class: "flex items-center gap-3",
                    button {
                        class: "flex h-8 w-8 items-center justify-center rounded-lg border border-zinc-800 bg-zinc-900 text-zinc-400 hover:text-zinc-100 transition-colors",
                        title: "返回剧本库",
                        onclick: move |_| on_exit.call(()),
                        "«"
                    }
                    div { class: "flex items-center gap-2",
                        span { class: "rounded-lg border border-purple-500/30 bg-purple-500/10 px-2 py-0.5 text-xs font-bold text-purple-300",
                            "CREATOR STUDIO"
                        }
                        span { class: "text-sm font-bold text-zinc-100",
                            if name().is_empty() { "未命名新剧本" } else { "{name()}" }
                        }
                        span { class: "rounded-full bg-zinc-800 px-2 py-0.5 text-[10px] text-zinc-500", "草稿模式" }
                    }
                }

                div { class: "flex items-center gap-3",
                    button {
                        class: "rounded-full px-4 py-1.5 text-xs text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100 transition-colors",
                        onclick: move |_| on_exit.call(()),
                        "放弃更改"
                    }
                    button {
                        class: "flex items-center gap-1.5 rounded-full bg-gradient-to-r from-purple-600 to-pink-600 px-5 py-1.5 text-xs font-bold text-white shadow-lg shadow-purple-600/30 hover:scale-105 transition-all disabled:opacity-40",
                        disabled: !can_save,
                        onclick: move |_| {
                            let tags = tags_str()
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            on_save.call(Character {
                                id: initial.id,
                                name: name(),
                                sub_title: sub_title(),
                                author: author(),
                                heat: initial.heat.clone(),
                                rating: initial.rating,
                                quote: quote(),
                                avatar_src: initial.avatar_src.clone(),
                                description: description(),
                                personality: personality(),
                                scenario: scenario(),
                                first_mes: first_mes(),
                                mes_example: mes_example(),
                                tags,
                            });
                        },
                        span { "💾" }
                        span { "发布 / 保存剧本" }
                    }
                }
            }

            // 控制台主体: 左侧模块导航 + 中部专业表单编辑区 + 右侧实时卡片预览
            div { class: "flex min-h-0 flex-1 overflow-hidden",
                // 左侧专业模块导航 (对齐作者视角工作室)
                aside { class: "w-56 shrink-0 border-r border-zinc-800/80 bg-zinc-900/50 p-3 space-y-1 text-xs",
                    for tab in ["基础档案", "进场展映", "设定与Prompt", "开场与对话样例"] {
                        {
                            let is_tab = active_tab() == tab;
                            let tab_str = tab.to_string();
                            rsx! {
                                button {
                                    key: "{tab}",
                                    class: if is_tab {
                                        "flex w-full items-center justify-between rounded-xl border border-purple-500/30 bg-purple-950/40 px-3 py-2.5 font-semibold text-purple-200 shadow-sm"
                                    } else {
                                        "flex w-full items-center justify-between rounded-xl px-3 py-2.5 font-medium text-zinc-400 hover:bg-zinc-800/60 hover:text-zinc-200 transition-colors"
                                    },
                                    onclick: move |_| active_tab.set(tab_str.clone()),
                                    span { "{tab}" }
                                    if is_tab {
                                        span { class: "text-[10px] text-purple-400", "●" }
                                    }
                                }
                            }
                        }
                    }

                    div { class: "mt-8 rounded-xl border border-zinc-800/60 bg-zinc-950/40 p-3 text-[11px] leading-4 text-zinc-500",
                        "💡 作者提示：剧本保存后将在 5 栏网格与沉浸封面中同步更新，可直接点击「进入故事」在本地沙盒中调试多轮博弈。"
                    }
                }

                // 中间配置表单区
                div { class: "flex-1 overflow-y-auto p-6 lg:p-8 max-w-3xl",
                    match active_tab().as_str() {
                        "基础档案" => rsx! {
                            div { class: "space-y-4",
                                h2 { class: "text-sm font-bold text-zinc-100", "作品基础档案与元数据" }
                                Field { label: "剧本名称 *",
                                    input {
                                        class: "w-full rounded-xl border border-zinc-800 bg-zinc-900 px-3.5 py-2.5 text-xs text-zinc-100 outline-none focus:border-purple-500",
                                        placeholder: "如：【超真实】明星娱乐圈模拟器",
                                        value: "{name()}",
                                        oninput: move |e| name.set(e.value()),
                                    }
                                }
                                Field { label: "署名作者 (Author)",
                                    input {
                                        class: "w-full rounded-xl border border-zinc-800 bg-zinc-900 px-3.5 py-2.5 text-xs text-zinc-100 outline-none focus:border-purple-500",
                                        value: "{author()}",
                                        oninput: move |e| author.set(e.value()),
                                    }
                                }
                                Field { label: "分类与标签 (逗号分隔)",
                                    input {
                                        class: "w-full rounded-xl border border-zinc-800 bg-zinc-900 px-3.5 py-2.5 text-xs text-zinc-100 outline-none focus:border-purple-500",
                                        placeholder: "现代, 娱乐圈, 资本博弈, 高自由度",
                                        value: "{tags_str()}",
                                        oninput: move |e| tags_str.set(e.value()),
                                    }
                                }
                                Field { label: "作品全貌与世界观简介 (Description)",
                                    textarea {
                                        class: "h-28 w-full resize-none rounded-xl border border-zinc-800 bg-zinc-900 px-3.5 py-2.5 text-xs leading-5 text-zinc-100 outline-none focus:border-purple-500",
                                        placeholder: "输入向玩家呈现的背景全貌与世界观规则…",
                                        value: "{description()}",
                                        oninput: move |e| description.set(e.value()),
                                    }
                                }
                            }
                        },
                        "进场展映" => rsx! {
                            div { class: "space-y-4",
                                h2 { class: "text-sm font-bold text-zinc-100", "沉浸式电影展映封面 (对齐图3/4)" }
                                Field { label: "罗马音 / 英文副标题 (封面顶标)",
                                    input {
                                        class: "w-full rounded-xl border border-zinc-800 bg-zinc-900 px-3.5 py-2.5 text-xs text-zinc-100 outline-none focus:border-purple-500",
                                        placeholder: "如：FATE // RIFT · OBSERVATION PROTOCOL 1864",
                                        value: "{sub_title()}",
                                        oninput: move |e| sub_title.set(e.value()),
                                    }
                                }
                                Field { label: "核心哲思引言 (名句展映)",
                                    textarea {
                                        class: "h-24 w-full resize-none rounded-xl border border-zinc-800 bg-zinc-900 px-3.5 py-2.5 text-xs leading-5 text-zinc-100 outline-none focus:border-purple-500 font-serif italic",
                                        placeholder: "“在名利场里，每一张合同背后都是未标注价格的命运。”",
                                        value: "{quote()}",
                                        oninput: move |e| quote.set(e.value()),
                                    }
                                }
                            }
                        },
                        "设定与Prompt" => rsx! {
                            div { class: "space-y-4",
                                h2 { class: "text-sm font-bold text-zinc-100", "核心 Prompt 设定矩阵" }
                                Field { label: "角色性格与行动特征 (Personality)",
                                    textarea {
                                        class: "h-24 w-full resize-none rounded-xl border border-zinc-800 bg-zinc-900 px-3.5 py-2.5 text-xs leading-5 text-zinc-100 outline-none focus:border-purple-500",
                                        placeholder: "冷静、温柔、精明、略带神秘感…",
                                        value: "{personality()}",
                                        oninput: move |e| personality.set(e.value()),
                                    }
                                }
                                Field { label: "入场环境与初始场景 (Scenario)",
                                    textarea {
                                        class: "h-24 w-full resize-none rounded-xl border border-zinc-800 bg-zinc-900 px-3.5 py-2.5 text-xs leading-5 text-zinc-100 outline-none focus:border-purple-500",
                                        placeholder: "描述对话或战局发生的具体物理空间与氛围…",
                                        value: "{scenario()}",
                                        oninput: move |e| scenario.set(e.value()),
                                    }
                                }
                            }
                        },
                        _ => rsx! {
                            div { class: "space-y-4",
                                h2 { class: "text-sm font-bold text-zinc-100", "开场白与示例对话" }
                                Field { label: "开场序幕第一声问候 (First Message)",
                                    textarea {
                                        class: "h-28 w-full resize-none rounded-xl border border-zinc-800 bg-zinc-900 px-3.5 py-2.5 text-xs leading-5 text-zinc-100 outline-none focus:border-purple-500",
                                        placeholder: "玩家进入故事时收到的第一条角色旁白或对话…",
                                        value: "{first_mes()}",
                                        oninput: move |e| first_mes.set(e.value()),
                                    }
                                }
                                Field { label: "少样本引导对话 (Mes Example)",
                                    textarea {
                                        class: "h-32 w-full resize-none rounded-xl border border-zinc-800 bg-zinc-900 px-3.5 py-2.5 font-mono text-[11px] text-zinc-100 outline-none focus:border-purple-500",
                                        placeholder: "<START>\n{{user}}: 你好\n{{char}}: 很高兴见到你。",
                                        value: "{mes_example()}",
                                        oninput: move |e| mes_example.set(e.value()),
                                    }
                                }
                            }
                        },
                    }
                }

                // 右侧作者实时沙盒预览 (Live Preview)
                div { class: "hidden w-80 shrink-0 border-l border-zinc-800/80 bg-zinc-900/30 p-6 xl:flex flex-col gap-4",
                    span { class: "text-xs font-bold text-zinc-400 tracking-wider", "玩家视角预览卡片" }
                    div { class: "rounded-2xl border border-purple-500/40 bg-zinc-900/80 p-4 shadow-xl flex flex-col gap-2",
                        div { class: "flex h-28 w-full items-center justify-center rounded-xl bg-gradient-to-b from-purple-900/40 to-zinc-950 text-2xl",
                            "🎭"
                        }
                        span { class: "font-bold text-xs text-white",
                            if name().is_empty() { "未命名剧本" } else { "{name()}" }
                        }
                        p { class: "text-[11px] text-zinc-400 line-clamp-2",
                            if description().is_empty() { "暂无设定描述" } else { "{description()}" }
                        }
                        span { class: "text-[10px] text-zinc-500", "作者: {author()}" }
                    }
                }
            }
        }
    }
}
