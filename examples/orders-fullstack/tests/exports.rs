#![cfg(feature = "ssr")]

use orders_fullstack::{
    auth::Principal,
    domain::new_order,
    exports::{
        ExportError, ExportFailure, ExportState, Exports, MAX_EXPORT_ATTEMPTS, MAX_EXPORT_ORDERS,
        clear_stored, new_token,
    },
    store::OrderStore,
};
use std::{path::PathBuf, time::Duration};
use tempfile::TempDir;

struct Fixture {
    _directory: TempDir,
    path: PathBuf,
    store: OrderStore,
    exports: Exports,
    alice: Principal,
    teammate: Principal,
    bob: Principal,
}
impl Fixture {
    async fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("orders.db");
        let store = open(&path).await;
        let alice = store
            .insert_user("team-a", "alice", "Alice", "test-only")
            .await
            .unwrap();
        let teammate = store
            .insert_user("team-a", "teammate", "Teammate", "test-only")
            .await
            .unwrap();
        let bob = store
            .insert_user("team-b", "bob", "Bob", "test-only")
            .await
            .unwrap();
        let exports = Exports::new(store.clone()).await.unwrap();
        Self {
            _directory: directory,
            path,
            store,
            exports,
            alice,
            teammate,
            bob,
        }
    }
    async fn finish(&self, token: &str) -> orders_fullstack::exports::ExportView {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let view = self.exports.status(&self.alice, token).await.unwrap();
                if !view.state.is_pending() {
                    return view;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("worker completed bounded export")
    }
    async fn order(&self, customer: &str) -> orders_fullstack::domain::Order {
        let mut draft = new_order().draft();
        draft.customer = customer.into();
        self.store.create(&self.alice, draft).await.unwrap()
    }
}
async fn open(path: &std::path::Path) -> OrderStore {
    #[cfg(feature = "sqlite")]
    {
        OrderStore::open_sqlite(path).await.unwrap()
    }
    #[cfg(not(feature = "sqlite"))]
    {
        OrderStore::open(path).await.unwrap()
    }
}

#[tokio::test]
async fn worker_exports_consistent_owned_snapshot_and_lost_response_retries_reuse_it() {
    let f = Fixture::new().await;
    f.store.seed_orders(&f.alice).await.unwrap();
    f.store.seed_orders(&f.bob).await.unwrap();
    let token = new_token().unwrap();
    let (first, duplicate) = tokio::join!(
        f.exports.start(&f.alice, token.clone()),
        f.exports.start(&f.alice, token.clone())
    );
    assert_eq!(first.unwrap().token, duplicate.unwrap().token);
    assert_eq!(
        f.finish(&token).await.state,
        ExportState::Complete { rows: 12 }
    );
    let csv = f.exports.download(&f.alice, &token).await.unwrap();
    let text = String::from_utf8(csv.clone()).unwrap();
    assert_eq!(text.lines().count(), 13);
    assert!(text.contains("12,1,"));
    assert!(!text.contains("24,1,"));
    f.order("Created after snapshot").await;
    let duplicate = f.exports.start(&f.alice, token.clone()).await.unwrap();
    assert_eq!(duplicate.attempt, 1);
    assert_eq!(duplicate.state, ExportState::Complete { rows: 12 });
    assert_eq!(f.exports.download(&f.alice, &token).await.unwrap(), csv);
    assert_eq!(f.exports.list(&f.alice).await.unwrap().len(), 1);
}

