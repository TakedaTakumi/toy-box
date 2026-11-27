use std::cell::Cell;

use gregory_editor::memo::Memo;
use proptest::prelude::*;

#[test]
fn initial_state() {
    let m: Memo<u8, u8> = Memo::default();
    assert_eq!(m.value(), None);
    assert_eq!(m.revision(), 0);
}

#[test]
fn same_key_does_not_recompute_and_new_key_computes_once() {
    let calls = Cell::new(0);
    let mut m = Memo::default();
    let mut get = |k: i32| {
        *m.get_or_compute(k, |k| {
            calls.set(calls.get() + 1);
            k * 10
        })
    };
    assert_eq!(get(1), 10);
    assert_eq!(get(1), 10);
    assert_eq!(calls.get(), 1);
    assert_eq!(get(2), 20);
    assert_eq!(calls.get(), 2);
    assert_eq!(get(1), 10);
    assert_eq!(calls.get(), 3);
    assert_eq!(m.revision(), 3);
    assert_eq!(m.value(), Some(&10));
}

proptest! {
    #[test]
    fn revision_counts_key_changes(keys in proptest::collection::vec(0u8..4, 0..60)) {
        let calls = Cell::new(0u64);
        let mut m = Memo::default();
        for &k in &keys {
            let v = *m.get_or_compute(k, |k| { calls.set(calls.get() + 1); u32::from(*k) * 7 });
            prop_assert_eq!(v, u32::from(k) * 7);
        }
        let changes = keys.windows(2).filter(|w| w[0] != w[1]).count() as u64;
        let expected = if keys.is_empty() { 0 } else { changes + 1 };
        prop_assert_eq!(m.revision(), expected);
        prop_assert_eq!(calls.get(), expected);
        prop_assert_eq!(m.value().copied(), keys.last().map(|&k| u32::from(k) * 7));
    }
}
