//! MemoryLedger 并发正确性测试（Shuttle 随机调度探索线程交错）。
//!
//! 靶子是全项目审查报告 M2（`todo/code-review-2026-09-17-report.md`）：
//! prehold/settle/release 原实现各自跨两把锁（balances 先改、holds 后改），
//! 且 settle/release 不防重复退额。
//!
//! 可观察不变式（刻意只用**无账本延迟**的判据，避免测试自身记账滞后造成假失败）：
//!   1. 期间任何线程任何时刻：`available <= INITIAL` —— 重复退额会超出初始值；
//!   2. join 后无并发：`available == INITIAL - (issued - refunded_once) * EST`。
//!
//! settle 后再 release 的顺序竞态由 `settle_then_release_does_not_refund`
//! 覆盖（并发期「哪些 hold 已结算」对外部不可观察，精确结算守恒只能留到
//! join 后；故并发测试聚焦 release 退额路径）。
//!
//! 运行：`cargo test -p metering --features shuttle`
#![cfg(feature = "shuttle")]

use std::collections::HashSet;

use shuttle::sync::atomic::{AtomicU64, Ordering};
use shuttle::sync::{Arc, Mutex};
use shuttle::thread;

use metering::ledger::{BalanceLedger, Ledger, MemoryLedger};

const USER: &str = "u";
const TOKEN: &str = "t";
const EST: i64 = 100;
const INITIAL: i64 = 1_000_000;

/// 文档声称 release「幂等」（请求失败 → 全额退，可重复调用）。
/// 双锁实现下第二次 release 仍退 EST → 凭空生钱。
#[test]
fn release_is_idempotent_no_money_created() {
    let ledger = MemoryLedger::new();
    ledger.set_balance(USER, TOKEN, INITIAL);
    let hold = ledger.prehold(USER, TOKEN, EST).unwrap();
    assert_eq!(ledger.available(USER, TOKEN), INITIAL - EST);

    ledger.release(&hold); // 首次退额：合理
    assert_eq!(ledger.available(USER, TOKEN), INITIAL);

    ledger.release(&hold); // 重复退额：必须幂等，不能回升到 INITIAL + EST
    assert_eq!(
        ledger.available(USER, TOKEN),
        INITIAL,
        "release 不幂等：重复退额凭空生钱"
    );
}

/// settle 消费 actual 后，同一 hold 再 release 不应回补（hold 已结算）。
#[test]
fn settle_then_release_does_not_refund() {
    let ledger = MemoryLedger::new();
    ledger.set_balance(USER, TOKEN, INITIAL);
    let hold = ledger.prehold(USER, TOKEN, EST).unwrap();

    ledger.settle(&hold, EST); // actual == 预估 → 消费 EST
    assert_eq!(ledger.available(USER, TOKEN), INITIAL - EST);

    ledger.release(&hold); // 已结算的 hold：release 必须 no-op
    assert_eq!(
        ledger.available(USER, TOKEN),
        INITIAL - EST,
        "settle 后再 release 重复退额"
    );
}

/// 并发交错下：每个 hold 的退额恰好一次，任何线程都看不到余额超过初始值。
///
/// 消费者循环 release 同一 hold 再放回池子，制造跨线程的重复 release；
/// Shuttle 随机调度探索「release 与 release 落到同一 hold」的交错。
#[test]
fn concurrent_release_refunds_exactly_once() {
    shuttle::check_random(
        || {
            let ledger = Arc::new(MemoryLedger::new());
            ledger.set_balance(USER, TOKEN, INITIAL);
            let pool: Arc<Mutex<Vec<metering::ledger::Hold>>> = Arc::new(Mutex::new(Vec::new()));
            let released: Arc<Mutex<HashSet<u64>>> = Arc::new(Mutex::new(HashSet::new()));
            let issued = Arc::new(AtomicU64::new(0));

            // 生产者：prehold 成功后入池。
            let mut producers = Vec::new();
            for _ in 0..3 {
                let (ledger, pool, issued) = (ledger.clone(), pool.clone(), issued.clone());
                producers.push(thread::spawn(move || {
                    for _ in 0..4 {
                        if let Ok(hold) = ledger.prehold(USER, TOKEN, EST) {
                            issued.fetch_add(1, Ordering::SeqCst);
                            // 期间断言（不依赖任何测试侧账本，无延迟）：
                            // 扣减只会让余额下降，绝不能超过初始值。
                            assert!(ledger.available(USER, TOKEN) <= INITIAL, "运行期间凭空生钱");
                            pool.lock().unwrap().push(hold);
                        }
                    }
                }));
            }

            // 消费者：pop → release → push。同一 hold 会被多个消费者反复取用，
            // 重复 release 在旧实现下每次都退 EST。
            let mut consumers = Vec::new();
            for _ in 0..3 {
                let (ledger, pool, released) = (ledger.clone(), pool.clone(), released.clone());
                consumers.push(thread::spawn(move || {
                    for _ in 0..10 {
                        let hold = { pool.lock().unwrap().pop() };
                        if let Some(hold) = hold {
                            ledger.release(&hold);
                            released.lock().unwrap().insert(hold.id);
                            assert!(
                                ledger.available(USER, TOKEN) <= INITIAL,
                                "重复 release 凭空生钱"
                            );
                            pool.lock().unwrap().push(hold);
                        }
                    }
                }));
            }

            for t in producers {
                t.join().unwrap();
            }
            for t in consumers {
                t.join().unwrap();
            }

            // join 后无并发：issued / released 都是终值，无记账延迟。
            // 保持扣减态。available 必须恰好等于 INITIAL - 未退额部分。
            let issued_n = issued.load(Ordering::SeqCst) as i64;
            let released_n = released.lock().unwrap().len() as i64;
            let expected = INITIAL - (issued_n - released_n) * EST;
            assert_eq!(
                ledger.available(USER, TOKEN),
                expected,
                "最终资金不守恒：多退或少扣"
            );
        },
        1000,
    );
}
