#![cfg(feature = "tokio")]
use stucco_core::{Capabilities, CollectionQuery, ColumnSpec, Direction, Window};
use stucco_redb::{CollectionMapping, PostcardCodec, RedbCollection, ScanRequest, Store, U64Key};
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
    let source = RedbCollection::new(table, Mapping(std::thread::current().id()));
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
        table,
        BlockingMapping {
            entered: entered.clone(),
            gate: gate.clone(),
        },
    )
    .concurrency(1);
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
