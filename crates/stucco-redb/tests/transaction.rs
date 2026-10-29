use std::panic::{AssertUnwindSafe, catch_unwind};
use stucco_redb::{Codec, IndexTable, PostcardCodec, Store, StoreError, Table, U64Key};

fn boom() -> StoreError {
    StoreError::new(std::io::Error::other("boom"))
}

type Orders = IndexTable<u64, String>;
type Audit = Table<u64, String>;

fn open(dir: &tempfile::TempDir) -> (Store, Orders, Audit) {
    let store = Store::open(dir.path().join("tx.redb")).unwrap();
    let orders = store
        .table::<u64, String, _, _>("orders", U64Key, PostcardCodec::default())
        .unwrap()
        .index("name", |v: &String| v.as_bytes().to_vec())
        .unwrap();
    let audit = store
        .table::<u64, String, _, _>("audit", U64Key, PostcardCodec::default())
        .unwrap();
    (store, orders, audit)
}

fn indexed(table: &Orders) -> Vec<String> {
    table
        .all(true)
        .unwrap()
        .into_iter()
        .map(|(_, v)| v)
        .collect()
}

/// Refuses to encode the value "bad".
struct Rejecting;
impl Codec<String> for Rejecting {
    fn encode(&self, value: &String) -> Result<Vec<u8>, StoreError> {
        if value == "bad" {
            Err(boom())
        } else {
            Ok(value.as_bytes().to_vec())
        }
    }
    fn decode(&self, bytes: &[u8]) -> Result<String, StoreError> {
        String::from_utf8(bytes.to_vec()).map_err(StoreError::new)
    }
}

#[test]
fn write_commits_across_tables_and_index() -> Result<(), StoreError> {
    let dir = tempfile::tempdir().unwrap();
    let (store, orders, audit) = open(&dir);
    store.write(|tx| {
        tx.put(&orders, &1, &"Ada".to_string())?;
        tx.put(&audit, &1, &"created".to_string())?;
        Ok::<_, StoreError>(())
    })?;
    assert_eq!(orders.get(&1)?, Some("Ada".into()));
    assert_eq!(audit.get(&1)?, Some("created".into()));
    assert_eq!(indexed(&orders), ["Ada"]);
    Ok(())
}

#[test]
fn error_rolls_back_every_table_and_index() {
    let dir = tempfile::tempdir().unwrap();
    let (store, orders, audit) = open(&dir);
    let result = store.write(|tx| {
        tx.put(&orders, &1, &"Ada".to_string())?;
        tx.put(&audit, &1, &"created".to_string())?;
        Err::<(), _>(boom())
    });
    assert!(result.is_err());
    assert_eq!(orders.get(&1).unwrap(), None);
    assert_eq!(audit.get(&1).unwrap(), None);
    assert!(orders.all(true).unwrap().is_empty());
}

#[test]
fn custom_error_type_passes_through() {
    #[derive(Debug)]
    enum AppError {
        Boom,
        #[allow(dead_code)]
        Store(StoreError),
    }
    impl From<StoreError> for AppError {
        fn from(error: StoreError) -> Self {
            AppError::Store(error)
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let (store, orders, _) = open(&dir);
    let result = store.write(|tx| {
        tx.put(&orders, &1, &"Ada".to_string())?;
        Err::<(), _>(AppError::Boom)
    });
    assert!(matches!(result, Err(AppError::Boom)));
    assert_eq!(orders.get(&1).unwrap(), None);
}

#[test]
fn panic_rolls_back_and_store_stays_usable() {
    let dir = tempfile::tempdir().unwrap();
    let (store, orders, audit) = open(&dir);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = store.write(|tx| {
            tx.put(&orders, &1, &"Ada".to_string())?;
            if true {
                panic!("boom");
            }
            Ok::<_, StoreError>(())
        });
    }));
    assert!(result.is_err());
    assert_eq!(orders.get(&1).unwrap(), None);
    store
        .write(|tx| tx.put(&audit, &2, &"after".to_string()))
        .unwrap();
    drop((store, orders, audit));
    let (_, orders, audit) = open(&dir);
    assert_eq!(orders.get(&1).unwrap(), None);
    assert_eq!(audit.get(&2).unwrap(), Some("after".into()));
}

