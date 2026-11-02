use stucco_core::Direction;
use stucco_redb::{Codec, IndexTable, PostcardCodec, Store, StoreError, U64Key};

type Scoped = IndexTable<u64, (u64, Vec<u8>)>;
fn scoped(dir: &tempfile::TempDir) -> (Store, Scoped) {
    let store = Store::open(dir.path().join("range.redb")).unwrap();
    let table = store
        .table::<u64, (u64, Vec<u8>), _, _>("rows", U64Key, PostcardCodec::default())
        .unwrap()
        .index("scope", |(scope, sort)| {
            [scope.to_be_bytes().as_slice(), sort].concat()
        })
        .unwrap();
    (store, table)
}
fn range_keys<'r>(
    store: &Store,
    table: &Scoped,
    range: impl std::ops::RangeBounds<&'r [u8]>,
    direction: Direction,
    limit: usize,
) -> Vec<Vec<u8>> {
    store
        .read(|tx| tx.index_range(table, range, direction, limit))
        .unwrap()
        .into_iter()
        .map(|(_, sort)| sort)
        .collect()
}
#[test]
fn index_range_stays_inside_a_scope_in_either_direction() {
    let dir = tempfile::tempdir().unwrap();
    let (store, table) = scoped(&dir);
    // Scope 0x0100 and 0x0101 share every byte but the last, and the scopes'
    // zero bytes are escaped in the stored keys.
    for (id, (scope, sort)) in [
        (0x0100, vec![2]),
        (0x0100, vec![0]),
        (0x0100, vec![1]),
        (0x0101, vec![9]),
        (0x00ff, vec![9]),
    ]
    .into_iter()
    .enumerate()
    {
        table.put(&(id as u64 + 1), &(scope, sort)).unwrap();
    }
    let (from, to) = (0x0100_u64.to_be_bytes(), 0x0101_u64.to_be_bytes());
    let scope = from.as_slice()..to.as_slice();
    assert_eq!(
        range_keys(&store, &table, scope.clone(), Direction::Asc, 10),
        [vec![0], vec![1], vec![2]]
    );
    assert_eq!(
        range_keys(&store, &table, scope.clone(), Direction::Desc, 2),
        [vec![2], vec![1]]
    );
    assert!(range_keys(&store, &table, scope, Direction::Asc, 0).is_empty());
}
#[test]
fn index_range_honours_each_kind_of_bound() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("bounds.redb")).unwrap();
    let table = store
        .table::<u64, Vec<u8>, _, _>("rows", U64Key, PostcardCodec::default())
        .unwrap()
        .index("value", |v| v.clone())
        .unwrap();
    let values: [&[u8]; 6] = [b"a", b"ab", b"ab\0", b"ab\0c", b"ac", b"b"];
    for (id, value) in values.iter().enumerate() {
        table.put(&(id as u64), &value.to_vec()).unwrap();
    }
    use std::ops::Bound::{Excluded, Included, Unbounded};
    let read = |range: (std::ops::Bound<&[u8]>, std::ops::Bound<&[u8]>)| {
        store
            .read(|tx| tx.index_range(&table, range, Direction::Asc, 10))
            .unwrap()
    };
    let ab: &[u8] = b"ab";
    let ac: &[u8] = b"ac";
    assert_eq!(read((Unbounded, Unbounded)).len(), 6);
    assert_eq!(read((Included(ab), Excluded(ac))), values[1..4]);
    assert_eq!(read((Included(ab), Included(ac))), values[1..5]);
    assert_eq!(read((Excluded(ab), Included(ac))), values[2..5]);
    assert_eq!(read((Unbounded, Excluded(ab))), values[..1]);
    assert_eq!(read((Excluded(ab), Unbounded)), values[2..]);
    // Empty and inverted ranges read nothing rather than panicking.
    assert!(read((Included(ab), Excluded(ab))).is_empty());
    assert!(read((Excluded(ac), Excluded(ab))).is_empty());
    assert!(read((Excluded(ab), Excluded(ab))).is_empty());
}
#[test]
fn index_range_follows_updates_and_removals() {
    let dir = tempfile::tempdir().unwrap();
    let (store, table) = scoped(&dir);
    table.put(&1, &(1, vec![5])).unwrap();
    table.put(&2, &(1, vec![6])).unwrap();
    table.put(&1, &(1, vec![7])).unwrap();
    table.remove(&2).unwrap();
    let (from, to) = (1_u64.to_be_bytes(), 2_u64.to_be_bytes());
    assert_eq!(
        range_keys(
            &store,
            &table,
            from.as_slice()..to.as_slice(),
            Direction::Asc,
            10
        ),
        [vec![7]]
    );
}
#[test]
fn index_range_in_a_write_transaction_sees_its_own_writes() {
    let dir = tempfile::tempdir().unwrap();
    let (store, table) = scoped(&dir);
    table.put(&1, &(1, vec![5])).unwrap();
    let (from, to) = (1_u64.to_be_bytes(), 2_u64.to_be_bytes());
    let seen = store
        .write(|tx| {
            tx.put(&table, &2, &(1, vec![6]))?;
            tx.remove(&table, &1)?;
            tx.index_range(&table, from.as_slice()..to.as_slice(), Direction::Asc, 10)
        })
        .unwrap();
    assert_eq!(seen, [(1, vec![6])]);
}
#[test]
fn index_range_rejects_a_handle_from_another_store() {
    let dir = tempfile::tempdir().unwrap();
    let (_, table) = scoped(&dir);
    let other = Store::open(dir.path().join("other.redb")).unwrap();
    assert!(
        other
            .read(|tx| tx.index_range(&table, .., Direction::Asc, 1))
            .is_err()
    );
}
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
