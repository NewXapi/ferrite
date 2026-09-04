//! 数据模型与默认种子形状测试
use tavern_page_characters::{Character, seed_all_characters};

#[test]
fn seed_characters_non_empty_and_valid() {
    let characters = seed_all_characters();
    assert!(!characters.is_empty(), "种子角色数据必须非空");
    for c in &characters {
        assert!(!c.name.is_empty(), "角色名不可为空");
        assert!(!c.sub_title.is_empty(), "副标题不可为空");
        assert!(!c.tags.is_empty(), "标签列表不可为空");
        assert!(c.rating >= 0.0 && c.rating <= 10.0, "评分在0-10之间");
        assert!(!c.default_model.is_empty(), "默认推荐模型必须配置");
    }
}

#[test]
fn character_empty_factory_defaults() {
    let empty_char = Character::empty(999);
    assert_eq!(empty_char.id, 999, "id 必须由入参确定");
    assert!(empty_char.name.is_empty(), "空角色名为空字符串");
    assert!(empty_char.is_user_created, "工厂方法产出为用户自建角色");
    assert!(!empty_char.is_published, "新建角色默认为草稿模式");
}
