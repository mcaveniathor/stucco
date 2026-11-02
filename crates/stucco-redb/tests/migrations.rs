#![cfg(feature = "migrations")]
use stucco_redb::{Migration, Migrations, PostcardCodec, Store, StoreError, U64Key};
fn failed() -> StoreError {
    StoreError::new(std::io::Error::other("injected failure"))
}

#[test]
fn pending_steps_and_history_roll_back_together_then_retry_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    {
        let store = Store::open(&path).unwrap();
        let data = store
            .table::<u64, String, _, _>("data", U64Key, PostcardCodec::default())
            .unwrap();
        let migrations = Migrations::open(&store, "migrations").unwrap();
        let failed_steps = [
            Migration::new(1, "create-v1", |tx| tx.put(&data, &1, &"first".into())),
            Migration::new(2, "transform-v1", |tx| {
                tx.put(&data, &2, &"second".into())?;
                Err(failed())
            }),
        ];
        assert!(migrations.run(&failed_steps).is_err());
        assert_eq!(migrations.version().unwrap(), 0);
        assert_eq!(data.get(&1).unwrap(), None);
        assert_eq!(data.get(&2).unwrap(), None);
        let steps = [
            Migration::new(1, "create-v1", |tx| tx.put(&data, &1, &"first".into())),
            Migration::new(2, "transform-v1", |tx| tx.put(&data, &2, &"second".into())),
        ];
        assert_eq!(migrations.run(&steps).unwrap(), 2);
        assert_eq!(migrations.run(&steps).unwrap(), 0);
    }
    let store = Store::open(&path).unwrap();
    let migrations = Migrations::open(&store, "migrations").unwrap();
    assert_eq!(migrations.version().unwrap(), 2);
    let already = [
        Migration::new(1, "create-v1", |_| panic!("must not replay")),
        Migration::new(2, "transform-v1", |_| panic!("must not replay")),
    ];
    assert_eq!(migrations.run(&already).unwrap(), 0);
    let data = store
        .table::<u64, String, _, _>("data", U64Key, PostcardCodec::default())
        .unwrap();
    assert_eq!(data.get(&2).unwrap().as_deref(), Some("second"));
}

#[test]
fn future_version_history_drift_and_invalid_plans_never_run_callbacks() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("db")).unwrap();
    let migrations = Migrations::open(&store, "migrations").unwrap();
    migrations
        .run(&[
            Migration::new(1, "original", |_| Ok(())),
            Migration::new(2, "second", |_| Ok(())),
        ])
        .unwrap();
    assert!(
        migrations
            .run(&[Migration::new(1, "original", |_| panic!())])
            .is_err()
    );
    assert!(
        migrations
            .run(&[
                Migration::new(1, "changed", |_| panic!()),
                Migration::new(2, "second", |_| panic!())
            ])
            .is_err()
    );
    for (version, id) in [(0, "zero"), (2, "gap"), (1, "")] {
        assert!(
            migrations
                .run(&[Migration::new(version, id, |_| panic!())])
                .is_err()
        );
    }
    assert_eq!(migrations.version().unwrap(), 2);
}

#[test]
fn panicking_step_rolls_back_and_releases_writer() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("db")).unwrap();
    let data = store
        .table::<u64, String, _, _>("data", U64Key, PostcardCodec::default())
        .unwrap();
    let migrations = Migrations::open(&store, "migrations").unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        migrations.run(&[Migration::new(1, "panic", |tx| {
            tx.put(&data, &1, &"uncommitted".into())?;
            panic!("injected panic");
        })])
    }));
    assert!(result.is_err());
    assert_eq!(migrations.version().unwrap(), 0);
    assert_eq!(data.get(&1).unwrap(), None);
    assert_eq!(
        migrations
            .run(&[Migration::new(1, "panic", |_| Ok(()))])
            .unwrap(),
        1
    );
}

#[test]
fn concurrent_startup_only_applies_once() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("db")).unwrap();
    let migrations = Migrations::open(&store, "migrations").unwrap();
    let count = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|threads| {
        for _ in 0..4 {
            let (migrations, count) = (&migrations, &count);
            threads.spawn(move || {
                migrations
                    .run(&[Migration::new(1, "once", |_| {
                        count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        Ok(())
                    })])
                    .unwrap()
            });
        }
    });
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);
}
