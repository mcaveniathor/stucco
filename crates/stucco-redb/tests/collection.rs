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
