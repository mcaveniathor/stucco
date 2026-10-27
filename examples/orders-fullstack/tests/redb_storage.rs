#![cfg(feature = "ssr")]

use orders_fullstack::{
    auth::Principal,
    domain::{DeleteError, LoadError, OrderDraft, SaveError, new_order},
    store::OrderStore,
};
use redb::{
    Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition, TableHandle,
};
use std::path::PathBuf;
use tempfile::TempDir;

const META: TableDefinition<&str, u64> = TableDefinition::new("metadata");
const ORDERS: TableDefinition<u64, &[u8]> = TableDefinition::new("orders");
const INDEX: TableDefinition<(i64, u64), ()> = TableDefinition::new("orders_by_tenant");
const SESSIONS: TableDefinition<&str, &[u8]> = TableDefinition::new("sessions");

struct Fixture {
    directory: TempDir,
    path: PathBuf,
    store: OrderStore,
    alice: Principal,
    bob: Principal,
}
impl Fixture {
    async fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("orders.redb");
        let store = OrderStore::open(&path).await.unwrap();
        let alice = store
            .insert_user("team-a", "alice", "Alice", "test-only-not-a-login-hash")
            .await
            .unwrap();
        let bob = store
            .insert_user("team-b", "bob", "Bob", "test-only-not-a-login-hash")
            .await
            .unwrap();
        Self {
            directory,
            path,
            store,
            alice,
            bob,
        }
    }
}
fn draft(customer: &str) -> OrderDraft {
    OrderDraft {
        customer: customer.into(),
        ..new_order().draft()
    }
}

#[tokio::test]
async fn open_is_empty_and_explicit_seed_never_overwrites_or_crosses_tenants() {
    let f = Fixture::new().await;
    assert_eq!(
        f.store
            .list(&f.alice, "".into(), 1)
            .await
            .unwrap()
            .total_count,
        0
    );
    f.store.seed_orders(&f.alice).await.unwrap();
    let original = f.store.load_by_id(&f.alice, 1).await.unwrap().unwrap();
    assert_eq!(original.customer, "Ada Lovelace");
    assert_eq!(original.total_cents(), Ok(5900));
    let mut changed = original.draft();
    changed.customer = "Keep this edit".into();
    let saved = f.store.save_by_id(&f.alice, 1, changed).await.unwrap();
    f.store.seed_orders(&f.alice).await.unwrap();
    assert_eq!(f.store.load_by_id(&f.alice, 1).await.unwrap(), Some(saved));
    assert_eq!(
        f.store
            .list(&f.alice, "".into(), 1)
            .await
            .unwrap()
            .total_count,
        12
    );
    assert_eq!(
        f.store
            .list(&f.bob, "".into(), 1)
            .await
            .unwrap()
            .total_count,
        0
    );
    f.store.seed_orders(&f.bob).await.unwrap();
    assert_eq!(
        f.store.list(&f.bob, "".into(), 1).await.unwrap().orders[0].id,
        24
    );
}

#[tokio::test]
async fn tenant_index_pages_have_stable_sort_bounded_pages_and_literal_unicode_search() {
    let f = Fixture::new().await;
    f.store.seed_orders(&f.alice).await.unwrap();
    let first = f.store.list(&f.alice, "".into(), 0).await.unwrap();
    assert_eq!(
        (first.page, first.total_pages, first.total_count),
        (1, 3, 12)
    );
    assert_eq!(
        first.orders.iter().map(|o| o.id).collect::<Vec<_>>(),
        [12, 11, 10, 9, 8]
    );
    let last = f.store.list(&f.alice, "".into(), u32::MAX).await.unwrap();
    assert_eq!((last.page, last.orders.len()), (3, 2));
    let unicode = f
        .store
        .create(&f.alice, draft("Élodie %_ SQL ' literal"))
        .await
        .unwrap();
    for query in ["  ÉLODIE  ", "%_", "SQL '"] {
        let page = f
            .store
            .list(&f.alice, query.into(), u32::MAX)
            .await
            .unwrap();
        assert_eq!(page.orders.as_slice(), std::slice::from_ref(&unicode));
        assert_eq!((page.page, page.total_pages, page.total_count), (1, 1, 1));
        assert_eq!(page.query, query.trim());
    }
    assert_eq!(
        f.store
            .list(&f.alice, "' OR 1=1".into(), 1)
            .await
            .unwrap()
            .total_count,
        0
    );
}

