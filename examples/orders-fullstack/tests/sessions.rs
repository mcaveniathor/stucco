#![cfg(feature = "ssr")]
use orders_fullstack::{sessions::Sessions, store::OrderStore};
use std::collections::HashMap;
use tower_sessions::{
    ExpiredDeletion, SessionStore,
    session::{Id, Record},
};
async fn contract(store: OrderStore) {
    let sessions = Sessions::new(&store).await.unwrap();
    let mut record = Record {
        id: Id::default(),
        data: HashMap::new(),
        expiry_date: time::OffsetDateTime::now_utc() + time::Duration::hours(1),
    };
    record.data.insert(
        "auth_deadline".into(),
        serde_json::json!(record.expiry_date.unix_timestamp()),
    );
    sessions.create(&mut record).await.unwrap();
    let mut collision = record.clone();
    sessions.create(&mut collision).await.unwrap();
    assert_ne!(collision.id, record.id);
    assert_eq!(
        sessions.load(&record.id).await.unwrap().unwrap().data,
        record.data
    );
    sessions.delete(&record.id).await.unwrap();
    assert!(
        sessions.save(&record).await.is_err(),
        "A delayed request must not resurrect logout"
    );
    assert!(sessions.load(&record.id).await.unwrap().is_none());
    collision.expiry_date = time::OffsetDateTime::UNIX_EPOCH;
    sessions.save(&collision).await.unwrap();
    assert!(sessions.load(&collision.id).await.unwrap().is_none());
    assert!(sessions.save(&collision).await.is_err());
    sessions.delete_expired().await.unwrap();
    drop(sessions);
    store.close().await;
}
#[tokio::test]
async fn default_redb_sessions_do_not_resurrect_revoked_or_expired_records() {
    let dir = tempfile::tempdir().unwrap();
    contract(
        OrderStore::open(&dir.path().join("sessions.redb"))
            .await
            .unwrap(),
    )
    .await;
}
#[cfg(feature = "sqlite")]
#[tokio::test]
async fn optional_sqlite_sessions_obey_the_same_revocation_contract() {
    let dir = tempfile::tempdir().unwrap();
    contract(
        OrderStore::open_sqlite(&dir.path().join("sessions.sqlite3"))
            .await
            .unwrap(),
    )
    .await;
}
