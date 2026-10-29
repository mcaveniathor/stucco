#![cfg(feature = "tokio")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use stucco_redb::{
    AsyncStore, AsyncTable, IndexTable, PostcardCodec, Store, StoreError, Table, U64Key,
};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tokio::time::timeout;

type Orders = IndexTable<u64, String>;
type Audit = Table<u64, String>;

fn boom() -> StoreError {
    StoreError::new(std::io::Error::other("boom"))
}

fn open(dir: &tempfile::TempDir) -> (Store, Orders, Audit) {
    let store = Store::open(dir.path().join("async-tx.redb")).unwrap();
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

/// A closure that reports it started, then blocks until the gate opens.
#[derive(Clone, Default)]
struct Gate(Arc<(Mutex<bool>, Condvar)>);

impl Gate {
    fn wait(&self) {
        let (mutex, signal) = &*self.0;
        let mut open = mutex.lock().unwrap();
        while !*open {
            open = signal.wait(open).unwrap();
        }
    }
    fn open(&self) {
        *self.0.0.lock().unwrap() = true;
        self.0.1.notify_all();
    }
}

async fn started(events: &mut UnboundedReceiver<()>) {
    timeout(Duration::from_secs(5), events.recv())
        .await
        .unwrap()
        .unwrap();
}

async fn is_blocked<T>(future: impl std::future::Future<Output = T>) -> bool {
    timeout(Duration::from_millis(100), future).await.is_err()
}

async fn second_transaction_waits_for_the_first(store: AsyncStore) {
    let gate = Gate::default();
    let (entered, mut events) = unbounded_channel();
    let first = {
        let (store, gate) = (store.clone(), gate.clone());
        tokio::spawn(async move {
            store
                .write(move |_| {
                    entered.send(()).unwrap();
                    gate.wait();
                    Ok::<_, StoreError>(())
                })
                .await
        })
    };
    started(&mut events).await;
    assert!(
        is_blocked(store.write(|_| Ok::<_, StoreError>(()))).await,
        "a running transaction must hold the only permit"
    );
    gate.open();
    first.await.unwrap().unwrap();
    timeout(
        Duration::from_secs(5),
        store.write(|_| Ok::<_, StoreError>(())),
    )
    .await
    .expect("the permit must come back")
    .unwrap();
}

#[tokio::test]
async fn async_transaction_commits_across_tables() {
    let dir = tempfile::tempdir().unwrap();
    let (store, orders, audit) = open(&dir);
    let store = AsyncStore::new(store);
    let (o, a) = (orders.clone(), audit.clone());
    store
        .write(move |tx| {
            tx.put(&o, &1, &"Ada".to_string())?;
            tx.put(&a, &1, &"created".to_string())?;
            Ok::<_, StoreError>(())
        })
        .await
        .unwrap();
    let seen = store
        .read(move |tx| Ok::<_, StoreError>((tx.get(&orders, &1)?, tx.get(&audit, &1)?)))
        .await
        .unwrap();
    assert_eq!(seen, (Some("Ada".into()), Some("created".into())));
}

#[tokio::test]
async fn async_transaction_rolls_back_on_error() {
    let dir = tempfile::tempdir().unwrap();
    let (store, orders, audit) = open(&dir);
    let store = AsyncStore::new(store);
    let (o, a) = (orders.clone(), audit.clone());
    let result = store
        .write(move |tx| {
            tx.put(&o, &1, &"Ada".to_string())?;
            tx.put(&a, &1, &"created".to_string())?;
            Err::<(), _>(boom())
        })
        .await;
    assert!(result.is_err());
    assert_eq!(orders.get(&1).unwrap(), None);
    assert_eq!(audit.get(&1).unwrap(), None);
}

#[tokio::test]
async fn a_transaction_holds_one_permit_until_it_finishes() {
    let dir = tempfile::tempdir().unwrap();
    let (store, _, _) = open(&dir);
    second_transaction_waits_for_the_first(AsyncStore::new(store).concurrency(1)).await;
}

#[tokio::test]
async fn concurrency_zero_means_one() {
    let dir = tempfile::tempdir().unwrap();
    let (store, _, _) = open(&dir);
    second_transaction_waits_for_the_first(AsyncStore::new(store).concurrency(0)).await;
}

#[tokio::test]
async fn async_table_calls_wait_for_a_running_transaction() {
    let dir = tempfile::tempdir().unwrap();
    let (store, orders, _) = open(&dir);
    let store = AsyncStore::new(store).concurrency(1);
    let table = store.table(orders);
    let gate = Gate::default();
    let (entered, mut events) = unbounded_channel();
    let running = {
        let (store, gate) = (store.clone(), gate.clone());
        tokio::spawn(async move {
            store
                .write(move |_| {
                    entered.send(()).unwrap();
                    gate.wait();
                    Ok::<_, StoreError>(())
                })
                .await
        })
    };
    started(&mut events).await;
    assert!(is_blocked(table.get(1)).await);
    gate.open();
    running.await.unwrap().unwrap();
    assert_eq!(table.get(1).await.unwrap(), None);
}

#[tokio::test]
async fn a_running_async_table_call_blocks_transactions_from_its_store() {
    let dir = tempfile::tempdir().unwrap();
    let (_, orders, _) = open(&dir);
    let table: AsyncTable<u64, String> = AsyncTable::new(orders).concurrency(1);
    let (_, revision) = table.create(|_| "Ada".to_string()).await.unwrap();
    let store = table.store();
    let gate = Gate::default();
    let (entered, mut events) = unbounded_channel();
    let running = {
        let (table, gate) = (table.clone(), gate.clone());
        tokio::spawn(async move {
            table
                .update_if(1, revision, move |v| {
                    entered.send(()).unwrap();
                    gate.wait();
                    v
                })
                .await
        })
    };
    started(&mut events).await;
    assert!(is_blocked(store.write(|_| Ok::<_, StoreError>(()))).await);
    gate.open();
    running.await.unwrap().unwrap();
    store.write(|_| Ok::<_, StoreError>(())).await.unwrap();
}

#[tokio::test]
async fn dropping_a_queued_transaction_runs_nothing_and_keeps_the_budget() {
    let dir = tempfile::tempdir().unwrap();
    let (store, _, _) = open(&dir);
    let store = AsyncStore::new(store).concurrency(1);
    let gate = Gate::default();
    let (entered, mut events) = unbounded_channel();
    let running = {
        let (store, gate) = (store.clone(), gate.clone());
        tokio::spawn(async move {
            store
                .write(move |_| {
                    entered.send(()).unwrap();
                    gate.wait();
                    Ok::<_, StoreError>(())
                })
                .await
        })
    };
    started(&mut events).await;
    let ran = Arc::new(AtomicBool::new(false));
    let queued = {
        let (store, ran) = (store.clone(), ran.clone());
        tokio::spawn(async move {
            store
                .write(move |_| {
                    ran.store(true, Ordering::SeqCst);
                    Ok::<_, StoreError>(())
                })
                .await
        })
    };
    tokio::time::sleep(Duration::from_millis(50)).await;
    queued.abort();
    assert!(queued.await.unwrap_err().is_cancelled());
    gate.open();
    running.await.unwrap().unwrap();
    assert!(!ran.load(Ordering::SeqCst));
    timeout(
        Duration::from_secs(5),
        store.write(|_| Ok::<_, StoreError>(())),
    )
    .await
    .expect("a dropped queued call must not leak its permit")
    .unwrap();
}

fn dropped_transaction_queued_in_a_saturated_blocking_pool_runs_nothing(read: bool) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async move {
        let dir = tempfile::tempdir().unwrap();
        let (store, _, _) = open(&dir);
        let store = AsyncStore::new(store);
        let gate = Gate::default();
        let (entered, mut events) = unbounded_channel();
        let occupied = {
            let gate = gate.clone();
            tokio::task::spawn_blocking(move || {
                entered.send(()).unwrap();
                gate.wait();
            })
        };
        started(&mut events).await;
        let ran = Arc::new(AtomicBool::new(false));
        let queued = {
            let (store, ran) = (store.clone(), ran.clone());
            tokio::spawn(async move {
                if read {
                    store
                        .read(move |_| {
                            ran.store(true, Ordering::SeqCst);
                            Ok::<_, StoreError>(())
                        })
                        .await
                } else {
                    store
                        .write(move |_| {
                            ran.store(true, Ordering::SeqCst);
                            Ok::<_, StoreError>(())
                        })
                        .await
                }
            })
        };
        // The call now holds its permit and sits in the blocking queue.
        tokio::time::sleep(Duration::from_millis(100)).await;
        queued.abort();
        assert!(queued.await.unwrap_err().is_cancelled());
        gate.open();
        occupied.await.unwrap();
        // The blocking queue is FIFO: once this finishes, anything queued
        // earlier has had its chance to run.
        store.write(|_| Ok::<_, StoreError>(())).await.unwrap();
        assert!(
            !ran.load(Ordering::SeqCst),
            "a dropped call must not run once it reaches the front of the queue"
        );
    });
}

#[test]
fn dropped_write_queued_in_a_saturated_blocking_pool_runs_nothing() {
    dropped_transaction_queued_in_a_saturated_blocking_pool_runs_nothing(false);
}

#[test]
fn dropped_read_queued_in_a_saturated_blocking_pool_runs_nothing() {
    dropped_transaction_queued_in_a_saturated_blocking_pool_runs_nothing(true);
}