#[tokio::test]
async fn validation_conflict_and_delete_keep_original_text_and_revision() {
    let f = Fixture::new().await;
    let order = f.store.create(&f.alice, draft("Original")).await.unwrap();
    let mut invalid = order.draft();
    invalid.customer = "  unsent  ".into();
    invalid.quantity_one = "7x".into();
    invalid.expected_revision = " 01 ".into();
    match f
        .store
        .save_by_id(&f.alice, order.id, invalid.clone())
        .await
    {
        Err(SaveError::Validation { draft, errors }) => {
            assert_eq!(draft, invalid);
            assert!(!errors.is_empty());
        }
        result => panic!("unexpected {result:?}"),
    }
    let mut edited = order.draft();
    edited.customer = "  Saved  ".into();
    let saved = f
        .store
        .save_by_id(&f.alice, order.id, edited)
        .await
        .unwrap();
    assert_eq!((saved.customer.as_str(), saved.revision), ("Saved", 2));
    let mut stale = order.draft();
    stale.expected_revision = " 01 ".into();
    assert_eq!(
        f.store.save_by_id(&f.alice, order.id, stale.clone()).await,
        Err(SaveError::Conflict {
            draft: stale,
            current: Box::new(saved.clone())
        })
    );
    assert_eq!(
        f.store.delete(&f.alice, order.id, " 01 ".into()).await,
        Err(DeleteError::Conflict {
            expected_revision: " 01 ".into(),
            current: Box::new(saved)
        })
    );
    assert!(
        matches!(f.store.delete(&f.alice, order.id, "1x".into()).await, Err(DeleteError::Validation { expected_revision, .. }) if expected_revision == "1x")
    );
    f.store
        .delete(&f.alice, order.id, " 02 ".into())
        .await
        .unwrap();
    assert_eq!(f.store.load_by_id(&f.alice, order.id).await.unwrap(), None);
}

#[tokio::test]
async fn cross_tenant_ids_are_absent_for_reads_search_edits_and_deletes() {
    let f = Fixture::new().await;
    let order = f.store.create(&f.bob, draft("Bob private")).await.unwrap();
    assert_eq!(f.store.load_by_id(&f.alice, order.id).await.unwrap(), None);
    assert_eq!(
        f.store
            .list(&f.alice, "Bob".into(), 1)
            .await
            .unwrap()
            .total_count,
        0
    );
    let wrong = draft("Wrong tenant");
    assert_eq!(
        f.store.save_by_id(&f.alice, order.id, wrong.clone()).await,
        Err(SaveError::NotFound { draft: wrong })
    );
    assert_eq!(
        f.store.delete(&f.alice, order.id, "1".into()).await,
        Err(DeleteError::NotFound {
            expected_revision: "1".into()
        })
    );
    assert_eq!(
        f.store.load_by_id(&f.bob, order.id).await.unwrap(),
        Some(order)
    );
}

