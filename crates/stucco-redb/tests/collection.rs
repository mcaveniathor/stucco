#![cfg(feature = "tokio")]
use stucco_core::{Capabilities, CollectionQuery, ColumnSpec, Direction, Window};
use stucco_redb::{
    AsyncTable, CollectionMapping, PostcardCodec, RedbCollection, ScanRequest, Store, U64Key,
};
use stucco_tower::{CollectionSource, RequestContext};
struct Mapping(std::thread::ThreadId);
impl CollectionMapping<String> for Mapping {
    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
    }
    fn columns(&self) -> Vec<ColumnSpec> {
        vec![]
    }
    fn scan_request(&self, q: &CollectionQuery) -> ScanRequest {
        ScanRequest {
            index: None,
            direction: Direction::Asc,
            window: Window::default(),
            per_page: q.per_page,
            scope: "all".into(),
            offset_mode: false,
        }
    }
    fn matches(&self, _: &String, _: &CollectionQuery) -> bool {
        assert_ne!(std::thread::current().id(), self.0);
        true
    }
}
#[tokio::test]
async fn scans_run_on_the_blocking_pool() {
    let dir = tempfile::tempdir().unwrap();
    let table = Store::open(dir.path().join("async.redb"))
        .unwrap()
        .table::<u64, String, _, _>("rows", U64Key, PostcardCodec::default())
        .unwrap()
        .index("name", |v: &String| v.as_bytes().to_vec())
        .unwrap();
    table.put(&1, &"Ada".into()).unwrap();
    let source = RedbCollection::new(AsyncTable::new(table), Mapping(std::thread::current().id()));
    let page = source
        .query(&CollectionQuery::default(), &RequestContext::default())
        .await
        .unwrap();
    assert_eq!(page.rows, ["Ada"]);
}

struct BlockingMapping {
    entered: tokio::sync::mpsc::UnboundedSender<()>,
    gate: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
}
impl CollectionMapping<String> for BlockingMapping {
    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
    }
    fn columns(&self) -> Vec<ColumnSpec> {
        vec![]
    }
    fn scan_request(&self, q: &CollectionQuery) -> ScanRequest {
        Mapping(std::thread::current().id()).scan_request(q)
    }
    fn matches(&self, _: &String, _: &CollectionQuery) -> bool {
        self.entered.send(()).unwrap();
        let (mutex, signal) = &*self.gate;
        let mut open = mutex.lock().unwrap();
        while !*open {
            open = signal.wait(open).unwrap();
        }
        true
    }
}
#[tokio::test]
async fn cancelled_requests_keep_capacity_and_mapping_views_share_the_budget() {
    let dir = tempfile::tempdir().unwrap();
    let table = Store::open(dir.path().join("limit.redb"))
        .unwrap()
        .table::<u64, String, _, _>("rows", U64Key, PostcardCodec::default())
        .unwrap()
        .index("name", |v| v.as_bytes().to_vec())
        .unwrap();
    table.put(&1, &"Ada".into()).unwrap();
    let (entered, mut events) = tokio::sync::mpsc::unbounded_channel();
    let gate = std::sync::Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
    let source = RedbCollection::new(
        AsyncTable::new(table).concurrency(1),
        BlockingMapping {
            entered: entered.clone(),
            gate: gate.clone(),
        },
    );
    let other = source.with_mapping(BlockingMapping {
        entered,
        gate: gate.clone(),
    });
    let first = tokio::spawn(async move {
        source
            .query(&CollectionQuery::default(), &RequestContext::default())
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), events.recv())
        .await
        .unwrap()
        .unwrap();
    first.abort();
    let second = tokio::spawn(async move {
        other
            .query(&CollectionQuery::default(), &RequestContext::default())
            .await
    });
    let blocked = tokio::time::timeout(std::time::Duration::from_millis(100), events.recv())
        .await
        .is_err();
    // Release even on a failed assertion, so Tokio shutdown cannot hang.
    *gate.0.lock().unwrap() = true;
    gate.1.notify_all();
    second.await.unwrap().unwrap();
    assert!(
        blocked,
        "a cancelled blocking scan must retain its shared permit"
    );
}

fn async_rows(dir: &tempfile::TempDir) -> AsyncTable<u64, String> {
    AsyncTable::new(
        Store::open(dir.path().join("writes.redb"))
            .unwrap()
            .table::<u64, String, _, _>("rows", U64Key, PostcardCodec::default())
            .unwrap()
            .index("name", |v: &String| v.as_bytes().to_vec())
            .unwrap(),
    )
}

#[tokio::test]
async fn async_writes_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let rows = async_rows(&dir);
    let (id, rev) = rows.create(|id| format!("row {id}")).await.unwrap();
    assert_eq!(id, 1);
    let (value, read) = rows.get_with_revision(1).await.unwrap().unwrap();
    assert_eq!((value.as_str(), read), ("row 1", rev));
    let next = rows.update_if(1, rev, |_| "Ada".into()).await.unwrap();
    assert_eq!(next, rows.revision_of(&"Ada".into()).unwrap());
    assert!(
        rows.update_if(1, rev, |_| "stale".into())
            .await
            .unwrap_err()
            .is_conflict()
    );
    rows.put(2, "Grace".into()).await.unwrap();
    assert_eq!(rows.get(2).await.unwrap(), Some("Grace".into()));
    rows.remove_if(1, next).await.unwrap();
    assert!(rows.remove(2).await.unwrap());
    assert_eq!(rows.get(1).await.unwrap(), None);
}

#[tokio::test]
async fn reads_and_writes_share_one_budget() {
    let dir = tempfile::tempdir().unwrap();
    let rows = async_rows(&dir).concurrency(1);
    let (_, rev) = rows.create(|_| "Ada".into()).await.unwrap();
    let source = RedbCollection::new(rows.clone(), Mapping(std::thread::current().id()));
    let gate = std::sync::Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
    let (entered, mut events) = tokio::sync::mpsc::unbounded_channel();
    let held = gate.clone();
    let write = tokio::spawn(async move {
        rows.update_if(1, rev, move |v| {
            entered.send(()).unwrap();
            let (mutex, signal) = &*held;
            let mut open = mutex.lock().unwrap();
            while !*open {
                open = signal.wait(open).unwrap();
            }
            v
        })
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), events.recv())
        .await
        .unwrap()
        .unwrap();
    let blocked = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        source.query(&CollectionQuery::default(), &RequestContext::default()),
    )
    .await
    .is_err();
    *gate.0.lock().unwrap() = true;
    gate.1.notify_all();
    write.await.unwrap().unwrap();
    assert!(
        blocked,
        "a scan must wait for the write holding the only permit"
    );
    let page = source
        .query(&CollectionQuery::default(), &RequestContext::default())
        .await
        .unwrap();
    assert_eq!(page.rows, ["Ada"]);
}
