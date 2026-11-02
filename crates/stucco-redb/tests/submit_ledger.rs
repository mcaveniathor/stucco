#![cfg(feature = "submit-ledger")]
use std::sync::{Arc, Barrier};
use stucco_redb::{PostcardCodec, Store, StoreError, SubmitLedger, Table, U64Key};

fn open(dir: &tempfile::TempDir) -> (Store, SubmitLedger<u64>, Table<u64, String>) {
    let store = Store::open(dir.path().join("ledger.redb")).unwrap();
    let ledger = SubmitLedger::open(&store, "submits").unwrap();
    let orders = store
        .table("orders", U64Key, PostcardCodec::default())
        .unwrap();
    (store, ledger, orders)
}

/// The usual shape: replay a known key, otherwise apply and record.
fn submit(
    store: &Store,
    ledger: &SubmitLedger<u64>,
    orders: &Table<u64, String>,
    key: &str,
    now: u64,
) -> u64 {
    store
        .write(|tx| {
            if let Some(first) = ledger.replay(tx, key, now)? {
                return Ok::<_, StoreError>(first);
            }
            let (id, _) = tx.create(orders, |id| format!("order {id}"))?;
            ledger.record(tx, key, now + 60, id)?;
            Ok(id)
        })
        .unwrap()
}

#[test]
fn a_repeated_key_replays_the_first_outcome() {
    let dir = tempfile::tempdir().unwrap();
    let (store, ledger, orders) = open(&dir);
    assert_eq!(submit(&store, &ledger, &orders, "a", 100), 1);
    assert_eq!(submit(&store, &ledger, &orders, "a", 101), 1);
    assert_eq!(submit(&store, &ledger, &orders, "b", 101), 2);
    assert_eq!(orders.get(&3).unwrap(), None);
}

#[test]
fn an_expired_entry_no_longer_replays_and_can_be_recorded_again() {
    let dir = tempfile::tempdir().unwrap();
    let (store, ledger, orders) = open(&dir);
    assert_eq!(submit(&store, &ledger, &orders, "a", 100), 1);
    assert_eq!(submit(&store, &ledger, &orders, "a", 159), 1);
    assert_eq!(submit(&store, &ledger, &orders, "a", 160), 2);
    assert_eq!(submit(&store, &ledger, &orders, "a", 161), 2);
}

#[test]
fn a_rolled_back_effect_leaves_no_record() {
    let dir = tempfile::tempdir().unwrap();
    let (store, ledger, orders) = open(&dir);
    let failed: Result<(), StoreError> = store.write(|tx| {
        ledger.record(tx, "a", 200, 9)?;
        Err(StoreError::new(std::io::Error::other("effect failed")))
    });
    assert!(failed.is_err());
    assert_eq!(submit(&store, &ledger, &orders, "a", 100), 1);
}

#[test]
fn racing_submissions_apply_once() {
    let dir = tempfile::tempdir().unwrap();
    let (store, ledger, orders) = open(&dir);
    let start = Arc::new(Barrier::new(8));
    let ids: Vec<_> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let (store, ledger, orders, start) = (&store, &ledger, &orders, start.clone());
                scope.spawn(move || {
                    start.wait();
                    submit(store, ledger, orders, "a", 100)
                })
            })
            .collect();
        workers.into_iter().map(|w| w.join().unwrap()).collect()
    });
    assert!(ids.iter().all(|id| *id == 1));
    assert_eq!(orders.get(&2).unwrap(), None);
}

#[test]
fn purge_removes_only_expired_entries_up_to_the_limit() {
    let dir = tempfile::tempdir().unwrap();
    let (store, ledger, _) = open(&dir);
    store
        .write(|tx| {
            ledger.record(tx, "soon", 100, 1)?;
            ledger.record(tx, "next", 110, 2)?;
            ledger.record(tx, "later", 200, 3)?;
            Ok::<_, StoreError>(())
        })
        .unwrap();
    assert_eq!(ledger.purge_expired(99, 10).unwrap(), 0);
    assert_eq!(ledger.purge_expired(150, 1).unwrap(), 1);
    assert_eq!(ledger.purge_expired(150, 10).unwrap(), 1);
    assert_eq!(ledger.purge_expired(150, 10).unwrap(), 0);
    store
        .write(|tx| {
            assert_eq!(ledger.replay(tx, "later", 150)?, Some(3));
            Ok::<_, StoreError>(())
        })
        .unwrap();
}