#[tokio::test]
async fn same_revision_saves_and_save_delete_races_have_only_one_winner() {
    let f = Fixture::new().await;
    let order = f.store.create(&f.alice, draft("Race")).await.unwrap();
    let mut a = order.draft();
    a.customer = "A".into();
    let mut b = order.draft();
    b.customer = "B".into();
    let (a, b) = tokio::join!(
        f.store.save_by_id(&f.alice, order.id, a),
        f.store.save_by_id(&f.alice, order.id, b)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(
        matches!(a, Err(SaveError::Conflict { .. }))
            || matches!(b, Err(SaveError::Conflict { .. }))
    );
    let current = f
        .store
        .load_by_id(&f.alice, order.id)
        .await
        .unwrap()
        .unwrap();
    let (save, delete) = tokio::join!(
        f.store.save_by_id(&f.alice, order.id, current.draft()),
        f.store
            .delete(&f.alice, order.id, current.revision.to_string())
    );
    assert_eq!(usize::from(save.is_ok()) + usize::from(delete.is_ok()), 1);
    match (save, delete) {
        (Ok(saved), Err(DeleteError::Conflict { current, .. })) => assert_eq!(saved, *current),
        (Err(SaveError::NotFound { .. }), Ok(())) => {}
        result => panic!("unexpected race outcome {result:?}"),
    }
}

#[tokio::test]
async fn concurrent_ids_are_unique_and_highest_deleted_id_stays_used_after_restart() {
    let f = Fixture::new().await;
    let mut tasks = Vec::new();
    for n in 0..4 {
        let store = f.store.clone();
        let user = f.alice.clone();
        tasks.push(tokio::spawn(async move {
            let mut ids = Vec::new();
            for i in 0..4 {
                ids.push(
                    store
                        .create(&user, draft(&format!("Order {n}-{i}")))
                        .await
                        .unwrap()
                        .id,
                );
            }
            ids
        }));
    }
    let mut ids = Vec::new();
    for task in tasks {
        ids.extend(task.await.unwrap());
    }
    ids.sort_unstable();
    assert_eq!(ids, (1..=16).collect::<Vec<_>>());
    f.store.delete(&f.alice, 16, "1".into()).await.unwrap();
    f.store.close().await;
    let reopened = OrderStore::open(&f.path).await.unwrap();
    assert_eq!(
        reopened
            .create(&f.alice, draft("Never reuse"))
            .await
            .unwrap()
            .id,
        17
    );
}

#[tokio::test]
async fn identity_creation_is_atomic_unique_and_revocation_and_disable_persist() {
    let f = Fixture::new().await;
    let (a, b) = tokio::join!(
        f.store.insert_user("new-team", "same-name", "A", "hash-a"),
        f.store.insert_user("new-team", "same-name", "B", "hash-b")
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(
        f.store
            .insert_user("should-not-be-created", "alice", "Duplicate", "hash")
            .await
            .is_err()
    );
    let next = f
        .store
        .insert_user("next-team", "next-user", "Next", "hash")
        .await
        .unwrap();
    assert_eq!((next.key.user_id, next.key.tenant_id), (4, 4));
    assert!(f.store.revoke_sessions("alice").await.unwrap());
    assert!(f.store.set_user_enabled("alice", false).await.unwrap());
    assert!(!f.store.revoke_sessions("missing").await.unwrap());
    assert!(!f.store.set_user_enabled("missing", false).await.unwrap());
    f.store.close().await;
    let reopened = OrderStore::open(&f.path).await.unwrap();
    let user = reopened
        .find_user_by_id(f.alice.key.user_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user.username, "alice");
    assert_eq!(user.session_epoch, 2);
    assert!(!user.enabled);
}

#[tokio::test]
async fn closed_service_handles_fail_without_losing_submitted_input() {
    let f = Fixture::new().await;
    let clone = f.store.clone();
    f.store.close().await;
    assert!(clone.health().await.is_err());
    assert_eq!(clone.load_by_id(&f.alice, 1).await, Err(LoadError));
    assert_eq!(clone.list(&f.alice, "".into(), 1).await, Err(LoadError));
    let input = draft("Retain me");
    assert_eq!(
        clone.create(&f.alice, input.clone()).await,
        Err(SaveError::Unavailable {
            draft: input.clone()
        })
    );
    assert_eq!(
        clone.save_by_id(&f.alice, 1, input.clone()).await,
        Err(SaveError::Unavailable { draft: input })
    );
    assert_eq!(
        clone.delete(&f.alice, 1, " 01 ".into()).await,
        Err(DeleteError::Unavailable {
            expected_revision: " 01 ".into()
        })
    );
}

#[tokio::test]
async fn counter_and_revision_exhaustion_roll_back_without_reusing_or_wrapping() {
    let f = Fixture::new().await;
    let original = f
        .store
        .create(&f.alice, draft("Max revision"))
        .await
        .unwrap();
    f.store.close().await;
    let database = Database::open(&f.path).unwrap();
    let tx = database.begin_write().unwrap();
    let mut encoded: serde_json::Value = serde_json::from_slice(
        tx.open_table(ORDERS)
            .unwrap()
            .get(original.id)
            .unwrap()
            .unwrap()
            .value(),
    )
    .unwrap();
    encoded["order"]["revision"] = (i64::MAX as u64).into();
    let bytes = serde_json::to_vec(&encoded).unwrap();
    tx.open_table(ORDERS)
        .unwrap()
        .insert(original.id, bytes.as_slice())
        .unwrap();
    tx.open_table(META)
        .unwrap()
        .insert("last_order_id", i64::MAX as u64)
        .unwrap();
    tx.commit().unwrap();
    drop(database);
    let store = OrderStore::open(&f.path).await.unwrap();
    let current = store
        .load_by_id(&f.alice, original.id)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        store
            .save_by_id(&f.alice, original.id, current.draft())
            .await,
        Err(SaveError::Unavailable { .. })
    ));
    assert_eq!(
        store.load_by_id(&f.alice, original.id).await.unwrap(),
        Some(current)
    );
    let input = draft("No ID available");
    assert_eq!(
        store.create(&f.alice, input.clone()).await,
        Err(SaveError::Unavailable { draft: input })
    );
    assert_eq!(
        store
            .list(&f.alice, "".into(), 1)
            .await
            .unwrap()
            .total_count,
        1
    );
    assert_eq!(store.load_by_id(&f.alice, 0).await.unwrap(), None);
    assert_eq!(store.load_by_id(&f.alice, u64::MAX).await.unwrap(), None);
}

