//! 卡片分页边界：过滤缩少结果后保留有效页，不能把已有记录藏进空白页。

use ui_components::{page_count, page_slice};

#[test]
fn clamps_visible_records_after_filtering() {
    let records: Vec<_> = (0..38).collect();
    assert_eq!(page_count(records.len(), 15), 3);
    assert_eq!(page_slice(&records, 2, 15), &records[30..]);

    // 用户停在第三页时筛到十六条，必须展示第二页剩余的一条。
    let filtered = &records[..16];
    assert_eq!(page_count(filtered.len(), 15), 2);
    assert_eq!(page_slice(filtered, 2, 15), &[15]);

    // 精确一页与空结果不残留旧页数据，非法页大小也不能除零。
    assert_eq!(page_count(15, 15), 1);
    assert_eq!(page_slice(&records[..15], 2, 15), &records[..15]);
    assert!(page_slice::<usize>(&[], 2, 15).is_empty());
    assert!(page_slice(&records, 2, 0).is_empty());
}
