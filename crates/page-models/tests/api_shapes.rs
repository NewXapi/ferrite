//! models page 集成测试:api.rs 薄壳的数据形状不变量。

use page_models::api;

#[test]
fn models_have_unique_names() {
    let models = api::fetch_models();
    assert!(!models.is_empty(), "至少一个模型");
    let mut names: Vec<_> = models.iter().map(|m| m.name).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), models.len(), "模型名唯一");
}

#[test]
fn models_fields_complete() {
    for m in api::fetch_models() {
        assert!(!m.name.is_empty(), "模型 {} 名字非空", m.name);
        assert!(!m.vendor.is_empty(), "模型 {} vendor 非空", m.name);
        assert!(!m.description.is_empty(), "模型 {} 描述非空", m.name);
        // 价格三件套是展示字符串, 非空即可
        assert!(!m.price_input.is_empty(), "模型 {} 输入价非空", m.name);
        assert!(!m.price_output.is_empty(), "模型 {} 输出价非空", m.name);
    }
}

#[test]
fn models_groups_reference_valid() {
    // GroupPrice { name, input, output, cache }: 每行非空即可
    for m in api::fetch_models() {
        for gp in m.groups.iter() {
            assert!(!gp.name.is_empty(), "模型 {} 分组行名非空", m.name);
        }
    }
}