#[tokio::test]
async fn checked_totals_reject_the_entire_document_update_without_advancing_revision() {
    let f = Fixture::new().await;
    let original = f
        .store
        .create(&f.alice, draft("Trusted snapshot"))
        .await
        .unwrap();
    f.store.close().await;
    let database = Database::open(&f.path).unwrap();
    let tx = database.begin_write().unwrap();
    let mut encoded: serde_json::Value = serde_json::from_slice(
        tx.open_table(ORDERS)
            .unwrap()
            .get(original.id)
            .unwrap()
            .unwrap()
            .value(),
    )
    .unwrap();
    encoded["order"]["items"][0]["unit_price_cents"] = (i64::MAX as u64).into();
    let bytes = serde_json::to_vec(&encoded).unwrap();
    tx.open_table(ORDERS)
        .unwrap()
        .insert(original.id, bytes.as_slice())
        .unwrap();
    tx.commit().unwrap();
    drop(database);
    let store = OrderStore::open(&f.path).await.unwrap();
    let before = store
        .load_by_id(&f.alice, original.id)
        .await
        .unwrap()
        .unwrap();
    let mut overflow = before.draft();
    overflow.customer = "This must not be committed".into();
    overflow.quantity_one = "3".into();
    match store
        .save_by_id(&f.alice, original.id, overflow.clone())
        .await
    {
        Err(SaveError::Validation { draft, .. }) => assert_eq!(draft, overflow),
        result => panic!("unexpected {result:?}"),
    }
    assert_eq!(
        store.load_by_id(&f.alice, original.id).await.unwrap(),
        Some(before)
    );
}

#[tokio::test]
async fn backup_omits_sessions_and_recovers_consistent_orders_users_and_counters() {
    let f = Fixture::new().await;
    let first = f.store.create(&f.alice, draft("Snapshot")).await.unwrap();
    let saved = f
        .store
        .save_by_id(&f.alice, first.id, first.draft())
        .await
        .unwrap();
    let deleted = f
        .store
        .create(&f.bob, draft("Deleted last ID"))
        .await
        .unwrap();
    f.store
        .delete(&f.bob, deleted.id, "1".into())
        .await
        .unwrap();
    f.store.close().await;
    let database = Database::open(&f.path).unwrap();
    let tx = database.begin_write().unwrap();
    tx.open_table(SESSIONS)
        .unwrap()
        .insert(
            "unique-secret-session-id",
            b"unique-private-native-recovery".as_slice(),
        )
        .unwrap();
    tx.commit().unwrap();
    drop(database);
    let store = OrderStore::open(&f.path).await.unwrap();
    let backup = f.directory.path().join("backup.redb");
    store.backup_to(&backup).await.unwrap();
    assert!(store.backup_to(&backup).await.is_err());
    OrderStore::validate_backup(&backup).await.unwrap();
    let bytes = std::fs::read(&backup).unwrap();
    for secret in ["unique-secret-session-id", "unique-private-native-recovery"] {
        assert!(
            !bytes
                .windows(secret.len())
                .any(|window| window == secret.as_bytes())
        );
    }
    store
        .create(&f.alice, draft("After snapshot"))
        .await
        .unwrap();
    store.close().await;
    let database = Database::open(&f.path).unwrap();
    assert_eq!(
        database
            .begin_read()
            .unwrap()
            .open_table(SESSIONS)
            .unwrap()
            .len()
            .unwrap(),
        1
    );
    drop(database);
    let restored_path = f.directory.path().join("restored.redb");
    std::fs::copy(&backup, &restored_path).unwrap();
    let restored = OrderStore::open(&restored_path).await.unwrap();
    assert_eq!(
        restored.load_by_id(&f.alice, saved.id).await.unwrap(),
        Some(saved)
    );
    assert_eq!(
        restored
            .list(&f.alice, "After snapshot".into(), 1)
            .await
            .unwrap()
            .total_count,
        0
    );
    assert_eq!(
        restored
            .find_user_by_username("alice")
            .await
            .unwrap()
            .unwrap()
            .tenant_id,
        f.alice.key.tenant_id
    );
    assert_eq!(
        restored
            .create(&f.alice, draft("After restore"))
            .await
            .unwrap()
            .id,
        deleted.id + 1
    );
}

