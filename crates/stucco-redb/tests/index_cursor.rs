use stucco_core::Direction::{Asc, Desc};
use stucco_redb::{PostcardCodec, Store, StoreError, Utf8Key};

#[test]
fn equal_cursor_is_exclusive_bounded_and_namespace_safe() -> Result<(), StoreError> {
    let dir = tempfile::tempdir().unwrap();
    let db = Store::open(dir.path().join("cursor.redb"))?;
    let rows = db
        .table::<String, (String, String), _, _>("rows", Utf8Key, PostcardCodec::default())?
        .index("group", |r| r.0.as_bytes().to_vec())?;
    for (key, group) in [
        ("", "a\0"),
        ("a", "a\0"),
        ("a\0", "a\0"),
        ("z", "a\0"),
        ("before", "a"),
        ("after", "a\0\0"),
        ("other", "b"),
    ] {
        rows.put(&key.into(), &(group.into(), key.into()))?;
    }
    let page = |after: Option<&str>, direction, limit| {
        db.read(|tx| {
            let key = after.map(str::to_owned);
            Ok::<_, StoreError>(
                tx.index_equal_after(&rows, b"a\0", key.as_ref(), direction, limit)?
                    .into_iter()
                    .map(|r| r.1)
                    .collect::<Vec<_>>(),
            )
        })
    };
    assert_eq!(page(None, Asc, 2)?, ["", "a"]);
    assert_eq!(page(Some("a"), Asc, 2)?, ["a\0", "z"]);
    assert_eq!(page(Some("a\0"), Desc, 2)?, ["a", ""]);
    assert_eq!(page(None, Desc, 1)?, ["z"]);
    assert_eq!(page(Some("missing"), Asc, 8)?, ["z"]);
    assert_eq!(page(Some("missing"), Desc, 8)?, ["a\0", "a", ""]);
    assert!(page(Some("z"), Asc, 8)?.is_empty());
    assert!(page(Some(""), Desc, 8)?.is_empty());
    assert!(page(None, Asc, 0)?.is_empty());
    assert!(
        db.read(|tx| tx.index_equal_after(&rows, b"absent", None, Asc, 4))?
            .is_empty()
    );
    rows.remove(&"a".into())?;
    assert_eq!(page(Some("a"), Asc, 1)?, ["a\0"]);
    Ok(())
}

#[test]
fn write_cursor_sees_own_writes_and_rolls_back() -> Result<(), StoreError> {
    let dir = tempfile::tempdir().unwrap();
    let db = Store::open(dir.path().join("write.redb"))?;
    let rows = db
        .table::<String, String, _, _>("rows", Utf8Key, PostcardCodec::default())?
        .index("group", |r| r.as_bytes().to_vec())?;
    rows.put(&"a".into(), &"home".into())?;
    let failed = db.write(|tx| {
        tx.put(&rows, &"b".into(), &"home".into())?;
        assert_eq!(
            tx.index_equal_after(&rows, b"home", Some(&"a".into()), Asc, 1)?,
            ["home"]
        );
        assert_eq!(
            tx.index_equal_after(&rows, b"home", Some(&"b".into()), Desc, 1)?,
            ["home"]
        );
        tx.remove(&rows, &"a".into())?;
        assert!(
            tx.index_equal_after(&rows, b"home", Some(&"b".into()), Desc, 1)?
                .is_empty()
        );
        Err::<(), _>(StoreError::new(std::io::Error::other("rollback")))
    });
    assert!(failed.is_err());
    assert_eq!(
        db.read(|tx| tx.index_equal_after(&rows, b"home", None, Asc, 100))?,
        ["home"]
    );
    let other_dir = tempfile::tempdir().unwrap();
    let other = Store::open(other_dir.path().join("other.redb"))?;
    assert!(
        other
            .read(|tx| tx.index_equal_after(&rows, b"home", None, Asc, 1))
            .is_err()
    );
    assert!(
        other
            .write(|tx| tx.index_equal_after(&rows, b"home", None, Asc, 0))
            .is_err()
    );
    Ok(())
}

#[test]
fn cursor_seeks_without_decoding_preceding_rows_or_lookahead() -> Result<(), StoreError> {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use stucco_redb::{Codec, U64Key};
    struct Counted(Arc<AtomicUsize>);
    impl Codec<u64> for Counted {
        fn encode(&self, value: &u64) -> Result<Vec<u8>, StoreError> {
            Ok(value.to_be_bytes().to_vec())
        }
        fn decode(&self, bytes: &[u8]) -> Result<u64, StoreError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(u64::from_be_bytes(bytes.try_into().unwrap()))
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let db = Store::open(dir.path().join("bounded.redb"))?;
    let count = Arc::new(AtomicUsize::new(0));
    let rows = db
        .table("rows", U64Key, Counted(count.clone()))?
        .index("group", |_| b"home".to_vec())?;
    db.write(|tx| {
        for id in 1..=100 {
            tx.put(&rows, &id, &id)?;
        }
        Ok::<_, StoreError>(())
    })?;
    count.store(0, Ordering::SeqCst);
    assert_eq!(
        db.read(|tx| tx.index_equal_after(&rows, b"home", Some(&90), Asc, 2))?,
        [91, 92]
    );
    assert_eq!(count.load(Ordering::SeqCst), 2);
    count.store(0, Ordering::SeqCst);
    assert_eq!(
        db.write(|tx| tx.index_equal_after(&rows, b"home", Some(&90), Desc, 2))?,
        [89, 88]
    );
    assert_eq!(count.load(Ordering::SeqCst), 2);
    count.store(0, Ordering::SeqCst);
    assert!(
        db.read(|tx| tx.index_equal_after(&rows, b"home", None, Asc, 0))?
            .is_empty()
    );
    assert_eq!(count.load(Ordering::SeqCst), 0);
    Ok(())
}
