use stucco_redb::{Codec, PostcardCodec, Store, StoreError, U64Key, Utf8Key};
#[test]
fn values_survive_reopening_and_can_be_replaced_and_removed() -> Result<(), StoreError> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.redb");
    {
        let store = Store::open(&path)?;
        let rows = store.table::<u64, String, _, _>("orders", U64Key, PostcardCodec::default())?;
        assert_eq!(rows.get(&1)?, None);
        rows.put(&256, &"Ada".into())?;
        rows.put(&256, &"Grace".into())?;
    }
    let store = Store::open(&path)?;
    let rows = store.table::<u64, String, _, _>("orders", U64Key, PostcardCodec::default())?;
    assert_eq!(rows.get(&256)?, Some("Grace".into()));
    assert!(rows.remove(&256)?);
    assert!(!rows.remove(&256)?);
    assert!(
        store
            .table::<u64, String, _, _>("bad/name", U64Key, PostcardCodec::default())
            .is_err()
    );
    Ok(())
}
#[test]
fn ordered_keys_and_corrupt_values_are_explicit() {
    let bytes: Vec<_> = [1, 2, 255, 256]
        .iter()
        .map(|k| U64Key.encode(k).unwrap())
        .collect();
    assert!(bytes.windows(2).all(|w| w[0] < w[1]));
    assert!(U64Key.decode(&[1]).is_err());
    assert!(Utf8Key.decode(&[255]).is_err());
    assert!(PostcardCodec::<String>::default().decode(&[255]).is_err());
}
