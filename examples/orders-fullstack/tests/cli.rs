#![cfg(feature = "ssr")]
use orders_fullstack::{auth::principal_for_username, sessions::Sessions, store::OrderStore};
use std::{
    collections::{BTreeSet, HashMap},
    ffi::OsString,
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};
use tower_sessions::{
    SessionStore,
    session::{Id, Record},
};
fn cli(database: &Path, args: &[&str], password: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_orders-fullstack"));
    command
        .arg("--database")
        .arg(database)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if password.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = command.spawn().unwrap();
    if let Some(password) = password {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(password.as_bytes())
            .unwrap();
    }
    child.wait_with_output().unwrap()
}
fn success(output: Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn directory_entries(path: &Path) -> BTreeSet<OsString> {
    std::fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect()
}
#[tokio::test]
async fn default_nosql_provision_backup_restore_and_session_revocation_use_real_cli() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("original.redb");
    let backup = dir.path().join("backup.redb");
    let restored = dir.path().join("restored.redb");
    success(cli(
        &source,
        &[
            "user-add",
            "--tenant",
            "team",
            "--username",
            "alice",
            "--display-name",
            "Alice",
        ],
        Some("test-password-123\n"),
    ));
    success(cli(&source, &["seed", "--username", "alice"], None));
    let original = OrderStore::open(&source).await.unwrap();
    let principal = principal_for_username(&original, "alice").await.unwrap();
    let order = original.load_by_id(&principal, 1).await.unwrap().unwrap();
    let sessions = Sessions::new(&original).await.unwrap();
    let mut record = Record {
        id: Id::default(),
        data: HashMap::new(),
        expiry_date: time::OffsetDateTime::now_utc() + time::Duration::hours(1),
    };
    record.data.insert(
        "native_recovery".into(),
        serde_json::json!("secret draft fixture"),
    );
    sessions.create(&mut record).await.unwrap();
    // redb maintenance is offline and must refuse a file held by the server.
    assert!(
        !cli(
            &source,
            &["backup", "--destination", backup.to_str().unwrap()],
            None
        )
        .status
        .success()
    );
    drop(sessions);
    original.close().await;
    success(cli(
        &source,
        &["backup", "--destination", backup.to_str().unwrap()],
        None,
    ));
    let backup_bytes = std::fs::read(&backup).unwrap();
    assert!(
        !cli(
            &source,
            &["backup", "--destination", backup.to_str().unwrap()],
            None
        )
        .status
        .success()
    );
    success(cli(
        &source,
        &["sessions-revoke", "--username", "alice"],
        None,
    ));
    let original = OrderStore::open(&source).await.unwrap();
    assert_eq!(
        original
            .find_user_by_username("alice")
            .await
            .unwrap()
            .unwrap()
            .session_epoch,
        1
    );
    original.close().await;
    success(cli(
        &restored,
        &["restore", "--source", backup.to_str().unwrap()],
        None,
    ));
    assert_eq!(std::fs::read(&backup).unwrap(), backup_bytes);
    assert_eq!(directory_entries(dir.path()).len(), 3);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&restored).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    assert!(
        !cli(
            &restored,
            &["restore", "--source", backup.to_str().unwrap()],
            None
        )
        .status
        .success()
    );
    let restored_store = OrderStore::open(&restored).await.unwrap();
    let restored_principal = principal_for_username(&restored_store, "alice")
        .await
        .unwrap();
    assert_eq!(restored_principal.key.user_id, principal.key.user_id);
    assert_eq!(restored_principal.key.tenant_id, principal.key.tenant_id);
    assert_ne!(restored_principal.key.generation, principal.key.generation);
    assert_eq!(
        restored_store
            .load_by_id(&restored_principal, 1)
            .await
            .unwrap()
            .unwrap(),
        order
    );
    assert_eq!(
        restored_store
            .list(&restored_principal, "".into(), 1)
            .await
            .unwrap()
            .total_count,
        12
    );
    let sessions = Sessions::new(&restored_store).await.unwrap();
    assert!(sessions.load(&record.id).await.unwrap().is_none());
    drop(sessions);
    restored_store.close().await;
}