#[tokio::test]
async fn export_status_download_and_mutations_are_principal_scoped_even_within_tenant() {
    let f = Fixture::new().await;
    f.order("Alice private export").await;
    let token = new_token().unwrap();
    f.exports.start(&f.alice, token.clone()).await.unwrap();
    f.finish(&token).await;
    for other in [&f.bob, &f.teammate] {
        assert_eq!(
            f.exports.status(other, &token).await.unwrap_err(),
            ExportError::NotFound
        );
        assert_eq!(
            f.exports.download(other, &token).await.unwrap_err(),
            ExportError::NotFound
        );
        assert_eq!(
            f.exports.cancel(other, &token).await.unwrap_err(),
            ExportError::NotFound
        );
        assert_eq!(
            f.exports.retry(other, &token, 1).await.unwrap_err(),
            ExportError::NotFound
        );
        assert!(f.exports.list(other).await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn csv_escapes_delimiters_newlines_quotes_and_spreadsheet_formulas() {
    let f = Fixture::new().await;
    for customer in [
        "Plain, with \"quotes\"\nand a line",
        "=HYPERLINK(\"bad\")",
        "+command",
        "-command",
        "@command",
    ] {
        f.order(customer).await;
    }
    let token = new_token().unwrap();
    f.exports.start(&f.alice, token.clone()).await.unwrap();
    assert_eq!(
        f.finish(&token).await.state,
        ExportState::Complete { rows: 5 }
    );
    let csv = String::from_utf8(f.exports.download(&f.alice, &token).await.unwrap()).unwrap();
    assert!(csv.contains("\"Plain, with \"\"quotes\"\"\nand a line\""));
    assert!(csv.contains("\"'=HYPERLINK(\"\"bad\"\")\""));
    for prefix in ['+', '-', '@'] {
        assert!(csv.contains(&format!("\"'{prefix}command\"")));
    }
    assert!(csv.ends_with("\r\n"));
}

#[tokio::test]
async fn cancelled_export_removes_bytes_and_late_work_cannot_publish_results() {
    let f = Fixture::new().await;
    f.store.seed_orders(&f.alice).await.unwrap();
    let token = new_token().unwrap();
    f.exports.start(&f.alice, token.clone()).await.unwrap();
    assert_eq!(
        f.exports.cancel(&f.alice, &token).await.unwrap().state,
        ExportState::Cancelled
    );
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        f.exports.status(&f.alice, &token).await.unwrap().state,
        ExportState::Cancelled
    );
    assert_eq!(
        f.exports.download(&f.alice, &token).await.unwrap_err(),
        ExportError::NotFound
    );
    assert_eq!(
        f.exports.retry(&f.alice, &token, 1).await.unwrap_err(),
        ExportError::NotRetryable
    );
}

#[tokio::test]
async fn revocation_disable_and_database_history_changes_invalidate_results() {
    let f = Fixture::new().await;
    let token = new_token().unwrap();
    f.exports.start(&f.alice, token.clone()).await.unwrap();
    f.finish(&token).await;
    f.store.revoke_sessions("alice").await.unwrap();
    assert_eq!(
        f.exports.download(&f.alice, &token).await.unwrap_err(),
        ExportError::NotFound
    );
    assert!(f.exports.list(&f.alice).await.unwrap().is_empty());
    let second = new_token().unwrap();
    f.exports.start(&f.alice, second.clone()).await.unwrap();
    f.finish(&second).await;
    f.store.set_user_enabled("alice", false).await.unwrap();
    assert_eq!(
        f.exports.download(&f.alice, &second).await.unwrap_err(),
        ExportError::NotFound
    );
    f.store.set_user_enabled("alice", true).await.unwrap();
    f.store.rotate_generation().await.unwrap();
    assert_eq!(
        f.exports
            .start(&f.alice, new_token().unwrap())
            .await
            .unwrap_err(),
        ExportError::NotFound
    );
}

#[tokio::test]
async fn completed_exports_survive_restart_and_offline_restore_can_scrub_them() {
    let f = Fixture::new().await;
    f.order("Persisted export").await;
    let token = new_token().unwrap();
    f.exports.start(&f.alice, token.clone()).await.unwrap();
    f.finish(&token).await;
    let original = f.exports.download(&f.alice, &token).await.unwrap();
    f.exports.shutdown().await;
    drop(f.exports);
    f.store.close().await;
    let store = open(&f.path).await;
    let exports = Exports::new(store.clone()).await.unwrap();
    assert_eq!(exports.download(&f.alice, &token).await.unwrap(), original);
    exports.shutdown().await;
    drop(exports);
    clear_stored(&store).await.unwrap();
    let exports = Exports::new(store).await.unwrap();
    assert_eq!(
        exports.status(&f.alice, &token).await.unwrap_err(),
        ExportError::NotFound
    );
}

#[tokio::test]
async fn bounded_jobs_reject_bad_tokens_and_excess_per_user_capacity() {
    let f = Fixture::new().await;
    for token in [
        "",
        "../result",
        "a",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
    ] {
        assert_eq!(
            f.exports.start(&f.alice, token.into()).await.unwrap_err(),
            ExportError::InvalidToken
        );
    }
    for _ in 0..8 {
        f.exports
            .start(&f.alice, new_token().unwrap())
            .await
            .unwrap();
    }
    assert_eq!(
        f.exports
            .start(&f.alice, new_token().unwrap())
            .await
            .unwrap_err(),
        ExportError::Capacity
    );
    assert!(f.exports.start(&f.bob, new_token().unwrap()).await.is_ok());
}

#[tokio::test]
async fn cancelled_replay_records_keep_the_user_quota_without_blocking_other_tenants() {
    let f = Fixture::new().await;
    let mut tokens = Vec::new();
    for _ in 0..8 {
        let token = new_token().unwrap();
        f.exports.start(&f.alice, token.clone()).await.unwrap();
        f.exports.cancel(&f.alice, &token).await.unwrap();
        tokens.push(token);
    }
    assert_eq!(
        f.exports
            .start(&f.alice, new_token().unwrap())
            .await
            .unwrap_err(),
        ExportError::Capacity
    );
    assert_eq!(
        f.exports
            .start(&f.alice, tokens[0].clone())
            .await
            .unwrap()
            .state,
        ExportState::Cancelled
    );
    assert_eq!(f.exports.list(&f.alice).await.unwrap().len(), 8);
    assert!(f.exports.start(&f.bob, new_token().unwrap()).await.is_ok());
}

#[tokio::test]
async fn failed_attempts_wait_for_explicit_idempotent_retry_and_enforce_attempt_limit() {
    let f = Fixture::new().await;
    let mut last = None;
    for index in 0..=MAX_EXPORT_ORDERS {
        last = Some(f.order(&format!("Order {index}")).await);
    }
    let token = new_token().unwrap();
    f.exports.start(&f.alice, token.clone()).await.unwrap();
    assert_eq!(
        f.finish(&token).await.state,
        ExportState::Failed(ExportFailure::TooManyOrders)
    );
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(f.exports.status(&f.alice, &token).await.unwrap().attempt, 1);
    let (retry, repeated) = tokio::join!(
        f.exports.retry(&f.alice, &token, 1),
        f.exports.retry(&f.alice, &token, 1)
    );
    assert_eq!(retry.unwrap().attempt, 2);
    assert_eq!(repeated.unwrap().attempt, 2);
    assert_eq!(
        f.finish(&token).await.state,
        ExportState::Failed(ExportFailure::TooManyOrders)
    );
    f.exports.retry(&f.alice, &token, 2).await.unwrap();
    assert_eq!(f.finish(&token).await.attempt, MAX_EXPORT_ATTEMPTS);
    assert_eq!(
        f.exports.retry(&f.alice, &token, 3).await.unwrap_err(),
        ExportError::AttemptsExhausted
    );
    let last = last.unwrap();
    f.store
        .delete(&f.alice, last.id, last.revision.to_string())
        .await
        .unwrap();
    let next = new_token().unwrap();
    f.exports.start(&f.alice, next.clone()).await.unwrap();
    assert_eq!(
        f.finish(&next).await.state,
        ExportState::Complete {
            rows: MAX_EXPORT_ORDERS
        }
    );
}
