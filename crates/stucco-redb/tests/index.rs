use stucco_redb::{Codec, PostcardCodec, Store, StoreError, U64Key};
#[test]
fn index_namespace_cannot_alias_primary_tables_or_other_indexes() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("names.redb")).unwrap();
    let unrelated = store
        .table::<u64, String, _, _>("orders--customer", U64Key, PostcardCodec::default())
        .unwrap();
    unrelated.put(&1, &"preserve me".into()).unwrap();
    let orders = store
        .table::<u64, String, _, _>("orders", U64Key, PostcardCodec::default())
        .unwrap()
        .index("customer", |v| v.as_bytes().to_vec())
        .unwrap();
    orders.rebuild().unwrap();
    assert_eq!(unrelated.get(&1).unwrap(), Some("preserve me".into()));
    let left = store
        .table::<u64, String, _, _>("a--b", U64Key, PostcardCodec::default())
        .unwrap()
        .index("c", |v| v.as_bytes().to_vec())
        .unwrap();
    let right = store
        .table::<u64, String, _, _>("a", U64Key, PostcardCodec::default())
        .unwrap()
        .index("b--c", |v| v.as_bytes().to_vec())
        .unwrap();
    left.put(&1, &"Left".into()).unwrap();
    right.put(&2, &"Right".into()).unwrap();
    left.rebuild().unwrap();
    assert_eq!(
        right
            .all(true)
            .unwrap()
            .into_iter()
            .map(|(_, v)| v)
            .collect::<Vec<_>>(),
        ["Right"]
    );
}
#[test]
fn indexes_track_duplicates_updates_deletes_and_rebuilds() -> Result<(), StoreError> {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("index.redb"))?;
    let table = store.table::<u64, String, _, _>("rows", U64Key, PostcardCodec::default())?;
    table.put(&9, &"Zed".into())?;
    let index = table.index("name", |v: &String| v.as_bytes().to_vec())?;
    index.rebuild()?;
    index.put_many(&[(1, "Ada".into()), (2, "Ada".into())])?;
    assert_eq!(
        index
            .all(true)?
            .into_iter()
            .map(|(_, v)| v)
            .collect::<Vec<_>>(),
        ["Ada", "Ada", "Zed"]
    );
    index.put(&1, &"Grace".into())?;
    assert_eq!(
        index
            .all(true)?
            .into_iter()
            .map(|(_, v)| v)
            .collect::<Vec<_>>(),
        ["Ada", "Grace", "Zed"]
    );
    assert!(index.remove(&2)?);
    assert_eq!(index.all(true)?.len(), 2);
    assert_eq!(index.get(&1)?, Some("Grace".into()));
    Ok(())
}
struct Reject;
impl Codec<String> for Reject {
    fn encode(&self, v: &String) -> Result<Vec<u8>, StoreError> {
        if v == "reject" {
            Err(StoreError::new(std::io::Error::other("rejected")))
        } else {
            Ok(v.as_bytes().to_vec())
        }
    }
    fn decode(&self, b: &[u8]) -> Result<String, StoreError> {
        String::from_utf8(b.to_vec()).map_err(StoreError::new)
    }
}
#[test]
fn failed_batch_does_not_commit_partial_data_or_index_changes() -> Result<(), StoreError> {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("fail.redb"))?;
    let table = store.table("rows", U64Key, Reject)?;
    let index = table.index("name", |v: &String| v.as_bytes().to_vec())?;
    assert!(
        index
            .put_many(&[(1, "ok".into()), (2, "reject".into())])
            .is_err()
    );
    assert!(index.get(&1)?.is_none());
    assert!(index.all(true)?.is_empty());
    Ok(())
}
