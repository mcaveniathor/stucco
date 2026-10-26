#![cfg(feature = "sqlite")]

use orders_fullstack::{
    auth::{Principal, PrincipalKey},
    domain::{DeleteError, Field, LoadError, OrderDraft, SaveError, new_order},
    sqlite_store::SqliteStore as OrderStore,
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tempfile::TempDir;

struct Fixture {
    directory: TempDir,
    store: OrderStore,
    alice: Principal,
    bob: Principal,
}

async fn principal(store: &OrderStore, name: &str) -> Principal {
    let tenant_id = sqlx::query("INSERT INTO tenants (name) VALUES (?)")
        .bind(name)
        .execute(store.pool())
        .await
        .unwrap()
        .last_insert_rowid();
    let user_id = sqlx::query(
        "INSERT INTO users (tenant_id, username, display_name, password_hash) VALUES (?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(name)
    .bind(name)
    .bind("storage-test-fixture-not-a-login-hash")
    .execute(store.pool())
    .await
    .unwrap()
    .last_insert_rowid();
    Principal {
        key: PrincipalKey {
            user_id,
            tenant_id,
            generation: 1,
        },
        display_name: name.into(),
    }
}

impl Fixture {
    async fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let store = OrderStore::open(&directory.path().join("orders.sqlite"))
            .await
            .unwrap();
        let alice = principal(&store, "Alice").await;
        let bob = principal(&store, "Bob").await;
        Self {
            directory,
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

#[cfg(unix)]
#[tokio::test]
async fn new_database_and_wal_are_private_and_existing_permissions_are_unchanged() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("private.sqlite");
    let store = OrderStore::open(&path).await.unwrap();
    for path in [&path, &directory.path().join("private.sqlite-wal")] {
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    store.pool().close().await;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
    let store = OrderStore::open(&path).await.unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
    store.pool().close().await;
}

#[tokio::test]
async fn optional_sqlite_facade_preserves_identity_and_application_service_contract() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("optional.sqlite");
    let store = orders_fullstack::store::OrderStore::open_sqlite(&path)
        .await
        .unwrap();
    let alice = store
        .insert_user("team-a", "alice", "Alice", "test-only-hash")
        .await
        .unwrap();
    assert!(
        store
            .insert_user("must-roll-back", "alice", "Duplicate", "hash")
            .await
            .is_err()
    );
    let bob = store
        .insert_user("team-b", "bob", "Bob", "test-only-hash")
        .await
        .unwrap();
    assert_eq!((bob.key.user_id, bob.key.tenant_id), (2, 2));
    let first = store
        .create(&alice, draft("Via public facade"))
        .await
        .unwrap();
    assert_eq!(store.load_by_id(&bob, first.id).await.unwrap(), None);
    let saved = store
        .save_by_id(&alice, first.id, first.draft())
        .await
        .unwrap();
    assert_eq!(saved.revision, 2);
    assert!(store.revoke_sessions("alice").await.unwrap());
    assert!(store.set_user_enabled("alice", false).await.unwrap());
    let user = store.find_user_by_username("alice").await.unwrap().unwrap();
    assert_eq!(user.session_epoch, 2);
    assert!(!user.enabled);
    assert_eq!(
        store
            .find_user_by_id(alice.key.user_id)
            .await
            .unwrap()
            .unwrap()
            .username,
        "alice"
    );
    assert!(!store.revoke_sessions("missing").await.unwrap());
    store.close().await;
    let reopened = orders_fullstack::store::OrderStore::open_sqlite(&path)
        .await
        .unwrap();
    assert_eq!(
        reopened.load_by_id(&alice, saved.id).await.unwrap(),
        Some(saved)
    );
    reopened.close().await;
}

#[tokio::test]
async fn opening_is_empty_and_explicit_seeding_never_overwrites_tenant_records() {
    let f = Fixture::new().await;
    assert_eq!(
        f.store
            .list(&f.alice, String::new(), 1)
            .await
            .unwrap()
            .total_count,
        0
    );
    f.store.seed_orders(&f.alice).await.unwrap();
    let ada = f.store.load_by_id(&f.alice, 1).await.unwrap().unwrap();
    assert_eq!(ada.customer, "Ada Lovelace");
    assert_eq!(ada.total_cents(), Ok(5_900));
    let mut changed = ada.draft();
    changed.customer = "Keep this edit".into();
    let saved = f.store.save_by_id(&f.alice, ada.id, changed).await.unwrap();
    f.store.seed_orders(&f.alice).await.unwrap();
    assert_eq!(
        f.store.load_by_id(&f.alice, ada.id).await.unwrap(),
        Some(saved)
    );
    assert_eq!(
        f.store
            .list(&f.alice, String::new(), 1)
            .await
            .unwrap()
            .total_count,
        12
    );
    assert_eq!(
        f.store
            .list(&f.bob, String::new(), 1)
            .await
            .unwrap()
            .total_count,
        0
    );
    f.store.seed_orders(&f.bob).await.unwrap();
    assert_eq!(
        f.store
            .list(&f.bob, String::new(), 1)
            .await
            .unwrap()
            .total_count,
        12
    );
    f.store.pool().close().await;
}

#[tokio::test]
async fn search_and_pagination_preserve_boundaries_unicode_and_literal_substrings() {
    let f = Fixture::new().await;
    f.store.seed_orders(&f.alice).await.unwrap();
    let first = f.store.list(&f.alice, String::new(), 0).await.unwrap();
    assert_eq!(
        (first.page, first.total_pages, first.total_count),
        (1, 3, 12)
    );
    assert_eq!(
        first
            .orders
            .iter()
            .map(|order| order.id)
            .collect::<Vec<_>>(),
        [12, 11, 10, 9, 8]
    );
    let last = f
        .store
        .list(&f.alice, String::new(), u32::MAX)
        .await
        .unwrap();
    assert_eq!((last.page, last.total_pages), (3, 3));
    assert_eq!(
        last.orders.iter().map(|order| order.id).collect::<Vec<_>>(),
        [2, 1]
    );
    let found = f
        .store
        .list(&f.alice, "  hOpPeR  ".into(), 8)
        .await
        .unwrap();
    assert_eq!(found.query, "hOpPeR");
    assert_eq!(
        (found.page, found.total_pages, found.total_count),
        (1, 1, 1)
    );
    f.store
        .create(&f.alice, draft("Élodie _100% O'Neil"))
        .await
        .unwrap();
    for query in ["éLO", "_100%", "O'Neil"] {
        assert_eq!(
            f.store
                .list(&f.alice, query.into(), 1)
                .await
                .unwrap()
                .total_count,
            1
        );
    }
    let empty = f
        .store
        .list(&f.alice, "' OR 1=1 --".into(), u32::MAX)
        .await
        .unwrap();
    assert_eq!(
        (empty.page, empty.total_pages, empty.total_count),
        (1, 1, 0)
    );
    assert!(empty.orders.is_empty());
    f.store.pool().close().await;
}

#[tokio::test]
async fn validation_conflict_and_deletion_preserve_the_original_submitted_text() {
    let f = Fixture::new().await;
    let mut invalid = draft("  New customer  ");
    invalid.quantity_one = "not a quantity".into();
    let SaveError::Validation {
        draft: retained, ..
    } = f.store.create(&f.alice, invalid.clone()).await.unwrap_err()
    else {
        panic!("validation expected")
    };
    assert_eq!(retained, invalid);
    invalid.quantity_one = "3".into();
    invalid.expected_revision = "2".into();
    let SaveError::Validation {
        draft: retained,
        errors,
    } = f.store.create(&f.alice, invalid.clone()).await.unwrap_err()
    else {
        panic!("revision validation expected")
    };
    assert_eq!(retained, invalid);
    assert_eq!(errors[0].field, Field::Revision);
    invalid.expected_revision = "1".into();
    let original = f.store.create(&f.alice, invalid).await.unwrap();
    assert_eq!(original.id, 1);
    assert_eq!(original.customer, "New customer");
    let mut edited = original.draft();
    edited.quantity_two = "2".into();
    let saved = f
        .store
        .save_by_id(&f.alice, original.id, edited)
        .await
        .unwrap();
    assert_eq!(saved.revision, 2);
    assert_eq!(
        saved.items[0].unit_price_cents,
        original.items[0].unit_price_cents
    );
    let stale = original.draft();
    assert_eq!(
        f.store
            .save_by_id(&f.alice, original.id, stale.clone())
            .await,
        Err(SaveError::Conflict {
            draft: stale.clone(),
            current: Box::new(saved.clone())
        })
    );
    assert_eq!(
        f.store.delete(&f.alice, original.id, " 01 ".into()).await,
        Err(DeleteError::Conflict {
            expected_revision: " 01 ".into(),
            current: Box::new(saved)
        })
    );
    let DeleteError::Validation {
        expected_revision, ..
    } = f
        .store
        .delete(&f.alice, original.id, " bad ".into())
        .await
        .unwrap_err()
    else {
        panic!("validation expected")
    };
    assert_eq!(expected_revision, " bad ");
    f.store
        .delete(&f.alice, original.id, "2".into())
        .await
        .unwrap();
    assert_eq!(
        f.store.load_by_id(&f.alice, original.id).await.unwrap(),
        None
    );
    assert_eq!(
        f.store
            .save_by_id(&f.alice, original.id, stale.clone())
            .await,
        Err(SaveError::NotFound { draft: stale })
    );
    assert_eq!(
        f.store.delete(&f.alice, original.id, "2".into()).await,
        Err(DeleteError::NotFound {
            expected_revision: "2".into()
        })
    );
    let line_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM order_lines")
        .fetch_one(f.store.pool())
        .await
        .unwrap();
    assert_eq!(line_count, 0);
    f.store.pool().close().await;
}

#[tokio::test]
async fn another_tenants_id_never_exposes_a_snapshot_or_accepts_a_write() {
    let f = Fixture::new().await;
    let alice = f
        .store
        .create(&f.alice, draft("Alice private customer"))
        .await
        .unwrap();
    let bob = f
        .store
        .create(&f.bob, draft("Bob private customer"))
        .await
        .unwrap();
    assert_eq!(f.store.load_by_id(&f.alice, bob.id).await.unwrap(), None);
    assert_eq!(f.store.load_by_id(&f.bob, alice.id).await.unwrap(), None);
    let forged = alice.draft();
    assert_eq!(
        f.store.save_by_id(&f.bob, alice.id, forged.clone()).await,
        Err(SaveError::NotFound { draft: forged })
    );
    assert_eq!(
        f.store.delete(&f.bob, alice.id, "1".into()).await,
        Err(DeleteError::NotFound {
            expected_revision: "1".into()
        })
    );
    assert_eq!(
        f.store
            .list(&f.bob, "Alice".into(), 1)
            .await
            .unwrap()
            .total_count,
        0
    );
    assert_eq!(
        f.store
            .list(&f.alice, String::new(), 1)
            .await
            .unwrap()
            .orders,
        [alice]
    );
    assert_eq!(
        f.store.list(&f.bob, String::new(), 1).await.unwrap().orders,
        [bob]
    );
    f.store.pool().close().await;
}

#[tokio::test]
async fn two_pools_allow_only_one_save_for_the_same_revision() {
    let f = Fixture::new().await;
    let second = OrderStore::open(&f.directory.path().join("orders.sqlite"))
        .await
        .unwrap();
    let order = f.store.create(&f.alice, draft("Concurrent")).await.unwrap();
    let mut one = order.draft();
    one.customer = "First".into();
    let mut two = order.draft();
    two.customer = "Second".into();
    let (one, two) = tokio::join!(
        f.store.save_by_id(&f.alice, order.id, one),
        second.save_by_id(&f.alice, order.id, two),
    );
    match (one, two) {
        (Ok(saved), Err(SaveError::Conflict { current, .. }))
        | (Err(SaveError::Conflict { current, .. }), Ok(saved)) => {
            assert_eq!(*current, saved);
            assert_eq!(saved.revision, 2);
            assert_eq!(
                f.store.load_by_id(&f.alice, order.id).await.unwrap(),
                Some(saved)
            );
        }
        results => panic!("one save and one conflict required: {results:?}"),
    }
    second.pool().close().await;
    f.store.pool().close().await;
}

#[tokio::test]
async fn concurrent_edit_and_delete_cannot_both_use_the_same_revision() {
    let f = Fixture::new().await;
    let order = f.store.create(&f.alice, draft("Concurrent")).await.unwrap();
    let (saved, deleted) = tokio::join!(
        f.store.save_by_id(&f.alice, order.id, order.draft()),
        f.store.delete(&f.alice, order.id, "1".into()),
    );
    match (saved, deleted) {
        (Ok(saved), Err(DeleteError::Conflict { current, .. })) => assert_eq!(saved, *current),
        (Err(SaveError::NotFound { .. }), Ok(())) => {
            assert_eq!(f.store.load_by_id(&f.alice, order.id).await.unwrap(), None)
        }
        results => panic!("one mutation must win: {results:?}"),
    }
    f.store.pool().close().await;
}

#[tokio::test]
async fn concurrent_creates_are_distinct_and_deleted_ids_stay_used_after_restart() {
    let f = Fixture::new().await;
    let mut tasks = tokio::task::JoinSet::new();
    for index in 0..8 {
        let store = f.store.clone();
        let principal = f.alice.clone();
        tasks.spawn(async move {
            store
                .create(&principal, draft(&format!("Concurrent {index}")))
                .await
                .unwrap()
        });
    }
    let mut ids = Vec::new();
    while let Some(order) = tasks.join_next().await {
        let order = order.unwrap();
        assert_eq!(
            f.store.load_by_id(&f.alice, order.id).await.unwrap(),
            Some(order.clone())
        );
        ids.push(order.id);
    }
    ids.sort_unstable();
    assert_eq!(ids, (1..=8).collect::<Vec<_>>());
    f.store.delete(&f.alice, 8, "1".into()).await.unwrap();
    f.store.pool().close().await;
    let reopened = OrderStore::open(&f.directory.path().join("orders.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        reopened
            .list(&f.alice, String::new(), 1)
            .await
            .unwrap()
            .total_count,
        7
    );
    assert_eq!(
        reopened
            .create(&f.alice, draft("After restart"))
            .await
            .unwrap()
            .id,
        9
    );
    reopened.pool().close().await;
}

#[tokio::test]
async fn failed_line_writes_roll_back_the_order_and_all_earlier_line_writes() {
    let f = Fixture::new().await;
    let original = f.store.create(&f.alice, draft("Original")).await.unwrap();
    sqlx::raw_sql("CREATE TRIGGER reject_second_update BEFORE UPDATE OF quantity ON order_lines WHEN NEW.slot = 1 BEGIN SELECT RAISE(ABORT, 'test rollback'); END;")
        .execute(f.store.pool()).await.unwrap();
    let mut changed = original.draft();
    changed.customer = "Must roll back".into();
    changed.quantity_one = "3".into();
    assert_eq!(
        f.store
            .save_by_id(&f.alice, original.id, changed.clone())
            .await,
        Err(SaveError::Unavailable { draft: changed })
    );
    assert_eq!(
        f.store.load_by_id(&f.alice, original.id).await.unwrap(),
        Some(original)
    );
    sqlx::raw_sql("CREATE TRIGGER reject_second_insert BEFORE INSERT ON order_lines WHEN NEW.slot = 1 BEGIN SELECT RAISE(ABORT, 'test rollback'); END;")
        .execute(f.store.pool()).await.unwrap();
    let failed = draft("Failed creation");
    assert_eq!(
        f.store.create(&f.alice, failed.clone()).await,
        Err(SaveError::Unavailable { draft: failed })
    );
    assert_eq!(
        f.store
            .list(&f.alice, String::new(), 1)
            .await
            .unwrap()
            .total_count,
        1
    );
    sqlx::query("DROP TRIGGER reject_second_insert")
        .execute(f.store.pool())
        .await
        .unwrap();
    assert_eq!(
        f.store
            .create(&f.alice, draft("Next creation"))
            .await
            .unwrap()
            .id,
        2
    );
    f.store.pool().close().await;
}

#[tokio::test]
async fn integer_and_total_bounds_never_wrap_or_partially_save() {
    let f = Fixture::new().await;
    let original = f.store.create(&f.alice, draft("Bounds")).await.unwrap();
    assert_eq!(f.store.load_by_id(&f.alice, u64::MAX).await.unwrap(), None);
    assert_eq!(f.store.load_by_id(&f.alice, 0).await.unwrap(), None);
    let mut impossible = original.draft();
    impossible.expected_revision = u64::MAX.to_string();
    assert!(matches!(
        f.store.save_by_id(&f.alice, original.id, impossible).await,
        Err(SaveError::Conflict { .. })
    ));
    sqlx::query("UPDATE orders SET revision = ? WHERE id = ?")
        .bind(i64::MAX)
        .bind(original.id as i64)
        .execute(f.store.pool())
        .await
        .unwrap();
    let maximal = f
        .store
        .load_by_id(&f.alice, original.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        f.store
            .save_by_id(&f.alice, original.id, maximal.draft())
            .await,
        Err(SaveError::Unavailable {
            draft: maximal.draft()
        })
    );
    assert_eq!(
        f.store.load_by_id(&f.alice, original.id).await.unwrap(),
        Some(maximal)
    );
    sqlx::query("UPDATE orders SET revision = 1 WHERE id = ?")
        .bind(original.id as i64)
        .execute(f.store.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE order_lines SET unit_price_cents = ? WHERE order_id = ? AND slot = 0")
        .bind(i64::MAX)
        .bind(original.id as i64)
        .execute(f.store.pool())
        .await
        .unwrap();
    let before = f
        .store
        .load_by_id(&f.alice, original.id)
        .await
        .unwrap()
        .unwrap();
    let mut overflow = before.draft();
    overflow.quantity_one = "3".into();
    assert!(matches!(
        f.store.save_by_id(&f.alice, original.id, overflow).await,
        Err(SaveError::Validation { .. })
    ));
    assert_eq!(
        f.store.load_by_id(&f.alice, original.id).await.unwrap(),
        Some(before)
    );
    sqlx::query("UPDATE sqlite_sequence SET seq = ? WHERE name = 'orders'")
        .bind(i64::MAX)
        .execute(f.store.pool())
        .await
        .unwrap();
    let exhausted = draft("No ID available");
    assert_eq!(
        f.store.create(&f.alice, exhausted.clone()).await,
        Err(SaveError::Unavailable {
            draft: exhausted.clone()
        })
    );
    f.store
        .delete(&f.alice, original.id, "1".into())
        .await
        .unwrap();
    assert_eq!(
        f.store.create(&f.alice, exhausted.clone()).await,
        Err(SaveError::Unavailable { draft: exhausted })
    );
    f.store.pool().close().await;
}

#[tokio::test]
async fn a_closed_pool_returns_public_unavailable_errors_and_retains_inputs() {
    let f = Fixture::new().await;
    f.store.health().await.unwrap();
    f.store.pool().close().await;
    assert!(f.store.health().await.is_err());
    assert_eq!(f.store.load_by_id(&f.alice, 1).await, Err(LoadError));
    assert_eq!(
        f.store.list(&f.alice, String::new(), 1).await,
        Err(LoadError)
    );
    let draft = draft("Unsent");
    assert_eq!(
        f.store.create(&f.alice, draft.clone()).await,
        Err(SaveError::Unavailable {
            draft: draft.clone()
        })
    );
    assert_eq!(
        f.store.save_by_id(&f.alice, 1, draft.clone()).await,
        Err(SaveError::Unavailable { draft })
    );
    assert_eq!(
        f.store.delete(&f.alice, 1, " 1 ".into()).await,
        Err(DeleteError::Unavailable {
            expected_revision: " 1 ".into()
        })
    );
}

#[tokio::test]
async fn backup_scrubs_session_rows_and_deleted_bytes_without_revoking_live_sessions() {
    let f = Fixture::new().await;
    let order = f
        .store
        .create(&f.alice, draft("Keep this order"))
        .await
        .unwrap();
    let sessions = tower_sessions_sqlx_store::SqliteStore::new(f.store.pool().clone());
    sessions.migrate().await.unwrap();
    let bearer = "unique-backup-test-bearer-credential";
    let recovery = "unique-backup-test-private-native-recovery".repeat(200);
    sqlx::query("INSERT INTO tower_sessions(id,data,expiry_date) VALUES (?, ?, ?)")
        .bind(bearer)
        .bind(recovery.as_bytes())
        .bind(time::OffsetDateTime::now_utc() + time::Duration::hours(1))
        .execute(f.store.pool())
        .await
        .unwrap();
    let backup = f.directory.path().join("no-sessions.sqlite");
    f.store.backup_to(&backup).await.unwrap();
    let bytes = std::fs::read(&backup).unwrap();
    for secret in [bearer, "unique-backup-test-private-native-recovery"] {
        assert!(
            !bytes
                .windows(secret.len())
                .any(|window| window == secret.as_bytes())
        );
    }
    for suffix in ["-wal", "-shm", "-journal"] {
        assert!(
            !f.directory
                .path()
                .join(format!("no-sessions.sqlite{suffix}"))
                .exists()
        );
    }
    let live_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tower_sessions")
        .fetch_one(f.store.pool())
        .await
        .unwrap();
    assert_eq!(live_count, 1);
    OrderStore::validate_backup(&backup).await.unwrap();
    let copy = OrderStore::open(&backup).await.unwrap();
    let copy_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tower_sessions")
        .fetch_one(copy.pool())
        .await
        .unwrap();
    assert_eq!(copy_count, 0);
    assert_eq!(
        copy.load_by_id(&f.alice, order.id).await.unwrap(),
        Some(order)
    );
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(copy.pool())
        .await
        .unwrap();
    assert_eq!(users, 2);
    copy.pool().close().await;
    f.store.pool().close().await;
}

#[tokio::test]
async fn backup_restores_a_consistent_snapshot_with_revisions_and_id_history() {
    let f = Fixture::new().await;
    let first = f.store.create(&f.alice, draft("Snapshot")).await.unwrap();
    let mut edit = first.draft();
    edit.quantity_one = "3".into();
    let snapshot = f.store.save_by_id(&f.alice, first.id, edit).await.unwrap();
    let deleted = f
        .store
        .create(&f.alice, draft("Deleted before backup"))
        .await
        .unwrap();
    f.store
        .delete(&f.alice, deleted.id, "1".into())
        .await
        .unwrap();
    let bob = f.store.create(&f.bob, draft("Bob snapshot")).await.unwrap();
    let backup = f.directory.path().join("snapshot.sqlite");
    f.store.backup_to(&backup).await.unwrap();
    OrderStore::validate_backup(&backup).await.unwrap();
    assert!(f.store.backup_to(&backup).await.is_err());
    let empty = f.directory.path().join("existing-empty.sqlite");
    std::fs::write(&empty, []).unwrap();
    assert!(f.store.backup_to(&empty).await.is_err());
    f.store
        .create(&f.alice, draft("After backup"))
        .await
        .unwrap();
    f.store.pool().close().await;
    let restore = f.directory.path().join("restored.sqlite");
    std::fs::copy(&backup, &restore).unwrap();
    OrderStore::validate_backup(&restore).await.unwrap();
    let restored = OrderStore::open(&restore).await.unwrap();
    assert_eq!(
        restored.load_by_id(&f.alice, first.id).await.unwrap(),
        Some(snapshot)
    );
    assert_eq!(
        restored.load_by_id(&f.bob, bob.id).await.unwrap(),
        Some(bob.clone())
    );
    assert_eq!(restored.load_by_id(&f.alice, bob.id).await.unwrap(), None);
    assert_eq!(
        restored
            .list(&f.alice, "After backup".into(), 1)
            .await
            .unwrap()
            .total_count,
        0
    );
    assert_eq!(
        restored
            .create(&f.alice, draft("After restore"))
            .await
            .unwrap()
            .id,
        bob.id + 1
    );
    restored.pool().close().await;
}

#[tokio::test]
async fn upgrading_the_initial_schema_preserves_records_and_adds_indexes() {
    let directory = tempfile::tempdir().unwrap();
    let migrations = directory.path().join("old-migrations");
    std::fs::create_dir(&migrations).unwrap();
    std::fs::write(
        migrations.join("0001_initial.sql"),
        include_str!("../migrations/0001_initial.sql"),
    )
    .unwrap();
    let path = directory.path().join("upgrade.sqlite");
    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(migrations.as_path())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    let old = OrderStore::from_pool(pool);
    let alice = principal(&old, "Upgrade Alice").await;
    let first = old.create(&alice, draft("Before upgrade")).await.unwrap();
    let before = old
        .save_by_id(&alice, first.id, first.draft())
        .await
        .unwrap();
    old.pool().close().await;
    let upgraded = OrderStore::open(&path).await.unwrap();
    assert_eq!(
        upgraded.load_by_id(&alice, before.id).await.unwrap(),
        Some(before)
    );
    let versions: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
            .fetch_all(upgraded.pool())
            .await
            .unwrap();
    assert_eq!(versions, [1, 2, 3, 4]);
    let indexes: Vec<String> = sqlx::query_scalar("SELECT name FROM sqlite_schema WHERE type = 'index' AND name LIKE 'orders_tenant_%' ORDER BY name").fetch_all(upgraded.pool()).await.unwrap();
    assert_eq!(
        indexes,
        ["orders_tenant_customer_search", "orders_tenant_id_desc"]
    );
    upgraded.pool().close().await;
    let again = OrderStore::open(&path).await.unwrap();
    assert_eq!(
        again
            .list(&alice, String::new(), 1)
            .await
            .unwrap()
            .total_count,
        1
    );
    again.pool().close().await;
}

#[tokio::test]
async fn restore_validation_rejects_corrupt_files_and_changed_migration_history() {
    let f = Fixture::new().await;
    let corrupt = f.directory.path().join("corrupt.sqlite");
    std::fs::write(&corrupt, b"not a database").unwrap();
    assert!(OrderStore::validate_backup(&corrupt).await.is_err());
    sqlx::query("UPDATE _sqlx_migrations SET checksum = X'00' WHERE version = 1")
        .execute(f.store.pool())
        .await
        .unwrap();
    let backup = f.directory.path().join("incompatible.sqlite");
    f.store.backup_to(&backup).await.unwrap();
    assert!(OrderStore::validate_backup(&backup).await.is_err());
    f.store.pool().close().await;
}

#[tokio::test]
async fn sqlite_generation_uses_full_u64_range_persists_and_rotates() {
    let f = Fixture::new().await;
    let first = f.store.generation().await.unwrap();
    assert_ne!(first, 0);
    let created = f
        .store
        .insert_user(
            "generation-team",
            "generation-user",
            "Generation",
            "test-hash",
        )
        .await
        .unwrap();
    assert_eq!(created.key.generation, first);
    sqlx::query("UPDATE database_metadata SET value = ? WHERE name = 'generation'")
        .bind(u64::MAX.to_string())
        .execute(f.store.pool())
        .await
        .unwrap();
    assert_eq!(f.store.generation().await.unwrap(), u64::MAX);
    let second = f.store.rotate_generation().await.unwrap();
    assert_ne!(second, 0);
    assert_ne!(second, u64::MAX);
    let backup = f.directory.path().join("generation.sqlite");
    f.store.backup_to(&backup).await.unwrap();
    OrderStore::validate_backup(&backup).await.unwrap();
    f.store.close().await;
    let reopened = OrderStore::open(&f.directory.path().join("orders.sqlite"))
        .await
        .unwrap();
    assert_eq!(reopened.generation().await.unwrap(), second);
    let copy = OrderStore::open(&backup).await.unwrap();
    assert_eq!(copy.generation().await.unwrap(), second);
    assert_ne!(copy.rotate_generation().await.unwrap(), second);
    assert_eq!(reopened.generation().await.unwrap(), second);
    reopened.close().await;
    copy.close().await;
}

#[tokio::test]
async fn sqlite_backup_validation_rejects_field_catalogue_and_generation_corruption() {
    let f = Fixture::new().await;
    f.store
        .create(&f.alice, draft("Valid customer"))
        .await
        .unwrap();
    for (index, statement) in [
        "UPDATE orders SET customer = '   '",
        "UPDATE orders SET customer = replace(hex(zeroblob(41)), '0', 'x')",
        "UPDATE order_lines SET catalog_id = 9 WHERE slot = 0",
        "UPDATE order_lines SET name = 'Counterfeit' WHERE slot = 0",
        "UPDATE order_lines SET unit_price_cents = 1 WHERE slot = 0",
        "UPDATE users SET username = '' WHERE id = 1",
        "UPDATE users SET display_name = '' WHERE id = 1",
        "UPDATE tenants SET name = '' WHERE id = 1",
        "UPDATE database_metadata SET value = '0' WHERE name = 'generation'",
    ]
    .into_iter()
    .enumerate()
    {
        let backup = f
            .directory
            .path()
            .join(format!("invalid-field-{index}.sqlite"));
        f.store.backup_to(&backup).await.unwrap();
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().filename(&backup))
            .await
            .unwrap();
        sqlx::query(statement).execute(&pool).await.unwrap();
        pool.close().await;
        assert!(
            OrderStore::validate_backup(&backup).await.is_err(),
            "accepted {statement}"
        );
    }
    f.store.close().await;
}
