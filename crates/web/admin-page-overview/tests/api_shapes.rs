//! Dashboard section 集成测试:api.rs 三个子模块的数据形状不变量。
//! leaderboard 的 data.rs 含 asset!() 立绘字段, 随 MODELS 一起编译; asset! 是
//! 编译期宏, 在 cargo test 下同样展开, 无需浏览器上下文。

mod overview {
    use admin_page_overview::api::overview as api;

    #[test]
    fn stats_pairs_complete() {
        let stats = api::fetch_stats();
        assert!(!stats.is_empty(), "统计卡非空");
        for (value, label) in stats {
            assert!(!value.is_empty(), "统计卡值非空");
            assert!(!label.is_empty(), "统计卡标签非空");
        }
    }

    #[test]
    fn users_shares_sum_to_reasonable() {
        let users = api::fetch_users();
        assert!(!users.is_empty(), "用户分布非空");
        // (名称, 消耗量, 占比%): 占比 0..=100
        for (name, _usage, pct) in users {
            assert!(!name.is_empty(), "用户名非空");
            assert!((0.0..=100.0).contains(pct), "用户 {name} 占比 {pct} 超界");
        }
    }

    #[test]
    fn models_shares_sum_to_reasonable() {
        let models = api::fetch_models();
        assert!(!models.is_empty(), "模型分布非空");
        for (name, _usage, pct) in models {
            assert!(!name.is_empty(), "模型名非空");
            assert!((0.0..=100.0).contains(pct), "模型 {name} 占比 {pct} 超界");
        }
    }
}

mod models {
    use admin_page_overview::api::models as api;

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
}

mod leaderboard {
    use admin_page_overview::api::leaderboard::{
        DIMS, MODELS, avg_norms, composite, dim_rank, norms,
    };

    #[test]
    fn dims_shape() {
        assert_eq!(DIMS.len(), 6, "六维雷达");
    }

    #[test]
    fn norms_in_unit_interval() {
        for m in MODELS {
            for (i, v) in norms(m).iter().enumerate() {
                assert!(
                    (0.0..=1.0).contains(v),
                    "模型 {} 维度 {} 归一值 {v} 越界 [0,1]",
                    m.name,
                    DIMS[i]
                );
            }
        }
    }

    #[test]
    fn composite_in_0_100() {
        for m in MODELS {
            let c = composite(m);
            assert!(
                (0.0..=100.0).contains(&c),
                "模型 {} 综合分 {c} 越界 [0,100]",
                m.name
            );
        }
    }

    #[test]
    fn dim_rank_is_permutation() {
        // 每个维度上, 全体模型的名次应是 1..=N 的一个排列 (并列时会有重复,
        // 但最小名次必为 1, 最大不超过 N)
        let n = MODELS.len();
        for (i, dim) in DIMS.iter().enumerate() {
            let ranks: Vec<usize> = MODELS.iter().map(|m| dim_rank(m)[i]).collect();
            assert!(ranks.iter().min() == Some(&1), "维度 {dim} 最小名次应为 1");
            assert!(
                ranks.iter().max().unwrap() <= &n,
                "维度 {dim} 最大名次不超过 {n}"
            );
        }
    }

    #[test]
    fn avg_norms_in_unit_interval() {
        let avg = avg_norms();
        for (i, v) in avg.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(v),
                "平均归一 维度 {} = {v} 越界",
                DIMS[i]
            );
        }
    }
}
