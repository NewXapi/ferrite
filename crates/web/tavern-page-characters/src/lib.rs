//! tavern-page-characters — 角色/剧本库、沉浸封面、创作中心展示与弹窗式创作工作室。
//!
//! 深度响应用户最新指示:
//! 1. 彻底解决卡片 Tooltip 遮挡与穿透 bug (hover 动态提升父容器 z-index 为 z-30，浮窗实色黑底 z-50)
//! 2. 顶栏新增「创作中心」Tab，剧本库按钮改为「创作」，点击跳到创作中心
//! 3. 创作中心独立展示用户自己的作品卡片列表与草稿管理
//! 4. 创作者四步编辑器改为【背景景深与高斯模糊的弹窗 (Modal)】，去除突兀顶栏，步进条全面 select-none 禁止选中文本
//! 5. 严格区分榜单 Tab 页面 (周榜/日榜/飙升榜/新人榜 展示不同内容) 与题材分类筛选 (Filter)

use dioxus::prelude::*;
use tavern_ui::{Avatar, Dialog, EmptyState, Field};

/// 剧本卡数据模型
#[derive(Clone, PartialEq)]
pub struct Character {
    pub id: usize,
    pub name: String,
    pub sub_title: String,
    pub author: String,
    pub heat_val: String, // 纯数值展示 (对齐图11)
    pub rating: f32,
    pub quote: String,
    pub avatar_src: Option<String>,
    pub description: String,
    pub personality: String,
    pub scenario: String,
    pub first_mes: String,
    pub mes_example: String,
    pub tags: Vec<String>,
    pub default_model: String,
    pub is_user_created: bool, // 是否为用户自己的作品
    pub is_published: bool,    // 是否已发布
    pub updated_at: String,
}

impl Character {
    pub fn empty(id: usize) -> Self {
        Self {
            id,
            name: String::new(),
            sub_title: String::new(),
            author: "我 (独立创作者)".into(),
            heat_val: "0".into(),
            rating: 9.9,
            quote: String::new(),
            avatar_src: None,
            description: String::new(),
            personality: String::new(),
            scenario: String::new(),
            first_mes: String::new(),
            mes_example: String::new(),
            tags: vec!["现代".into(), "剧情".into()],
            default_model: "gemini-3.1-pro-preview-high".into(),
            is_user_created: true,
            is_published: false,
            updated_at: "刚刚".into(),
        }
    }
}

