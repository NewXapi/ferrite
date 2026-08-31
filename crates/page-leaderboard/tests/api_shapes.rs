//! leaderboard 集成测试: 六维归一化/综合分/名次的数学不变量。
//! data.rs 含 asset!() 立绘字段, 随 api::MODELS 一起编译; asset! 是
//! 编译期宏, 在 cargo test 下同样展开, 无需浏览器上下文。

use page_leaderboard::api::{DIMS, MODELS, avg_norms, composite, dim_rank, norms};

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
    for i in 0..6 {
        let ranks: Vec<usize> = MODELS.iter().map(|m| dim_rank(m)[i]).collect();
        assert!(
            ranks.iter().min() == Some(&1),
            "维度 {} 最小名次应为 1",
            DIMS[i]
        );
        assert!(
            ranks.iter().max().unwrap() <= &n,
            "维度 {} 最大名次不超过 {n}",
            DIMS[i]
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