#[test]
fn codec_failure_midway_rolls_back_earlier_writes() {
    let dir = tempfile::tempdir().unwrap();
    let (store, orders, _) = open(&dir);
    let strict = store
        .table::<u64, String, _, _>("strict", U64Key, Rejecting)
        .unwrap();
    let result = store.write(|tx| {
        tx.put(&orders, &1, &"Ada".to_string())?;
        tx.put(&strict, &1, &"bad".to_string())?;
        Ok::<_, StoreError>(())
    });
    assert!(result.is_err());
    assert_eq!(orders.get(&1).unwrap(), None);
    assert!(orders.all(true).unwrap().is_empty());
}

#[test]
fn write_lock_is_released_after_failure() {
    let dir = tempfile::tempdir().unwrap();
    let (store, orders, _) = open(&dir);
    let _ = store.write(|tx| {
        tx.put(&orders, &1, &"Ada".to_string())?;
        Err::<(), _>(boom())
    });
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _ = store.write(|_| -> Result<(), StoreError> { panic!("boom") });
    }));
    let (done, finished) = std::sync::mpsc::channel();
    let (store, orders) = (store.clone(), orders.clone());
    std::thread::spawn(move || {
        let result = store.write(|tx| tx.put(&orders, &2, &"Grace".to_string()));
        done.send(result.is_ok()).unwrap();
    });
    assert!(
        finished
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap()
    );
}

#[test]
fn empty_transaction_commits_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (store, orders, audit) = open(&dir);
    store.write(|_| Ok::<_, StoreError>(())).unwrap();
    assert!(orders.all(true).unwrap().is_empty());
    assert_eq!(audit.get(&1).unwrap(), None);
}

#[test]
fn handle_from_another_store_is_rejected() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let (store_a, _, _) = open(&first);
    let (_, _, audit_b) = open(&second);
    let result = store_a.write(|tx| tx.put(&audit_b, &1, &"x".to_string()));
    assert!(result.is_err());
    assert_eq!(audit_b.get(&1).unwrap(), None);
}

#[test]
fn reads_see_the_transactions_own_writes() -> Result<(), StoreError> {
    let dir = tempfile::tempdir().unwrap();
    let (store, orders, _) = open(&dir);
    store.write(|tx| {
        assert!(!tx.remove(&orders, &1)?);
        tx.put(&orders, &1, &"Ada".to_string())?;
        assert_eq!(tx.get(&orders, &1)?, Some("Ada".into()));
        assert!(tx.remove(&orders, &1)?);
        assert_eq!(tx.get(&orders, &1)?, None);
        Ok::<_, StoreError>(())
    })?;
    assert!(orders.all(true)?.is_empty());
    Ok(())
}

#[test]
fn read_snapshot_is_stable_during_a_concurrent_write() -> Result<(), StoreError> {
    let dir = tempfile::tempdir().unwrap();
    let (store, orders, _) = open(&dir);
    orders.put(&1, &"Ada".into())?;
    store.read(|tx| {
        let before = tx.get(&orders, &1)?;
        orders.put(&1, &"Grace".into())?;
        assert_eq!(tx.get(&orders, &1)?, before);
        Ok::<_, StoreError>(())
    })?;
    assert_eq!(orders.get(&1)?, Some("Grace".into()));
    Ok(())
}

#[test]
fn concurrent_writers_serialize() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("counter.redb")).unwrap();
    let counter = store
        .table::<u64, u64, _, _>("counter", U64Key, PostcardCodec::default())
        .unwrap();
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let (store, counter) = (store.clone(), counter.clone());
            std::thread::spawn(move || {
                for _ in 0..25 {
                    store
                        .write(|tx| {
                            let n = tx.get(&counter, &0)?.unwrap_or(0);
                            tx.put(&counter, &0, &(n + 1))
                        })
                        .unwrap();
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(counter.get(&0).unwrap(), Some(200));
}