#[tokio::test]
async fn v1_to_v2_upgrade_builds_tenant_index_without_changing_records_or_ids() {
    let f = Fixture::new().await;
    let order = f
        .store
        .create(&f.alice, draft("Existing version one"))
        .await
        .unwrap();
    f.store.close().await;
    let database = Database::open(&f.path).unwrap();
    let tx = database.begin_write().unwrap();
    tx.open_table(META)
        .unwrap()
        .insert("schema_version", 1)
        .unwrap();
    tx.delete_table(INDEX).unwrap();
    tx.commit().unwrap();
    drop(database);
    let upgraded = OrderStore::open(&f.path).await.unwrap();
    assert_eq!(
        upgraded.load_by_id(&f.alice, order.id).await.unwrap(),
        Some(order.clone())
    );
    assert_eq!(
        upgraded
            .list(&f.alice, "version one".into(), 1)
            .await
            .unwrap()
            .orders,
        [order]
    );
    let backup = f.directory.path().join("upgraded.redb");
    upgraded.backup_to(&backup).await.unwrap();
    OrderStore::validate_backup(&backup).await.unwrap();
    upgraded.close().await;
    let again = OrderStore::open(&f.path).await.unwrap();
    assert_eq!(again.create(&f.alice, draft("Next")).await.unwrap().id, 2);
}

#[tokio::test]
async fn foreign_redb_tables_are_rejected_without_creating_metadata_or_changing_bytes() {
    let directory = tempfile::tempdir().unwrap();
    for name in ["orders", "unrelated"] {
        let path = directory.path().join(format!("foreign-{name}.redb"));
        let definition = TableDefinition::<&[u8], &[u8]>::new(name);
        let database = Database::create(&path).unwrap();
        let tx = database.begin_write().unwrap();
        tx.open_table(definition)
            .unwrap()
            .insert(
                b"legacy-key".as_slice(),
                b"preserve-original-record".as_slice(),
            )
            .unwrap();
        tx.commit().unwrap();
        drop(database);
        let error = OrderStore::open(&path).await.unwrap_err();
        assert!(error.to_string().contains("Foreign database schema"));
        let database = Database::open(&path).unwrap();
        let read = database.begin_read().unwrap();
        assert_eq!(
            read.list_tables()
                .unwrap()
                .map(|table| table.name().to_owned())
                .collect::<Vec<_>>(),
            [name]
        );
        assert_eq!(
            read.open_table(definition)
                .unwrap()
                .get(b"legacy-key".as_slice())
                .unwrap()
                .unwrap()
                .value(),
            b"preserve-original-record"
        );
    }
}