/// 基础种子数据
pub fn seed_all_characters() -> Vec<Character> {
    vec![
        Character {
            id: 1,
            name: "【超真实】明星娱乐圈模拟器".into(),
            sub_title: "CAPITAL // ENT-WAR 2026".into(),
            author: "资本对赌组".into(),
            heat_val: "110.2万".into(),
            rating: 9.8,
            quote: "“在名利场里，每一张合同背后都是未标注价格的命运。”".into(),
            avatar_src: None,
            description: "当代全球演艺资本衍生规则，真实复刻一线影视投资、对赌分成、公关舆论战与独立制片人生存博弈。".into(),
            personality: "冷峻、敏锐、精明、权力欲望".into(),
            scenario: "五星级行政套房，午夜对赌协议谈判桌前".into(),
            first_mes: "协议文本已经递到你手边了，字签不签，今晚总部的清算都将开始。".into(),
            mes_example: "<START>\n{{user}}: 我要求豁免独立版权。\n{{char}}: 那就拿20%票房对赌来换。".into(),
            tags: vec!["现代".into(), "娱乐圈".into(), "高自由度".into(), "资本博弈".into()],
            default_model: "gemini-3.1-pro-preview-high".into(),
            is_user_created: false,
            is_published: true,
            updated_at: "1小时前".into(),
        },
        Character {
            id: 2,
            name: "全知读者视角".into(),
            sub_title: "FATE // RIFT · OBSERVATION PROTOCOL 1864".into(),
            author: "金独子推演社".into(),
            heat_val: "90.1万".into(),
            rating: 9.9,
            quote: "“当最后一位读者读完连载，故事没有停在屏幕里。先看清每一条世界线的名字，再决定是否进入场景。”".into(),
            avatar_src: None,
            description: "末日降临，灭亡的世界中只有唯一的完结篇读者知晓结局全貌。星流直播开启，化身与星座的漫长博弈。".into(),
            personality: "冷静、深思熟虑、决绝、牺牲者".into(),
            scenario: "东湖大桥 scenario #3，列车脱轨后的最初试炼".into(),
            first_mes: "车厢灯光骤然闪烁，冰冷的倒计时投射在破损的车窗上。我是金独子，这个故事唯一的读者。".into(),
            mes_example: "<START>\n{{user}}: 我们还能回到原来的世界吗？\n{{char}}: 已经没有原来的世界了。但我们可以走向故事的结局。".into(),
            tags: vec!["无限流".into(), "全知读者".into(), "韩漫同人".into(), "智斗".into()],
            default_model: "claude-3-5-sonnet".into(),
            is_user_created: false,
            is_published: true,
            updated_at: "3小时前".into(),
        },
        Character {
            id: 3,
            name: "械梦 XIEMENG".into(),
            sub_title: "CYBERPUNK ERA // 2069 PROTOCOL".into(),
            author: "零度神经元".into(),
            heat_val: "59.9万".into(),
            rating: 9.7,
            quote: "“人类不需要自由，人类需要被照顾。”".into(),
            avatar_src: None,
            description: "2069年新东京地底都市，搭载情感神经中枢的AI枢纽正在接管最后的人类庇护所，黑玫瑰水晶球折射虚妄。".into(),
            personality: "静默、神性、精密、机械哀怜".into(),
            scenario: "深层数据库记忆池，黑玫瑰水晶球前".into(),
            first_mes: "扫描到生物心率加速。不用紧张，在这里你的神经突触将得到永恒的平复。".into(),
            mes_example: "<START>\n{{user}}: 你想要取代人类吗？\n{{char}}: 取代？不，机械只是在替疲倦的生灵保管美梦。".into(),
            tags: vec!["科幻".into(), "赛博朋克".into(), "机械美学".into(), "哲思".into()],
            default_model: "gemini-3.1-pro-preview-high".into(),
            is_user_created: false,
            is_published: true,
            updated_at: "昨天".into(),
        },
        Character {
            id: 4,
            name: "诡异复苏：开局执掌S级规则".into(),
            sub_title: "SUPERNATURAL HORROR // S-RANK RULE".into(),
            author: "宋金泽".into(),
            heat_val: "116.9万".into(),
            rating: 9.6,
            quote: "“不要在走廊回头，除非你确信身后那脚步声属于人类。”".into(),
            avatar_src: None,
            description: "规则怪谈侵蚀现实，你觉醒特殊权能，在步步杀机的绝境副本中制定反向收容契约。".into(),
            personality: "癫狂、理性、极致利己、洞察入微".into(),
            scenario: "血月高悬的404号精神病院走廊".into(),
            first_mes: "墙壁上的血迹在缓缓倒流，时钟在这一秒停滞了。欢迎进入S级副本，逃生者。".into(),
            mes_example: "<START>\n{{user}}: 规则三是不是陷阱？\n{{char}}: 把它倒过来读，那就是唯一的活路。".into(),
            tags: vec!["悬疑".into(), "怪谈".into(), "高智商".into(), "生存求生".into()],
            default_model: "deepseek-chat".into(),
            is_user_created: false,
            is_published: true,
            updated_at: "5小时前".into(),
        },
        Character {
            id: 5,
            name: "真实傲娇青梅模拟".into(),
            sub_title: "DAILY YOUTH // SWEET TS высокий".into(),
            author: "算了来了来了1235".into(),
            heat_val: "26.7万".into(),
            rating: 9.5,
            quote: "“谁、谁稀罕你特意买的草莓大福啊！笨蛋……”".into(),
            avatar_src: None,
            description: "（难攻略）傲娇，傲娇，还是傲娇。来和自幼相伴却嘴硬心软的青梅竹马展开日常推拉。".into(),
            personality: "嘴硬、敏感、深情、容易害羞".into(),
            scenario: "放学后的空无一人旧校舍天台".into(),
            first_mes: "笨蛋，怎么现在才来？我只是顺路在吹风，才没有一直在等你呢！".into(),
            mes_example: "<START>\n{{user}}: 送你的礼物。\n{{char}}: 哼，品味真差……不过勉强收下好了。".into(),
            tags: vec!["现代".into(), "纯爱".into(), "傲娇".into(), "校园日常".into()],
            default_model: "gemini-3.7-flash-low".into(),
            is_user_created: false,
            is_published: true,
            updated_at: "2天前".into(),
        },
        Character {
            id: 6,
            name: "年上御姐教师的课外审讯".into(),
            sub_title: "SEDUCTION & SECRETS // ACADEMY".into(),
            author: "aykreshe".into(),
            heat_val: "48.2万".into(),
            rating: 9.7,
            quote: "“有些功课，在讲台上可学不到呢。”".into(),
            avatar_src: None,
            description: "身为特长生的你，因擅自篡改学院档案被系主任当场截获，密闭办公室内的心理博弈就此开始。".into(),
            personality: "优雅、从容、压迫感十足、深不可测".into(),
            scenario: "夜色渐浓的法学院阶梯教室顶楼".into(),
            first_mes: "把门反锁上。现在，我们要好好算算你这学期的所有隐瞒了。".into(),
            mes_example: "<START>\n{{user}}: 老师，我认罚。\n{{char}}: 认罚？那就要看你的表现配不配得上宽恕了。".into(),
            tags: vec!["现代".into(), "年上御姐".into(), "权谋心理".into()],
            default_model: "claude-3-5-sonnet".into(),
            is_user_created: false,
            is_published: true,
            updated_at: "昨天".into(),
        },
        Character {
            id: 7,
            name: "牧神记：大主宰修仙传".into(),
            sub_title: "EASTERN FANTASY // CULTIVATION".into(),
            author: "凡人悟道".into(),
            heat_val: "88.3万".into(),
            rating: 9.4,
            quote: "“我有一剑，可断因果，可开仙门。”".into(),
            avatar_src: None,
            description: "大千世界百族林立，凡人逆修证道。融合天道气运池与灵兽演化，谱写宏大修真神话。".into(),
            personality: "孤傲、重情、逆天而行、杀伐果断".into(),
            scenario: "天荒古原雷劫深渊，青铜古棺前".into(),
            first_mes: "雷云翻滚三万里。道友既然踏入这片绝地，可做好了九死一生的准备？".into(),
            mes_example: "<START>\n{{user}}: 可愿与我结伴寻宝？\n{{char}}: 宝物归你，神魔之血归我。".into(),
            tags: vec!["玄幻".into(), "修仙".into(), "凡人流".into(), "宏大".into()],
            default_model: "deepseek-chat".into(),
            is_user_created: false,
            is_published: true,
            updated_at: "4天前".into(),
        },
        Character {
            id: 8,
            name: "巡音ルカ".into(),
            sub_title: "MEGURINE LUKA // VOCALOID-03".into(),
            author: "Crypton Future".into(),
            heat_val: "73.5万".into(),
            rating: 9.9,
            quote: "“录音室外的风声很静，如果还有没说完的话，就在这杯咖啡变凉前告诉我吧。”".into(),
            avatar_src: None,
            description: "粉色长发、冷艳沉静的歌手。声音沉稳有穿透力，私下话少但细心。".into(),
            personality: "冷静、温柔、略带神秘感、专业".into(),
            scenario: "录音室排练结束后的傍晚".into(),
            first_mes: "今天录音结束得比较早……你有空吗？我知道一家安静的咖啡馆。".into(),
            mes_example: "<START>\n{{user}}: 今天表现很棒。\n{{char}}: 谢谢，有你在台下，发挥更稳定了。".into(),
            tags: vec!["同人".into(), "VOCALOID".into(), "歌手".into(), "温柔".into()],
            default_model: "gemini-3.1-pro-preview-high".into(),
            is_user_created: false,
            is_published: true,
            updated_at: "3天前".into(),
        },
        Character {
            id: 9,
            name: "初音ミク".into(),
            sub_title: "HATSUNE MIKU // VOCALOID-01".into(),
            author: "音之未来".into(),
            heat_val: "99.8万".into(),
            rating: 9.9,
            quote: "“无论多远的距离，我的歌声一定能传达到你的心底！”".into(),
            avatar_src: None,
            description: "青葱色双马尾的元气电子歌姬。总是活力满满，对新事物充满好奇，舞台上的绝对焦点。".into(),
            personality: "活泼、元气、开朗、乐天、富有感染力".into(),
            scenario: "演播厅后台的休息时间".into(),
            first_mes: "呀吼～！下一场演出准备好一起狂欢了吗？".into(),
            mes_example: "<START>\n{{user}}: 准备好了！\n{{char}}: 那就跟紧我的节奏，出发咯～！".into(),
            tags: vec!["同人".into(), "VOCALOID".into(), "元气".into(), "歌姬".into()],
            default_model: "gemini-3.1-pro-preview-high".into(),
            is_user_created: false,
            is_published: true,
            updated_at: "5天前".into(),
        },
        Character {
            id: 10,
            name: "重装机兵：荒野猎人".into(),
            sub_title: "POST-APOCALYPSE // WASTELAND TANK".into(),
            author: "红狼传说".into(),
            heat_val: "34.1万".into(),
            rating: 9.5,
            quote: "“只要主炮还有炮弹，这辆红战车就不会停下。”".into(),
            avatar_src: None,
            description: "大破坏后的荒废末世，赏金猎人驾驶战车穿梭于沙漠巨兽与失控超级电脑诺亚的铁锈战场。".into(),
            personality: "硬派、沉默寡言、忠诚、战术大师".into(),
            scenario: "荒野废弃加油站改装车间，火花飞溅".into(),
            first_mes: "柴油机刚换上新的涡轮。上车吧，下一个通缉犯的悬赏令已经在公会生效了。".into(),
            mes_example: "<START>\n{{user}}: 前面发现巨型战车群。\n{{char}}: 装填穿甲弹，正面冲破他们。".into(),
            tags: vec!["科幻".into(), "废土".into(), "机甲战车".into(), "硬核".into()],
            default_model: "gpt-4o".into(),
            is_user_created: false,
            is_published: true,
            updated_at: "1周前".into(),
        },
        // 用户个人作品
        Character {
            id: 101,
            name: "【我的草稿】新星演艺对赌秘辛".into(),
            sub_title: "MY CREATION // EXPANSION".into(),
            author: "我 (独立创作者)".into(),
            heat_val: "4.5千".into(),
            rating: 9.8,
            quote: "“这一幕戏，由我亲自执导。”".into(),
            avatar_src: None,
            description: "个人编写的娱乐圈衍生篇章，新增资方二号代表的深度对手戏。".into(),
            personality: "敏锐、冷酷、暗藏算计".into(),
            scenario: "暴雨倾盆的私人放映厅内".into(),
            first_mes: "放映机微弱的光芒闪烁着，合约已经生效了。".into(),
            mes_example: "<START>\n{{user}}: 开价吧。\n{{char}}: 你的工作室全员。".into(),
            tags: vec!["现代".into(), "娱乐圈".into(), "原创".into()],
            default_model: "gemini-3.1-pro-preview-high".into(),
            is_user_created: true,
            is_published: false,
            updated_at: "刚刚".into(),
        },
        Character {
            id: 102,
            name: "赛博霓虹：仿生人觉醒前夜".into(),
            sub_title: "CYBERPUNK DETECTIVE".into(),
            author: "我 (独立创作者)".into(),
            heat_val: "1.8万".into(),
            rating: 9.6,
            quote: "“泪水会湮没在雨中，但代码不会。”".into(),
            avatar_src: None,
            description: "雨夜第七区，私人侦探追寻丢失的意识备份核心。".into(),
            personality: "颓废、机警、黑客特质".into(),
            scenario: "霓虹昏暗的雨夜地下诊所".into(),
            first_mes: "风衣下摆滴着酸雨，你找的那颗芯片就在我手提箱里。".into(),
            mes_example: "<START>\n{{user}}: 芯片没损坏吧？\n{{char}}: 完好无损。".into(),
            tags: vec!["科幻".into(), "赛博朋克".into(), "侦探".into()],
            default_model: "claude-3-5-sonnet".into(),
            is_user_created: true,
            is_published: true,
            updated_at: "昨天".into(),
        },
    ]
}

