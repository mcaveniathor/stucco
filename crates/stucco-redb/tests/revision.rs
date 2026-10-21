use stucco_redb::Revision;

#[test]
fn revision_round_trips_through_its_string() {
    let r: Revision = "AAAAAAAAAAAAAAAAAAAAAA".parse().unwrap(); // 16 zero bytes
    assert_eq!(r.to_string().len(), 22);
    assert_eq!(r.to_string().parse::<Revision>().unwrap(), r);
}

#[test]
fn malformed_revisions_are_rejected() {
    for bad in [
        "",
        "short",
        "AAAAAAAAAAAAAAAAAAAAAAAA",
        "!!!!!!!!!!!!!!!!!!!!!!",
        "AAAAAAAAAAAAAAAAAAAAA=",
    ] {
        assert!(bad.parse::<Revision>().is_err(), "{bad:?}");
    }
}

use stucco_redb::{IndexTable, PostcardCodec, Store, StoreError, U64Key};

fn names(dir: &tempfile::TempDir) -> IndexTable<u64, String> {
    Store::open(dir.path().join("rev.redb"))
        .unwrap()
        .table::<u64, String, _, _>("rows", U64Key, PostcardCodec::default())
        .unwrap()
        .index("name", |v: &String| v.as_bytes().to_vec())
        .unwrap()
}

fn indexed(table: &IndexTable<u64, String>) -> Vec<String> {
    table
        .all(true)
        .unwrap()
        .into_iter()
        .map(|(_, v)| v)
        .collect()
}

#[test]
fn update_if_writes_when_revision_matches() -> Result<(), StoreError> {
    let dir = tempfile::tempdir().unwrap();
    let table = names(&dir);
    table.put(&1, &"Ada".into())?;
    let (value, rev) = table.get_with_revision(&1)?.unwrap();
    assert_eq!(value, "Ada");
    assert_eq!(rev, table.revision_of(&"Ada".into())?);
    let next = table.update_if(&1, &rev, |_| "Grace".into())?;
    assert_eq!(next, table.revision_of(&"Grace".into())?);
    assert_eq!(indexed(&table), ["Grace"]);
    Ok(())
}

#[test]
fn update_if_conflicts_after_a_change() -> Result<(), StoreError> {
    let dir = tempfile::tempdir().unwrap();
    let table = names(&dir);
    table.put(&1, &"Ada".into())?;
    let (_, rev) = table.get_with_revision(&1)?.unwrap();
    table.put(&1, &"Grace".into())?;
    let err = table.update_if(&1, &rev, |_| "Zed".into()).unwrap_err();
    assert!(err.is_conflict());
    assert_eq!(
        err.conflict_revision(),
        Some(Some(table.revision_of(&"Grace".into())?))
    );
    assert_eq!(table.get(&1)?, Some("Grace".into()));
    assert_eq!(indexed(&table), ["Grace"]);
    Ok(())
}

#[test]
fn update_if_conflicts_when_deleted() -> Result<(), StoreError> {
    let dir = tempfile::tempdir().unwrap();
    let table = names(&dir);
    table.put(&1, &"Ada".into())?;
    let (_, rev) = table.get_with_revision(&1)?.unwrap();
    table.remove(&1)?;
    let err = table.update_if(&1, &rev, |_| "Zed".into()).unwrap_err();
    assert_eq!(err.conflict_revision(), Some(None));
    assert_eq!(table.get(&1)?, None);
    Ok(())
}

#[test]
fn remove_if_rejects_stale_and_removes_current() -> Result<(), StoreError> {
    let dir = tempfile::tempdir().unwrap();
    let table = names(&dir);
    table.put(&1, &"Ada".into())?;
    let (_, stale) = table.get_with_revision(&1)?.unwrap();
    table.put(&1, &"Grace".into())?;
    assert!(table.remove_if(&1, &stale).unwrap_err().is_conflict());
    assert_eq!(indexed(&table), ["Grace"]);
    let (_, current) = table.get_with_revision(&1)?.unwrap();
    table.remove_if(&1, &current)?;
    assert_eq!(table.get(&1)?, None);
    assert!(indexed(&table).is_empty());
    assert_eq!(
        table
            .remove_if(&1, &current)
            .unwrap_err()
            .conflict_revision(),
        Some(None)
    );
    Ok(())
}

#[test]
fn create_on_empty_table_starts_at_one() -> Result<(), StoreError> {
    let dir = tempfile::tempdir().unwrap();
    let table = names(&dir);
    let (id, rev) = table.create(|id| format!("row {id}"))?;
    assert_eq!(id, 1);
    assert_eq!(table.get_with_revision(&1)?, Some(("row 1".into(), rev)));
    table.put(&10, &"ten".into())?;
    let (first_11, old_rev) = table.create(|id| format!("row {id}"))?;
    assert_eq!(first_11, 11);
    table.remove(&11)?;
    // Deleting the highest id frees it for the next create.
    let (again, _) = table.create(|_| "replacement".into())?;
    assert_eq!(again, 11);
    assert!(
        table
            .update_if(&11, &old_rev, |_| "stale".into())
            .unwrap_err()
            .is_conflict()
    );
    assert_eq!(indexed(&table), ["replacement", "row 1", "ten"]);
    Ok(())
}

#[test]
fn concurrent_creates_get_distinct_ids() {
    let dir = tempfile::tempdir().unwrap();
    let table = names(&dir);
    let threads: Vec<_> = (0..8)
        .map(|t| {
            let table = table.clone();
            std::thread::spawn(move || {
                (0..25)
                    .map(|i| table.create(|_| format!("{t}-{i}")).unwrap().0)
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let mut ids: Vec<u64> = threads
        .into_iter()
        .flat_map(|t| t.join().unwrap())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids, (1..=200).collect::<Vec<_>>());
}