#[tokio::test]
async fn corrupt_files_and_future_schemas_are_rejected_without_resetting_data() {
    let f = Fixture::new().await;
    let corrupt = f.directory.path().join("corrupt.redb");
    std::fs::write(&corrupt, b"not a redb database").unwrap();
    assert!(OrderStore::open(&corrupt).await.is_err());
    assert!(OrderStore::validate_backup(&corrupt).await.is_err());
    let order = f
        .store
        .create(&f.alice, draft("Retain data"))
        .await
        .unwrap();
    f.store.close().await;
    let database = Database::open(&f.path).unwrap();
    let tx = database.begin_write().unwrap();
    tx.open_table(META)
        .unwrap()
        .insert("schema_version", 99)
        .unwrap();
    tx.commit().unwrap();
    drop(database);
    assert!(OrderStore::open(&f.path).await.is_err());
    assert!(OrderStore::validate_backup(&f.path).await.is_err());
    let database = Database::open(&f.path).unwrap();
    let read = database.begin_read().unwrap();
    assert!(
        read.open_table(ORDERS)
            .unwrap()
            .get(order.id)
            .unwrap()
            .is_some()
    );
    assert_eq!(
        read.open_table(META)
            .unwrap()
            .get("schema_version")
            .unwrap()
            .unwrap()
            .value(),
        99
    );
}

