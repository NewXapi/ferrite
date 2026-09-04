//! 故事流与节点大纲导航数据结构测试
use tavern_page_chat::{seed_sessions, seed_story_items};

#[test]
fn session_list_has_elements_and_valid() {
    let sessions = seed_sessions();
    assert!(!sessions.is_empty(), "种子会话列表必须非空");
    assert_eq!(sessions.len(), 3, "默认提供 3 条时间线分支");
}

#[test]
fn story_items_have_valid_ids_and_nav_titles() {
    let items = seed_story_items();
    assert!(!items.is_empty(), "种子剧情节点非空");
    for item in &items {
        assert!(item.id() > 0, "节点 ID 必须大于 0");
        let (kind, title) = item.nav_title();
        assert!(!kind.is_empty(), "大纲类别必须非空");
        assert!(!title.is_empty(), "大纲标题内容必须非空");
    }
}