#[tokio::test]
async fn restore_rejects_invalid_copies_without_publishing_or_changing_the_source() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("invalid.redb");
    let destination = dir.path().join("restored.redb");
    // A valid redb file with an unsupported application schema reaches copied
    // snapshot validation, but must never be migrated or left at the final path.
    let store = OrderStore::open(&source).await.unwrap();
    store.close().await;
    let database = redb::Database::open(&source).unwrap();
    let write = database.begin_write().unwrap();
    write
        .open_table(redb::TableDefinition::<&str, u64>::new("metadata"))
        .unwrap()
        .insert("schema_version", u64::MAX)
        .unwrap();
    write.commit().unwrap();
    drop(database);
    let before = std::fs::read(&source).unwrap();
    let entries = directory_entries(dir.path());
    let output = cli(
        &destination,
        &["restore", "--source", source.to_str().unwrap()],
        None,
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Unsupported backup schema"));
    assert!(!destination.exists());
    assert_eq!(std::fs::read(&source).unwrap(), before);
    assert_eq!(directory_entries(dir.path()), entries);

    std::fs::write(&source, b"not a database").unwrap();
    assert!(
        !cli(
            &destination,
            &["restore", "--source", source.to_str().unwrap()],
            None,
        )
        .status
        .success()
    );
    assert_eq!(std::fs::read(&source).unwrap(), b"not a database");
    assert_eq!(directory_entries(dir.path()), entries);
}