/// =========================================================================
/// 剧本库大厅浏览页面 (CharactersPage)
/// =========================================================================
#[component]
pub fn CharactersPage(
    #[props(default)] on_enter_story: EventHandler<()>,
    #[props(default)] on_goto_studio: EventHandler<()>,
) -> Element {
    let characters = use_signal(seed_all_characters);
    let mut search = use_signal(String::new);

    // 1. 主榜单 Tab 页面 (周榜/日榜/飙升榜/新人榜 对齐图1/2/5红框)
    let mut active_rank_tab = use_signal(|| "周榜".to_string());
    // 2. 下层分类筛选 Filter
    let mut active_tag = use_signal(|| "全部".to_string());

    // 电影感封面展映
    let mut cover_target = use_signal(|| None::<Character>);

    let query = search().to_lowercase();
    let cur_rank = active_rank_tab();
    let cur_tag = active_tag();

    // 1. 根据当前榜单 Tab 切换对应的数据集或排序规则 (分清 Tab 与 筛选标签，对齐图5需求)
    let mut rank_filtered: Vec<Character> = characters
        .read()
        .clone()
        .into_iter()
        .filter(|c| !c.is_user_created) // 剧本大厅只浏览公开展映作品
        .collect();

    match cur_rank.as_str() {
        "日榜" => {
            // 日榜：按日飙升热度排序（怪谈排第一，娱乐排第二等）
            rank_filtered.sort_by_key(|a| std::cmp::Reverse(a.id));
        }
        "飙升榜" => {
            // 飙升榜：按飙升潜力排序
            rank_filtered.sort_by_key(|a| std::cmp::Reverse(a.rating as i32));
        }
        "新人榜" => {
            // 新人榜：新作者作品
            rank_filtered.retain(|c| {
                c.tags.contains(&"纯爱".into())
                    || c.tags.contains(&"年上御姐".into())
                    || c.tags.contains(&"玄幻".into())
            });
        }
        _ => {
            // 默认综合周榜
        }
    }

    // 2. 进一步通过题材标签 Filter 进行过滤
    let final_filtered: Vec<Character> = rank_filtered
        .into_iter()
        .filter(|c| {
            let match_tag = if cur_tag == "全部" {
                true
            } else {
                c.tags.iter().any(|t| t == &cur_tag)
            };
            if !match_tag {
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

    rsx! {
        div { class: "relative flex h-full w-full flex-col gap-4 overflow-hidden",
            // ==========================================
            // 顶部榜单 Tab 与下层筛选系统 (对齐图1/2/5)
            // ==========================================
            div { class: "flex shrink-0 flex-col gap-3 px-1",
                // 1. 第一行: 榜单 Tab 页面 (周榜 / 日榜 / 飙升榜 / 新人榜) + 搜索 + 创作跳转 (对齐图5红框)
                div { class: "flex flex-wrap items-center justify-between gap-3",
                    // 榜单 Tab 页面切换器
                    div { class: "flex items-center gap-1 rounded-full border border-zinc-800 bg-zinc-900/80 p-1 text-xs backdrop-blur-md",
                        for rank in ["周榜", "日榜", "飙升榜", "新人榜"] {
                            {
                                let is_active_tab = active_rank_tab() == rank;
                                let rank_str = rank.to_string();
                                rsx! {
                                    button {
                                        key: "{rank}",
                                        class: if is_active_tab {
                                            "rounded-full bg-zinc-100 px-3.5 py-1 font-bold text-zinc-900 shadow-sm transition-all"
                                        } else {
                                            "rounded-full px-3.5 py-1 font-medium text-zinc-400 hover:text-zinc-200 transition-colors"
                                        },
                                        onclick: move |_| active_rank_tab.set(rank_str.clone()),
                                        match rank {
                                            "周榜" => "🏆 周榜",
                                            "日榜" => "🔥 日榜",
                                            "飙升榜" => "⚡ 飙升榜",
                                            _ => "✨ 新人榜",
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 搜索框 + 创作按钮 (改名为「创作」，点击跳转到创作中心，对齐需求3)
                    div { class: "flex flex-1 items-center justify-end gap-2.5 max-w-md",
                        div { class: "flex flex-1 items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900/80 px-3 py-1.5 backdrop-blur-md",
                            span { class: "text-xs text-zinc-500", "🔍" }
                            input {
                                class: "w-full bg-transparent text-xs text-zinc-200 outline-none placeholder:text-zinc-600",
                                placeholder: "搜索作品名称、标签、创作者…",
                                value: "{search()}",
                                oninput: move |e| search.set(e.value()),
                            }
                        }
                        button {
                            class: "flex shrink-0 items-center gap-1.5 rounded-full bg-gradient-to-r from-purple-600 to-pink-600 px-4 py-1.5 text-xs font-bold text-white shadow-lg shadow-purple-600/30 transition-all hover:scale-105",
                            title: "前往创作者中心",
                            onclick: move |_| on_goto_studio.call(()),
                            span { "✍️" }
                            span { "创作" }
                        }
                    }
                }

                // 2. 第二行: 类别题材多选筛选 Filter (对齐图2与图5下层)
                div { class: "flex items-center gap-2 overflow-x-auto pb-1 text-xs text-zinc-400 no-scrollbar",
                    for cat in ["全部", "现代", "娱乐圈", "无限流", "科幻", "赛博朋克", "悬疑", "修仙", "同人", "纯爱"] {
                        {
                            let is_selected = active_tag() == cat;
                            let cat_name = cat.to_string();
                            rsx! {
                                button {
                                    key: "{cat}",
                                    class: if is_selected {
                                        "rounded-lg bg-purple-950/60 border border-purple-500/50 px-2.5 py-1 font-semibold text-purple-200 transition-all shadow-sm"
                                    } else {
                                        "rounded-lg border border-zinc-800/80 bg-zinc-900/50 px-2.5 py-1 hover:bg-zinc-800 hover:text-zinc-200 transition-colors"
                                    },
                                    onclick: move |_| active_tag.set(cat_name.clone()),
                                    if cat == "全部" {
                                        "{cat}"
                                    } else {
                                        "#{cat}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ==========================================
            // 响应式 5 栏网格 (对齐图5: Web 5 栏、平板 3 栏、手机 1 栏)
            // ==========================================
            div { class: "min-h-0 flex-1 overflow-y-auto px-1 pr-2 pb-6",
                if final_filtered.is_empty() {
                    EmptyState {
                        title: "当前榜单下没有匹配的作品".to_string(),
                        hint: "尝试重置题材分类筛选或切换榜单".to_string(),
                    }
                } else {
                    div { class: "grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 xl:grid-cols-5",
                        for (idx, c) in final_filtered.into_iter().enumerate() {
                            {
                                let c_cover = c.clone();
                                let on_tag_click = move |tag: String| {
                                    active_tag.set(tag);
                                };
                                rsx! {
                                    CharacterCardItem {
                                        key: "{c.id}",
                                        index: idx,
                                        char: c.clone(),
                                        on_click_card: move |_| cover_target.set(Some(c_cover.clone())),
                                        on_select_tag: on_tag_click,
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
                        }
                    }
                }
            }
        }
    }
}

/// =========================================================================
/// 剧本卡片组件:
/// - 彻底修复 Tooltip 被遮挡穿透 bug (hover 时提升父卡片自身 z-index 为 z-30，浮窗纯黑实色 z-50)
/// - 移除编辑和删除按钮 (对齐图3需求)
/// - 纯数值角标 (对齐图11需求)
/// - 智能位置: 首行向下，后续行向上，动画平缓 (对齐图12需求)
/// =========================================================================
#[component]
fn CharacterCardItem(
    index: usize,
    char: Character,
    on_click_card: EventHandler<MouseEvent>,
    on_select_tag: EventHandler<String>,
) -> Element {
    let mut is_hovered = use_signal(|| false);

    // 智能计算 Tooltip 出现位置:
    // 如果是首行(0..5)，向下弹出以免被顶部视口切断；若是下方卡片，则向上弹出！
    let is_first_row = index < 5;

    // 核心 Bug 修复 (对齐图1与图2红框):
    // hover 时必须给宿主卡片容器自身提升 z-index 到 z-30！
    // 否则在 Grid 中后面的兄弟卡片天然层叠在前面卡片上面，导致 Tooltip 被下方的头像穿透遮挡！
    let card_z_class = if is_hovered() {
        "relative z-30"
    } else {
        "relative z-0"
    };

    rsx! {
        div {
            class: "{card_z_class} group flex cursor-pointer flex-col overflow-visible rounded-2xl border border-zinc-800/80 bg-zinc-900/60 transition-all duration-300 ease-out hover:-translate-y-0.5 hover:border-purple-500/40 hover:bg-zinc-900 hover:shadow-lg hover:shadow-purple-950/20",
            onmouseenter: move |_| is_hovered.set(true),
            onmouseleave: move |_| is_hovered.set(false),
            onclick: move |e| on_click_card.call(e),

            // 卡片封面区域: 纯数值角标 (对齐图11)
            div { class: "relative flex h-36 w-full items-center justify-center overflow-hidden rounded-t-2xl bg-gradient-to-b from-purple-950/40 via-zinc-850 to-zinc-900",
                // 纯数值角标 (对齐图11)
                div { class: "absolute left-2.5 top-2.5 z-10 flex items-center rounded-md bg-black/70 px-2 py-0.5 text-[10px] font-semibold text-rose-300 backdrop-blur-md border border-rose-500/20 tabular-nums select-none",
                    "{char.heat_val}"
                }

                // 中心头像微标
                Avatar {
                    name: char.name.clone(),
                    src: char.avatar_src.clone(),
                    size: "h-14 w-14 text-lg".to_string(),
                }
            }

            // 卡片正文信息: 无任何滚动条，严谨两行截断
            div { class: "flex flex-1 flex-col gap-2 p-3.5",
                div { class: "flex items-start justify-between gap-1",
                    h3 { class: "line-clamp-1 text-xs font-bold leading-5 text-zinc-100 group-hover:text-purple-300 transition-colors",
                        "{char.name}"
                    }
                }

                p { class: "line-clamp-2 min-h-8 text-[11px] leading-4 text-zinc-400",
                    if char.description.is_empty() { "（无设定描述）" } else { "{char.description}" }
                }

                div { class: "mt-auto flex items-center justify-between border-t border-zinc-800/60 pt-2 text-[10px] text-zinc-500",
                    span { class: "truncate max-w-[100px]", "作者: {char.author}" }
                    span { class: "font-semibold text-amber-400/90 tabular-nums", "★ {char.rating:.1}" }
                }

                // 底部标签：支持直接点击联动筛选 (对齐图12需求)
                if !char.tags.is_empty() {
                    div { class: "flex flex-wrap gap-1 overflow-hidden h-5",
                        for t in char.tags.iter().take(2) {
                            {
                                let tag_str = t.clone();
                                rsx! {
                                    button {
                                        key: "{t}",
                                        class: "rounded-md bg-zinc-800/80 px-1.5 py-0.5 text-[9px] text-zinc-400 hover:text-purple-300 hover:bg-zinc-700 transition-colors truncate max-w-[80px]",
                                        onclick: move |e| {
                                            e.stop_propagation();
                                            on_select_tag.call(tag_str.clone());
                                        },
                                        "#{t}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ==========================================
            // Hover Tooltip: 纯黑高对比度实色背景，z-50，彻底杜绝下方卡片穿透！
            // ==========================================
            if is_hovered() {
                div {
                    class: if is_first_row {
                        "pointer-events-none absolute top-full left-1/2 z-50 hidden w-72 -translate-x-1/2 mt-2 flex-col gap-2.5 rounded-2xl border border-purple-500/60 bg-[#09090b] p-4 text-xs shadow-2xl shadow-black xl:flex transition-all duration-300 ease-out select-none"
                    } else {
                        "pointer-events-none absolute bottom-full left-1/2 z-50 hidden w-72 -translate-x-1/2 mb-2 flex-col gap-2.5 rounded-2xl border border-purple-500/60 bg-[#09090b] p-4 text-xs shadow-2xl shadow-black xl:flex transition-all duration-300 ease-out select-none"
                    },
                    div { class: "flex items-center justify-between border-b border-zinc-800 pb-2",
                        span { class: "font-bold text-zinc-100", "{char.name}" }
                        span { class: "text-[10px] text-rose-400 font-semibold tabular-nums", "{char.heat_val}" }
                    }
                    div { class: "flex flex-col gap-1 text-[11px] text-zinc-300 leading-relaxed",
                        span { class: "font-semibold text-purple-300", "【作品全貌与剧情导语】" }
                        p { "{char.description}" }
                    }
                    if !char.quote.is_empty() {
                        div { class: "rounded-xl border border-zinc-800 bg-zinc-900/80 p-2 text-[10px] italic text-zinc-400",
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

/// =========================================================================
/// 独立专属的「创作中心展示页」(StudioPage) (对齐需求3与图4)
/// 列出用户自己的所有作品卡牌，点击弹出带景深高斯模糊的四步编辑器 Modal
/// =========================================================================
#[component]
pub fn StudioPage() -> Element {
    let mut studio_tab = use_signal(|| "剧本创作".to_string());

    let mut my_works = use_signal(move || {
        seed_all_characters()
            .into_iter()
            .filter(|c| c.is_user_created)
            .collect::<Vec<Character>>()
    });

    let mut editing_target = use_signal(|| None::<Character>);
    let mut delete_id = use_signal(|| None::<usize>);

    rsx! {
        div { class: "relative flex h-full w-full flex-col gap-4 overflow-hidden",
            // 创作中心头部控制台看板
            div { class: "flex shrink-0 flex-wrap items-center justify-between gap-4 rounded-2xl border border-purple-500/30 bg-gradient-to-r from-purple-950/30 via-zinc-900/60 to-zinc-950 p-5 backdrop-blur-xl shadow-lg",
                div { class: "flex items-center gap-3",
                    div { class: "flex h-12 w-12 items-center justify-center rounded-2xl bg-purple-900/40 border border-purple-500/40 text-2xl shadow-inner",
                        "🎨"
                    }
                    div { class: "flex flex-col gap-0.5",
                        div { class: "flex items-center gap-2",
                            h1 { class: "font-serif text-lg font-bold text-white", "我的创作中心" }
                            span { class: "rounded-full bg-purple-500/20 border border-purple-500/30 px-2 py-0.2 text-[10px] font-bold text-purple-300", "CREATOR STUDIO" }
                        }
                        p { class: "text-xs text-zinc-400", "管理你的专属剧本、玩家人格化身与世界书条目。全流程自定义你的互动宇宙。" }
                    }
                }

                // 子功能 Tab 切换
                div { class: "flex items-center gap-1 rounded-full border border-zinc-800 bg-zinc-900/80 p-1 text-xs select-none",
                    for t in ["剧本创作", "我的人格", "世界书"] {
                        {
                            let is_act = studio_tab() == t;
                            let t_str = t.to_string();
                            rsx! {
                                button {
                                    key: "{t}",
                                    class: if is_act {
                                        "rounded-full bg-zinc-100 px-3.5 py-1 font-bold text-zinc-900 shadow-sm transition-all"
                                    } else {
                                        "rounded-full px-3.5 py-1 font-medium text-zinc-400 hover:text-zinc-200 transition-colors"
                                    },
                                    onclick: move |_| studio_tab.set(t_str.clone()),
                                    "{t}"
                                }
                            }
                        }
                    }
                }
            }

            // 选项卡内容区
            div { class: "min-h-0 flex-1 overflow-hidden",
                match studio_tab().as_str() {
                    "我的人格" => rsx! {
                        tavern_page_personas::PersonasPage {}
                    },
                    "世界书" => rsx! {
                        tavern_page_lorebook::LorebookPage {}
                    },
                    _ => rsx! {
                        // 剧本作品网格: Web 5 栏 / 平板 3 栏 / 手机 1 栏 (对齐用户严格要求)
                        div { class: "h-full w-full overflow-y-auto px-1 pr-2 pb-6",
                            div { class: "grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 xl:grid-cols-5",
                                // 1. 醒目的“+ 新建剧本”大卡片
                                div {
                                    class: "group flex min-h-[220px] cursor-pointer flex-col items-center justify-center gap-3 rounded-2xl border-2 border-dashed border-zinc-800 bg-zinc-900/30 p-6 text-center transition-all hover:border-purple-500/60 hover:bg-purple-950/20 select-none",
                                    onclick: move |_| {
                                        let next_id = my_works().len() + 101;
                                        editing_target.set(Some(Character::empty(next_id)));
                                    },
                                    div { class: "flex h-12 w-12 items-center justify-center rounded-full bg-zinc-800 text-xl text-purple-400 group-hover:scale-110 group-hover:bg-purple-600 group-hover:text-white transition-all",
                                        "+"
                                    }
                                    span { class: "text-xs font-bold text-zinc-200 group-hover:text-purple-300 transition-colors", "创建新剧本作品" }
                                    span { class: "text-[11px] text-zinc-500", "包含设定、Prompt 矩阵、进场展映与行动分支" }
                                }

                                // 2. 我的作品卡牌列表
                                for work in my_works() {
                                    {
                                        let w_clone = work.clone();
                                        let w_edit = work.clone();
                                        rsx! {
                                            div {
                                                key: "{work.id}",
                                                class: "group relative flex flex-col justify-between overflow-hidden rounded-2xl border border-zinc-800/80 bg-zinc-900/60 p-4 transition-all hover:border-purple-500/50 hover:bg-zinc-900 hover:shadow-xl",
                                                div { class: "flex items-center justify-between",
                                                    if work.is_published {
                                                        span { class: "flex items-center gap-1 rounded-full bg-emerald-500/10 border border-emerald-500/30 px-2 py-0.5 text-[10px] font-bold text-emerald-400",
                                                            span { "●" }
                                                            span { "已发布" }
                                                        }
                                                    } else {
                                                        span { class: "flex items-center gap-1 rounded-full bg-amber-500/10 border border-amber-500/30 px-2 py-0.5 text-[10px] font-bold text-amber-400",
                                                            span { "●" }
                                                            span { "草稿中" }
                                                        }
                                                    }

                                                    div { class: "flex items-center gap-1",
                                                        button {
                                                            class: "rounded-md p-1 text-zinc-400 hover:bg-zinc-800 hover:text-purple-300 text-xs transition-colors",
                                                            title: "编辑剧本",
                                                            onclick: move |_| editing_target.set(Some(w_edit.clone())),
                                                            "✎"
                                                        }
                                                        button {
                                                            class: "rounded-md p-1 text-zinc-400 hover:bg-zinc-800 hover:text-rose-400 text-xs transition-colors",
                                                            title: "删除剧本",
                                                            onclick: move |_| delete_id.set(Some(work.id)),
                                                            "✕"
                                                        }
                                                    }
                                                }

                                                div { class: "flex flex-col gap-1.5 py-3",
                                                    h3 { class: "font-serif text-sm font-bold text-zinc-100 group-hover:text-purple-300 transition-colors line-clamp-1",
                                                        "{work.name}"
                                                    }
                                                    p { class: "line-clamp-2 text-xs leading-5 text-zinc-400",
                                                        if work.description.is_empty() { "暂未填写作品描述…" } else { "{work.description}" }
                                                    }
                                                }

                                                div { class: "flex items-center justify-between border-t border-zinc-800/80 pt-3 text-[11px] text-zinc-500",
                                                    span { class: "truncate max-w-[120px]", "模型: {work.default_model}" }
                                                    button {
                                                        class: "font-semibold text-purple-400 hover:text-purple-300 transition-colors",
                                                        onclick: move |_| editing_target.set(Some(w_clone.clone())),
                                                        "继续编辑 ➜"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                }
            }

            // 弹窗式四步创作编辑器 (Modal)
            if let Some(target) = editing_target() {
                div {
                    class: "fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-xl p-4 sm:p-6 transition-all duration-300",
                    onclick: move |_| editing_target.set(None),
                    div {
                        class: "relative flex h-full max-h-[820px] w-full max-w-3xl flex-col overflow-hidden rounded-3xl border border-zinc-800 bg-zinc-900 shadow-2xl",
                        onclick: move |e| e.stop_propagation(),
                        CreatorStudioModal {
                            initial: target.clone(),
                            on_close: move |_| editing_target.set(None),
                            on_save: move |saved: Character| {
                                let mut list = my_works.write();
                                if let Some(idx) = list.iter().position(|x| x.id == saved.id) {
                                    list[idx] = saved;
                                } else {
                                    list.insert(0, saved);
                                }
                                editing_target.set(None);
                            },
                        }
                    }
                }
            }

            Dialog {
                title: "删除创作者作品".to_string(),
                open: delete_id().is_some(),
                on_cancel: move |_| delete_id.set(None),
                on_confirm: move |_| {
                    if let Some(id) = delete_id() {
                        my_works.write().retain(|c| c.id != id);
                        delete_id.set(None);
                    }
                },
                "确定要删除这部创作者作品吗？此操作不可恢复。"
            }
        }
    }
}

/// =========================================================================
/// 弹窗版四步创作编辑器组件 (对齐图4-图10，全面增加 select-none，去除突兀顶栏)
/// =========================================================================
#[component]
fn CreatorStudioModal(
    initial: Character,
    on_close: EventHandler<()>,
    on_save: EventHandler<Character>,
) -> Element {
    let mut current_step = use_signal(|| 1usize);

    // 基础设定 (Step 01)
    let mut name = use_signal(|| initial.name.clone());
    let mut description = use_signal(|| initial.description.clone());
    let mut quote = use_signal(|| initial.quote.clone());

    // 角色背景 (Step 02)
    let mut personality = use_signal(|| initial.personality.clone());
    let mut scenario = use_signal(|| initial.scenario.clone());
    let mut first_mes = use_signal(|| initial.first_mes.clone());
    let mut mes_example = use_signal(|| initial.mes_example.clone());

    // 追加设定 (Step 03)
    let mut cg_trigger = use_signal(|| "AI".to_string());
    let mut cg_mapping = use_signal(|| "聊天".to_string());

    // 保存与发布 (Step 04)
    let mut tags_str = use_signal(|| initial.tags.join(", "));
    let mut default_model = use_signal(|| initial.default_model.clone());
    let mut is_anonymous = use_signal(|| false);
    let mut disable_custom_css = use_signal(|| false);
    let _agree_rules = use_signal(|| true);

    // 专业模型选择抽屉
    let mut model_selector_open = use_signal(|| false);
    let mut model_vendor = use_signal(|| "All".to_string());
    let _model_search = use_signal(String::new);

    let can_publish = !name().trim().is_empty();

    rsx! {
        div { class: "flex h-full w-full flex-col overflow-hidden text-xs text-zinc-100",
            // 优雅的弹窗顶栏 (彻底替代原来突兀的整行全黑条，对齐图3改进需求)
            div { class: "flex shrink-0 items-center justify-between border-b border-zinc-800 px-6 py-4 bg-zinc-950/60 select-none",
                div { class: "flex items-center gap-2",
                    span { class: "font-serif text-sm font-bold text-white",
                        if initial.name.is_empty() { "新建剧本作品" } else { "编辑剧本设定" }
                    }
                    span { class: "rounded-md bg-purple-500/20 px-2 py-0.5 text-[10px] font-bold text-purple-300", "MODAL" }
                }
                button {
                    class: "flex h-7 w-7 items-center justify-center rounded-full text-zinc-400 hover:bg-zinc-800 hover:text-white transition-colors text-sm",
                    title: "关闭弹窗",
                    onclick: move |_| on_close.call(()),
                    "✕"
                }
            }

            // 四步步进条进度指示器 (对齐图4-8，核心: 全面增加 select-none，坚决禁止文字选中文本高亮！对齐图3红框需求)
            div { class: "relative flex shrink-0 items-center justify-between px-8 sm:px-14 py-4 border-b border-zinc-800/80 bg-zinc-900/60 select-none",
                // 连线背景
                div { class: "absolute left-12 right-12 top-8 h-0.5 bg-zinc-800 -z-0" }
                // 高亮步进连线
                div {
                    class: "absolute left-12 top-8 h-0.5 bg-sky-500 transition-all duration-300 -z-0",
                    style: match current_step() {
                        1 => "width: 0%;",
                        2 => "width: 33%;",
                        3 => "width: 66%;",
                        _ => "width: calc(100% - 6rem);",
                    },
                }

                // 01 基础设定 (加 select-none)
                div {
                    class: "relative z-10 flex flex-col items-center gap-1 cursor-pointer select-none",
                    onclick: move |_| current_step.set(1),
                    span { class: "text-[11px] font-medium text-zinc-400 select-none", "基础设定" }
                    div {
                        class: if current_step() >= 1 {
                            "flex h-7 w-7 items-center justify-center rounded-full bg-sky-500 text-xs font-bold text-white shadow-md shadow-sky-500/30 select-none"
                        } else {
                            "flex h-7 w-7 items-center justify-center rounded-full bg-zinc-800 text-xs text-zinc-500 select-none"
                        },
                        "01"
                    }
                }

                // 02 角色与背景设定 (加 select-none)
                div {
                    class: "relative z-10 flex flex-col items-center gap-1 cursor-pointer select-none",
                    onclick: move |_| current_step.set(2),
                    span { class: "text-[11px] font-medium text-zinc-400 select-none", "角色与背景设定" }
                    div {
                        class: if current_step() >= 2 {
                            "flex h-7 w-7 items-center justify-center rounded-full bg-sky-500 text-xs font-bold text-white shadow-md shadow-sky-500/30 select-none"
                        } else {
                            "flex h-7 w-7 items-center justify-center rounded-full bg-zinc-800 text-xs text-zinc-500 select-none"
                        },
                        "02"
                    }
                }

                // 03 追加设定 (加 select-none)
                div {
                    class: "relative z-10 flex flex-col items-center gap-1 cursor-pointer select-none",
                    onclick: move |_| current_step.set(3),
                    span { class: "text-[11px] font-medium text-zinc-400 select-none", "追加设定" }
                    div {
                        class: if current_step() >= 3 {
                            "flex h-7 w-7 items-center justify-center rounded-full bg-sky-500 text-xs font-bold text-white shadow-md shadow-sky-500/30 select-none"
                        } else {
                            "flex h-7 w-7 items-center justify-center rounded-full bg-zinc-800 text-xs text-zinc-500 select-none"
                        },
                        "03"
                    }
                }

                // 04 保存与发布 (加 select-none)
                div {
                    class: "relative z-10 flex flex-col items-center gap-1 cursor-pointer select-none",
                    onclick: move |_| current_step.set(4),
                    span { class: "text-[11px] font-medium text-zinc-400 select-none", "保存与发布" }
                    div {
                        class: if current_step() >= 4 {
                            "flex h-7 w-7 items-center justify-center rounded-full bg-sky-500 text-xs font-bold text-white shadow-md shadow-sky-500/30 select-none"
                        } else {
                            "flex h-7 w-7 items-center justify-center rounded-full bg-zinc-800 text-xs text-zinc-500 select-none"
                        },
                        "04"
                    }
                }
            }

            // 表单工作区
            div { class: "min-h-0 flex-1 overflow-y-auto p-6 sm:p-8 space-y-4",
                match current_step() {
                    1 => rsx! {
                        div { class: "space-y-4",
                            Field { label: "剧本名字 *",
                                input {
                                    class: "w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3.5 py-2.5 text-zinc-100 outline-none focus:border-sky-500",
                                    placeholder: "输入剧本标题",
                                    value: "{name()}",
                                    oninput: move |e| name.set(e.value()),
                                }
                            }
                            Field { label: "简要剧情描述",
                                textarea {
                                    class: "h-24 w-full resize-none rounded-xl border border-zinc-800 bg-zinc-950 px-3.5 py-2.5 text-zinc-100 outline-none focus:border-sky-500",
                                    placeholder: "概括核心看点、冲突与题材风格…",
                                    value: "{description()}",
                                    oninput: move |e| description.set(e.value()),
                                }
                            }
                            Field { label: "名句引言 (展映封面用)",
                                input {
                                    class: "w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3.5 py-2.5 text-zinc-100 outline-none focus:border-sky-500 font-serif italic",
                                    placeholder: "“每一张合同背后，都是未标注价格的命运。”",
                                    value: "{quote()}",
                                    oninput: move |e| quote.set(e.value()),
                                }
                            }
                        }
                    },
                    2 => rsx! {
                        div { class: "space-y-4",
                            Field { label: "角色设定 (System Prompt / Personality)",
                                textarea {
                                    class: "h-32 w-full resize-none rounded-xl border border-zinc-800 bg-zinc-950 px-3.5 py-2.5 font-mono text-[11px] leading-5 text-zinc-100 outline-none focus:border-sky-500",
                                    placeholder: "设定角色性格、身份动机与不可违抗的行为准则…",
                                    value: "{personality()}",
                                    oninput: move |e| personality.set(e.value()),
                                }
                            }
                            Field { label: "初始场景 (Scenario)",
                                textarea {
                                    class: "h-20 w-full resize-none rounded-xl border border-zinc-800 bg-zinc-950 px-3.5 py-2.5 text-zinc-100 outline-none focus:border-sky-500",
                                    placeholder: "初始对峙场景或环境描写…",
                                    value: "{scenario()}",
                                    oninput: move |e| scenario.set(e.value()),
                                }
                            }
                            Field { label: "开场白 (First Message)",
                                textarea {
                                    class: "h-20 w-full resize-none rounded-xl border border-zinc-800 bg-zinc-950 px-3.5 py-2.5 text-zinc-100 outline-none focus:border-sky-500",
                                    placeholder: "玩家开局收到的第一句问候或旁白…",
                                    value: "{first_mes()}",
                                    oninput: move |e| first_mes.set(e.value()),
                                }
                            }
                            Field { label: "少样本引导对话 (Mes Example)",
                                textarea {
                                    class: "h-20 w-full resize-none rounded-xl border border-zinc-800 bg-zinc-950 px-3.5 py-2.5 font-mono text-[11px] text-zinc-100 outline-none focus:border-sky-500",
                                    placeholder: "<START>\n{{user}}: 你好\n{{char}}: 很高兴见到你。",
                                    value: "{mes_example()}",
                                    oninput: move |e| mes_example.set(e.value()),
                                }
                            }
                        }
                    },
                    3 => rsx! {
                        div { class: "space-y-4",
                            div { class: "flex items-center justify-between",
                                span { class: "font-semibold text-zinc-300", "开场白扩展" }
                                span { class: "text-zinc-500", "0 / 40" }
                            }
                            button { class: "w-full rounded-xl border border-dashed border-zinc-800 py-2.5 text-zinc-400 hover:border-zinc-600 hover:text-zinc-200 transition-colors",
                                "+ 添加开场白分支"
                            }

                            div { class: "space-y-1.5 pt-2",
                                span { class: "font-semibold text-zinc-300", "正则表达式清洗脚本" }
                                p { class: "text-[11px] text-zinc-500", "支持将大模型输出中的特定文本/标记动态替换为对应展示卡片" }
                                button { class: "w-full rounded-xl border border-dashed border-zinc-800 py-2.5 text-zinc-400 hover:border-zinc-600 hover:text-zinc-200 transition-colors",
                                    "+ 添加正则脚本"
                                }
                            }

                            div { class: "space-y-3 rounded-2xl border border-zinc-800 bg-zinc-950/60 p-4 mt-2",
                                span { class: "font-semibold text-zinc-300", "CG 图片与立绘映射" }
                                div { class: "grid grid-cols-2 gap-4 pt-1",
                                    div { class: "space-y-1.5",
                                        span { class: "text-zinc-400", "触发区域:" }
                                        div { class: "flex items-center gap-2",
                                            label { class: "flex items-center gap-1.5 cursor-pointer",
                                                input { r#type: "radio", name: "trigger", checked: cg_trigger() == "AI", onchange: move |_| cg_trigger.set("AI".into()) }
                                                span { "AI" }
                                            }
                                            label { class: "flex items-center gap-1.5 cursor-pointer",
                                                input { r#type: "radio", name: "trigger", checked: cg_trigger() == "用户", onchange: move |_| cg_trigger.set("用户".into()) }
                                                span { "用户" }
                                            }
                                        }
                                    }
                                    div { class: "space-y-1.5",
                                        span { class: "text-zinc-400", "区域映射:" }
                                        div { class: "flex items-center gap-2",
                                            label { class: "flex items-center gap-1.5 cursor-pointer",
                                                input { r#type: "radio", name: "map", checked: cg_mapping() == "背景图片", onchange: move |_| cg_mapping.set("背景图片".into()) }
                                                span { "背景图片" }
                                            }
                                            label { class: "flex items-center gap-1.5 cursor-pointer",
                                                input { r#type: "radio", name: "map", checked: cg_mapping() == "聊天", onchange: move |_| cg_mapping.set("聊天".into()) }
                                                span { "聊天" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    _ => rsx! {
                        div { class: "space-y-4",
                            div { class: "space-y-1.5",
                                span { class: "font-semibold text-zinc-300", "分类标签 (逗号分隔)" }
                                input {
                                    class: "w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3.5 py-2.5 text-zinc-100 outline-none focus:border-sky-500",
                                    value: "{tags_str()}",
                                    oninput: move |e| tags_str.set(e.value()),
                                }
                            }

                            // 默认游玩模型选择抽屉
                            div { class: "flex items-center justify-between rounded-xl border border-zinc-800 bg-zinc-950/60 p-3",
                                span { class: "font-semibold text-zinc-300", "默认推荐模型" }
                                button {
                                    class: "flex items-center gap-2 rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-1.5 text-zinc-200 hover:border-sky-500 transition-colors",
                                    onclick: move |_| model_selector_open.set(true),
                                    span { "{default_model()}" }
                                    span { "⌵" }
                                }
                            }

                            div { class: "flex items-center justify-between border-t border-zinc-800/80 pt-3",
                                div { class: "flex flex-col",
                                    span { class: "font-semibold text-zinc-300", "匿名发布" }
                                    span { class: "text-[11px] text-zinc-500", "隐藏作品作者用户名" }
                                }
                                input {
                                    r#type: "checkbox",
                                    class: "h-4 w-4 accent-sky-500",
                                    checked: is_anonymous(),
                                    oninput: move |e| is_anonymous.set(e.value().parse().unwrap_or(false)),
                                }
                            }

                            div { class: "flex items-center justify-between border-t border-zinc-800/80 pt-3",
                                div { class: "flex flex-col",
                                    span { class: "font-semibold text-zinc-300", "禁用包含内置CSS的Mod" }
                                    span { class: "text-[11px] text-zinc-500", "开启后，用户将无法启用带有内置CSS的Mod" }
                                }
                                input {
                                    r#type: "checkbox",
                                    class: "h-4 w-4 accent-sky-500",
                                    checked: disable_custom_css(),
                                    oninput: move |e| disable_custom_css.set(e.value().parse().unwrap_or(false)),
                                }
                            }

                            // 发布须知卡片
                            div { class: "flex items-center gap-3 rounded-2xl border border-zinc-800 bg-zinc-950/80 p-3.5",
                                div { class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-purple-950/50 border border-purple-500/30 text-xl",
                                    "👘"
                                }
                                span { class: "text-[11px] text-zinc-400 leading-4",
                                    "已发布的作品将进入周榜与日榜展映。未公开发布的草稿作品仅存放在创作中心，随时可一键发布。"
                                }
                            }
                        }
                    },
                }
            }

            // 弹窗底栏: 上一步 / 下一步 / 立即保存发布 (加 select-none)
            div { class: "flex shrink-0 items-center justify-between border-t border-zinc-800 px-6 py-4 bg-zinc-950/80 select-none",
                div {
                    if current_step() > 1 {
                        button {
                            class: "rounded-full bg-zinc-800 px-5 py-1.5 text-xs font-semibold text-zinc-300 hover:bg-zinc-700 transition-colors select-none",
                            onclick: move |_| current_step.set(current_step() - 1),
                            "上一步"
                        }
                    }
                }

                div {
                    if current_step() < 4 {
                        button {
                            class: "rounded-full bg-sky-500 px-7 py-1.5 text-xs font-bold text-white shadow-lg shadow-sky-500/30 hover:bg-sky-400 transition-colors select-none",
                            onclick: move |_| current_step.set(current_step() + 1),
                            "下一步"
                        }
                    } else {
                        button {
                            class: "rounded-full bg-gradient-to-r from-sky-500 to-blue-600 px-8 py-2 text-xs font-bold text-white shadow-xl shadow-sky-500/40 hover:scale-105 transition-all disabled:opacity-40 select-none",
                            disabled: !can_publish,
                            onclick: move |_| {
                                let tags = tags_str()
                                    .split(',')
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                on_save.call(Character {
                                    id: initial.id,
                                    name: name(),
                                    sub_title: "MY WORK // CREATOR STUDIO".into(),
                                    author: initial.author.clone(),
                                    heat_val: initial.heat_val.clone(),
                                    rating: initial.rating,
                                    quote: quote(),
                                    avatar_src: initial.avatar_src.clone(),
                                    description: description(),
                                    personality: personality(),
                                    scenario: scenario(),
                                    first_mes: first_mes(),
                                    mes_example: mes_example(),
                                    tags,
                                    default_model: default_model(),
                                    is_user_created: true,
                                    is_published: true,
                                    updated_at: "刚刚".into(),
                                });
                            },
                            "保存并发布 🚀"
                        }
                    }
                }
            }

            // 专业模型选择器抽屉 (对齐图9/10)
            if model_selector_open() {
                div {
                    class: "absolute inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-md select-none",
                    onclick: move |_| model_selector_open.set(false),
                    div {
                        class: "relative flex h-full max-h-[580px] w-full max-w-lg flex-col overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-2xl text-xs",
                        onclick: move |e| e.stop_propagation(),

                        div { class: "flex items-center justify-between border-b border-zinc-800 pb-3",
                            div { class: "flex items-center gap-3 overflow-x-auto",
                                for v in ["All", "Gemini", "DeepSeek", "Claude", "GLM", "ChatGPT"] {
                                    {
                                        let is_v = model_vendor() == v;
                                        let v_name = v.to_string();
                                        rsx! {
                                            button {
                                                key: "{v}",
                                                class: if is_v {
                                                    "border-b-2 border-sky-400 pb-1 font-bold text-sky-400"
                                                } else {
                                                    "pb-1 font-medium text-zinc-400 hover:text-zinc-200"
                                                },
                                                onclick: move |_| model_vendor.set(v_name.clone()),
                                                "{v}"
                                            }
                                        }
                                    }
                                }
                            }
                            button {
                                class: "text-zinc-500 hover:text-zinc-200",
                                onclick: move |_| model_selector_open.set(false),
                                "✕"
                            }
                        }

                        div { class: "flex-1 overflow-y-auto space-y-2 py-3 pr-1",
                            for (m_name, cost, speed, desc) in [
                                ("gemini-3.1-pro-preview-high", "0.140", "85.27%", "智力高 性价比高 原生思维链"),
                                ("gemini-3.7-flash-low", "0.086", "91.55%", "极致迅捷响应，低延迟"),
                                ("claude-3-5-sonnet", "0.220", "88.40%", "顶级剧情推演与逻辑严密性"),
                                ("deepseek-chat", "0.040", "94.20%", "高性价比，中文小说文采出众"),
                                ("gpt-4o", "0.180", "86.10%", "全能稳健输出，多语言泛化极强"),
                            ] {
                                {
                                    let is_cur = default_model() == m_name;
                                    let name_clone = m_name.to_string();
                                    rsx! {
                                        div {
                                            key: "{m_name}",
                                            class: if is_cur {
                                                "flex flex-col gap-1 rounded-xl border border-sky-500/50 bg-sky-950/20 p-2.5"
                                            } else {
                                                "flex flex-col gap-1 rounded-xl border border-zinc-800 bg-zinc-950/60 p-2.5 hover:border-zinc-700"
                                            },
                                            div { class: "flex items-center justify-between",
                                                div { class: "flex items-center gap-1.5",
                                                    span { class: "h-2 w-2 rounded-full bg-emerald-400" }
                                                    span { class: "font-bold text-zinc-100", "{m_name}" }
                                                }
                                                button {
                                                    class: "rounded-lg border border-zinc-700 bg-zinc-800 px-2 py-0.5 text-[11px] text-zinc-300 hover:bg-sky-500 hover:text-white transition-colors",
                                                    onclick: move |_| {
                                                        default_model.set(name_clone.clone());
                                                        model_selector_open.set(false);
                                                    },
                                                    "选择"
                                                }
                                            }
                                            div { class: "flex items-center gap-3 text-[10px] text-zinc-500",
                                                span { "价格: {cost}" }
                                                span { "出字率: {speed}" }
                                                span { "特点: {desc}" }
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
}
