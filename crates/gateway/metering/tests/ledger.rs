//! ledger 测试 — 验证预扣/结算/释放。

use metering::ledger::{BalanceLedger, Insufficient, Ledger, MemoryLedger};

#[test]
fn ledger_prehold_succeeds_when_balance_sufficient() {
    let ledger = MemoryLedger::new();
    ledger.set_balance("user1", "tok1", 1000);

    let hold = ledger.prehold("user1", "tok1", 500).unwrap();
    assert_eq!(hold.amount, 500);
    assert_eq!(hold.user_key, "user1");
    assert_eq!(hold.token_key, "tok1");

    // 余额减少
    assert_eq!(ledger.available("user1", "tok1"), 500);
}

#[test]
fn ledger_prehold_fails_when_insufficient() {
    let ledger = MemoryLedger::new();
    ledger.set_balance("user1", "tok1", 100);

    let err = ledger.prehold("user1", "tok1", 500).unwrap_err();
    assert_eq!(err.need, 500);
    assert_eq!(err.have, 100);
}

#[test]
fn ledger_settle_refunds_difference() {
    let ledger = MemoryLedger::new();
    ledger.set_balance("user1", "tok1", 1000);

    let hold = ledger.prehold("user1", "tok1", 500).unwrap();
    // 实际只用了 300, 退回 200
    let diff = ledger.settle(&hold, 300);
    assert_eq!(diff, 200);
    assert_eq!(ledger.available("user1", "tok1"), 700);
}

#[test]
fn ledger_settle_charges_extra_when_actual_exceeds_estimate() {
    let ledger = MemoryLedger::new();
    ledger.set_balance("user1", "tok1", 1000);

    let hold = ledger.prehold("user1", "tok1", 500).unwrap();
    // 实际用了 700, 补扣 200
    let diff = ledger.settle(&hold, 700);
    assert_eq!(diff, -200);
    assert_eq!(ledger.available("user1", "tok1"), 300);
}

#[test]
fn ledger_release_restores_full_amount() {
    let ledger = MemoryLedger::new();
    ledger.set_balance("user1", "tok1", 1000);

    let hold = ledger.prehold("user1", "tok1", 500).unwrap();
    ledger.release(&hold);
    assert_eq!(ledger.available("user1", "tok1"), 1000);
}

#[test]
fn ledger_available_default_zero() {
    let ledger = MemoryLedger::new();
    assert_eq!(ledger.available("user1", "tok1"), 0);
}

#[test]
fn ledger_multiple_users_isolated() {
    let ledger = MemoryLedger::new();
    ledger.set_balance("user1", "tok1", 1000);
    ledger.set_balance("user2", "tok1", 500);

    let hold = ledger.prehold("user1", "tok1", 300).unwrap();
    assert_eq!(ledger.available("user1", "tok1"), 700);
    assert_eq!(ledger.available("user2", "tok1"), 500);

    ledger.settle(&hold, 200);
    assert_eq!(ledger.available("user1", "tok1"), 800);
    assert_eq!(ledger.available("user2", "tok1"), 500);
}