#[cfg(unix)]
#[tokio::test]
async fn new_database_and_backup_are_private_and_existing_permissions_are_preserved() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new().await;
    assert_eq!(
        std::fs::metadata(&f.path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let backup = f.directory.path().join("private.redb");
    f.store.backup_to(&backup).await.unwrap();
    assert_eq!(
        std::fs::metadata(&backup).unwrap().permissions().mode() & 0o777,
        0o600
    );
    f.store.close().await;
    std::fs::set_permissions(&f.path, std::fs::Permissions::from_mode(0o640)).unwrap();
    let store = OrderStore::open(&f.path).await.unwrap();
    assert_eq!(
        std::fs::metadata(&f.path).unwrap().permissions().mode() & 0o777,
        0o640
    );
    store.close().await;
}

#[tokio::test]
async fn database_generation_is_nonzero_stable_and_rotates_without_changing_records() {
    let f = Fixture::new().await;
    let first = f.store.generation().await.unwrap();
    assert_ne!(first, 0);
    assert_eq!(f.alice.key.generation, first);
    let order = f
        .store
        .create(&f.alice, draft("Keep through rotation"))
        .await
        .unwrap();
    let second = f.store.rotate_generation().await.unwrap();
    assert_ne!(second, 0);
    assert_ne!(second, first);
    let new_user = f
        .store
        .insert_user("team-a", "new-after-rotation", "New", "test-hash")
        .await
        .unwrap();
    assert_eq!(new_user.key.generation, second);
    f.store.close().await;
    let store = OrderStore::open(&f.path).await.unwrap();
    assert_eq!(store.generation().await.unwrap(), second);
    assert_eq!(
        store.load_by_id(&f.alice, order.id).await.unwrap(),
        Some(order)
    );
    let backup = f.directory.path().join("generation.redb");
    store.backup_to(&backup).await.unwrap();
    OrderStore::validate_backup(&backup).await.unwrap();
    let copy = OrderStore::open(&backup).await.unwrap();
    assert_eq!(copy.generation().await.unwrap(), second);
    assert_ne!(copy.rotate_generation().await.unwrap(), second);
    assert_eq!(store.generation().await.unwrap(), second);
}

#[tokio::test]
async fn migration_repairs_lagging_counters_and_schema_two_backups_upgrade_generation() {
    let f = Fixture::new().await;
    let order = f
        .store
        .create(&f.alice, draft("Existing schema two"))
        .await
        .unwrap();
    f.store.close().await;
    let database = Database::open(&f.path).unwrap();
    let tx = database.begin_write().unwrap();
    {
        let mut meta = tx.open_table(META).unwrap();
        meta.insert("schema_version", 2).unwrap();
        meta.remove("generation").unwrap();
    }
    tx.commit().unwrap();
    drop(database);
    OrderStore::validate_backup(&f.path).await.unwrap();
    let database = Database::open(&f.path).unwrap();
    let tx = database.begin_write().unwrap();
    for counter in ["last_order_id", "last_user_id", "last_tenant_id"] {
        tx.open_table(META).unwrap().insert(counter, 0).unwrap();
    }
    tx.commit().unwrap();
    drop(database);
    let upgraded = OrderStore::open(&f.path).await.unwrap();
    assert_ne!(upgraded.generation().await.unwrap(), 0);
    assert_eq!(
        upgraded.load_by_id(&f.alice, order.id).await.unwrap(),
        Some(order)
    );
    assert_eq!(
        upgraded
            .create(&f.alice, draft("Next ID"))
            .await
            .unwrap()
            .id,
        2
    );
    let user = upgraded
        .insert_user("team-c", "charlie", "Charlie", "test-hash")
        .await
        .unwrap();
    assert_eq!((user.key.user_id, user.key.tenant_id), (3, 3));
}

#[tokio::test]
async fn backup_validation_rejects_invalid_fields_catalogue_and_generation() {
    const USERS: TableDefinition<i64, &[u8]> = TableDefinition::new("users");
    const USER_NAMES: TableDefinition<&str, i64> = TableDefinition::new("users_by_username");
    const TENANTS: TableDefinition<i64, &str> = TableDefinition::new("tenants");
    const TENANT_NAMES: TableDefinition<&str, i64> = TableDefinition::new("tenants_by_name");
    let f = Fixture::new().await;
    let order = f
        .store
        .create(&f.alice, draft("Valid customer"))
        .await
        .unwrap();
    for case in [
        "empty-customer",
        "long-customer",
        "catalogue-id",
        "catalogue-name",
        "catalogue-price",
        "username",
        "display-name",
        "tenant-name",
        "generation",
    ] {
        let backup = f.directory.path().join(format!("invalid-{case}.redb"));
        f.store.backup_to(&backup).await.unwrap();
        let database = Database::open(&backup).unwrap();
        let tx = database.begin_write().unwrap();
        match case {
            "username" | "display-name" => {
                let mut user: serde_json::Value = serde_json::from_slice(
                    tx.open_table(USERS)
                        .unwrap()
                        .get(f.alice.key.user_id)
                        .unwrap()
                        .unwrap()
                        .value(),
                )
                .unwrap();
                if case == "username" {
                    tx.open_table(USER_NAMES).unwrap().remove("alice").unwrap();
                    tx.open_table(USER_NAMES)
                        .unwrap()
                        .insert("", f.alice.key.user_id)
                        .unwrap();
                    user["username"] = "".into();
                } else {
                    user["display_name"] = "x".repeat(129).into();
                }
                let bytes = serde_json::to_vec(&user).unwrap();
                tx.open_table(USERS)
                    .unwrap()
                    .insert(f.alice.key.user_id, bytes.as_slice())
                    .unwrap();
            }
            "tenant-name" => {
                tx.open_table(TENANT_NAMES)
                    .unwrap()
                    .remove("team-a")
                    .unwrap();
                tx.open_table(TENANT_NAMES)
                    .unwrap()
                    .insert("", f.alice.key.tenant_id)
                    .unwrap();
                tx.open_table(TENANTS)
                    .unwrap()
                    .insert(f.alice.key.tenant_id, "")
                    .unwrap();
            }
            "generation" => {
                tx.open_table(META)
                    .unwrap()
                    .insert("generation", 0)
                    .unwrap();
            }
            _ => {
                let mut row: serde_json::Value = serde_json::from_slice(
                    tx.open_table(ORDERS)
                        .unwrap()
                        .get(order.id)
                        .unwrap()
                        .unwrap()
                        .value(),
                )
                .unwrap();
                match case {
                    "empty-customer" => row["order"]["customer"] = "  ".into(),
                    "long-customer" => row["order"]["customer"] = "x".repeat(81).into(),
                    "catalogue-id" => row["order"]["items"][0]["id"] = 9.into(),
                    "catalogue-name" => row["order"]["items"][0]["name"] = "Counterfeit".into(),
                    "catalogue-price" => row["order"]["items"][0]["unit_price_cents"] = 1.into(),
                    _ => unreachable!(),
                }
                let bytes = serde_json::to_vec(&row).unwrap();
                tx.open_table(ORDERS)
                    .unwrap()
                    .insert(order.id, bytes.as_slice())
                    .unwrap();
            }
        }
        tx.commit().unwrap();
        drop(database);
        assert!(
            OrderStore::validate_backup(&backup).await.is_err(),
            "accepted {case}"
        );
    }
}