#[tokio::test]
async fn restore_refuses_locked_sources_and_preserves_existing_destination_entries() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.redb");
    let destination = dir.path().join("restored.redb");
    let store = OrderStore::open(&source).await.unwrap();
    let entries = directory_entries(dir.path());
    assert!(
        !cli(
            &destination,
            &["restore", "--source", source.to_str().unwrap()],
            None,
        )
        .status
        .success()
    );
    assert_eq!(directory_entries(dir.path()), entries);
    store.close().await;
    let before = std::fs::read(&source).unwrap();

    for suffix in ["", "-wal", "-shm", "-journal"] {
        let occupied = dir.path().join(format!("restored.redb{suffix}"));
        std::fs::write(&occupied, b"existing destination data").unwrap();
        let entries = directory_entries(dir.path());
        let output = cli(
            &destination,
            &["restore", "--source", source.to_str().unwrap()],
            None,
        );
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("destination or sidecars"));
        assert_eq!(
            std::fs::read(&occupied).unwrap(),
            b"existing destination data"
        );
        assert_eq!(std::fs::read(&source).unwrap(), before);
        assert_eq!(directory_entries(dir.path()), entries);
        std::fs::remove_file(occupied).unwrap();
    }

    #[cfg(unix)]
    {
        // Path::exists would miss dangling symlinks, including sidecars.
        let sidecar = dir.path().join("restored.redb-wal");
        let target = dir.path().join("missing-target");
        std::os::unix::fs::symlink(&target, &sidecar).unwrap();
        let entries = directory_entries(dir.path());
        assert!(
            !cli(
                &destination,
                &["restore", "--source", source.to_str().unwrap()],
                None,
            )
            .status
            .success()
        );
        assert_eq!(std::fs::read_link(sidecar).unwrap(), target);
        assert_eq!(directory_entries(dir.path()), entries);
    }
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn sqlite_restore_prepares_a_copy_and_cleans_up_when_preparation_fails() {
    use sqlx::{
        Connection, SqliteConnection,
        sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    };

    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.sqlite3");
    let destination = dir.path().join("restored.sqlite3");
    success(cli(
        &source,
        &[
            "--backend",
            "sqlite",
            "user-add",
            "--tenant",
            "team",
            "--username",
            "alice",
            "--display-name",
            "Alice",
        ],
        Some("test-password-123\n"),
    ));
    success(cli(
        &source,
        &["--backend", "sqlite", "seed", "--username", "alice"],
        None,
    ));
    let store = OrderStore::open_sqlite(&source).await.unwrap();
    let original_principal = principal_for_username(&store, "alice").await.unwrap();
    let sessions = Sessions::new(&store).await.unwrap();
    let mut record = Record {
        id: Id::default(),
        data: HashMap::new(),
        expiry_date: time::OffsetDateTime::now_utc() + time::Duration::hours(1),
    };
    sessions.create(&mut record).await.unwrap();
    drop(sessions);
    store.close().await;
    // A restore source is an offline, self-contained snapshot. Let SQLite
    // checkpoint this fixture instead of discarding its committed WAL bytes.
    SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(&source)
            .journal_mode(SqliteJournalMode::Delete),
    )
    .await
    .unwrap()
    .close()
    .await
    .unwrap();
    assert_eq!(
        directory_entries(dir.path()),
        BTreeSet::from([OsString::from("source.sqlite3")]),
        "the SQLite source must be self-contained before restore starts",
    );
    let before = std::fs::read(&source).unwrap();
    success(cli(
        &destination,
        &[
            "--backend",
            "sqlite",
            "restore",
            "--source",
            source.to_str().unwrap(),
        ],
        None,
    ));
    assert_eq!(std::fs::read(&source).unwrap(), before);
    assert_eq!(directory_entries(dir.path()).len(), 2);
    let restored = OrderStore::open_sqlite(&destination).await.unwrap();
    let principal = principal_for_username(&restored, "alice").await.unwrap();
    assert_ne!(principal.key.generation, original_principal.key.generation);
    assert_eq!(
        restored
            .list(&principal, "".into(), 1)
            .await
            .unwrap()
            .total_count,
        12
    );
    let sessions = Sessions::new(&restored).await.unwrap();
    assert!(sessions.load(&record.id).await.unwrap().is_none());
    drop(sessions);
    restored.close().await;

    // Both fixtures pass the read-only snapshot checks. One fails during
    // migrations; the other fails when clearing sessions after opening succeeds.
    // Neither failure may publish a database or leave staging files/sidecars.
    for (name, corruption, expected_error) in [
        (
            "migration",
            "DELETE FROM _sqlx_migrations WHERE version = 2",
            "already exists",
        ),
        (
            "sessions",
            "CREATE TRIGGER reject_session_clear BEFORE DELETE ON tower_sessions BEGIN SELECT RAISE(ABORT, 'fixture session clear failed'); END",
            "fixture session clear failed",
        ),
    ] {
        let fixture = dir.path().join(format!("{name}.sqlite3"));
        let rejected = dir.path().join(format!("{name}-restored.sqlite3"));
        std::fs::copy(&source, &fixture).unwrap();
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&fixture)
                    .journal_mode(SqliteJournalMode::Delete),
            )
            .await
            .unwrap();
        sqlx::query(corruption).execute(&pool).await.unwrap();
        pool.close().await;
        OrderStore::validate_sqlite_backup(&fixture).await.unwrap();
        let before = std::fs::read(&fixture).unwrap();
        let entries = directory_entries(dir.path());
        let output = cli(
            &rejected,
            &[
                "--backend",
                "sqlite",
                "restore",
                "--source",
                fixture.to_str().unwrap(),
            ],
            None,
        );
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected_error),
            "{}",
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(!rejected.exists());
        assert_eq!(std::fs::read(&fixture).unwrap(), before);
        assert_eq!(directory_entries(dir.path()), entries);
    }
}

#[test]
fn wrong_paths_and_snapshot_sidecars_cannot_create_silent_empty_backups() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("typo.redb");
    let backup = dir.path().join("backup.redb");
    for command in [
        vec!["backup", "--destination", backup.to_str().unwrap()],
        vec!["serve"],
        vec!["seed", "--username", "alice"],
        vec!["sessions-revoke", "--username", "alice"],
    ] {
        assert!(!cli(&missing, &command, None).status.success());
        assert!(!missing.exists());
        assert!(!backup.exists());
    }
    std::fs::write(dir.path().join("backup.redb-wal"), "uncheckpointed").unwrap();
    let reply = cli(
        &missing,
        &["restore", "--source", backup.to_str().unwrap()],
        None,
    );
    assert!(!reply.status.success());
    assert!(String::from_utf8_lossy(&reply.stderr).contains("sidecars"));
    assert!(!missing.exists());
}
