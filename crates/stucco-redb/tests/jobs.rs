#![cfg(feature = "jobs")]
use stucco_redb::{JobQueue, JobState, PostcardCodec, Store, StoreError, U64Key};
fn failed() -> StoreError {
    StoreError::new(std::io::Error::other("injected failure"))
}

#[test]
fn enqueue_and_acknowledgement_commit_with_application_effects() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("db")).unwrap();
    let app = store
        .table::<u64, String, _, _>("app", U64Key, PostcardCodec::default())
        .unwrap();
    let queue = JobQueue::<String>::open(&store, "jobs").unwrap();
    let aborted: Result<(), StoreError> = store.write(|tx| {
        queue.enqueue(tx, "h1:1", "payload".into(), 10, 3.try_into().unwrap())?;
        tx.put(&app, &1, &"queued".into())?;
        Err(failed())
    });
    assert!(aborted.is_err());
    assert!(store.write(|tx| queue.get(tx, "h1:1")).unwrap().is_none());
    assert!(app.get(&1).unwrap().is_none());
    store
        .write(|tx| queue.enqueue(tx, "h1:1", "payload".into(), 10, 3.try_into().unwrap()))
        .unwrap();
    let lease = store
        .write(|tx| queue.claim(tx, 10, 10.try_into().unwrap()))
        .unwrap()
        .unwrap();
    let aborted: Result<(), StoreError> = store.write(|tx| {
        assert!(queue.acknowledge(tx, &lease, 11)?);
        tx.put(&app, &1, &"done".into())?;
        Err(failed())
    });
    assert!(aborted.is_err());
    assert!(matches!(
        store
            .write(|tx| queue.get(tx, "h1:1"))
            .unwrap()
            .unwrap()
            .state,
        JobState::Leased { .. }
    ));
    store
        .write(|tx| {
            assert!(queue.acknowledge(tx, &lease, 11)?);
            tx.put(&app, &1, &"done".into())
        })
        .unwrap();
    assert!(!store.write(|tx| queue.acknowledge(tx, &lease, 11)).unwrap());
    assert!(
        !store
            .write(|tx| queue.enqueue(tx, "h1:1", "replacement".into(), 12, 1.try_into().unwrap()))
            .unwrap()
    );
    let job = store.write(|tx| queue.get(tx, "h1:1")).unwrap().unwrap();
    assert_eq!(job.state, JobState::Done);
    assert_eq!(job.payload, "payload");
    assert_eq!(app.get(&1).unwrap().as_deref(), Some("done"));
}

#[test]
fn expired_lease_is_fenced_at_boundary_before_and_after_reclaim() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("db")).unwrap();
    let queue = JobQueue::<u64>::open(&store, "jobs").unwrap();
    store
        .write(|tx| queue.enqueue(tx, "a", 7, 10, 3.try_into().unwrap()))
        .unwrap();
    assert!(
        store
            .write(|tx| queue.claim(tx, 9, 5.try_into().unwrap()))
            .unwrap()
            .is_none()
    );
    let old = store
        .write(|tx| queue.claim(tx, 10, 5.try_into().unwrap()))
        .unwrap()
        .unwrap();
    assert!(
        store
            .write(|tx| queue.claim(tx, 14, 5.try_into().unwrap()))
            .unwrap()
            .is_none()
    );
    assert!(!store.write(|tx| queue.acknowledge(tx, &old, 15)).unwrap());
    assert!(!store.write(|tx| queue.retry(tx, &old, 15, 20)).unwrap());
    let new = store
        .write(|tx| queue.claim(tx, 15, 5.try_into().unwrap()))
        .unwrap()
        .unwrap();
    assert_eq!(new.attempt(), 2);
    assert_eq!(*new.payload(), 7);
    assert_eq!(new.until(), 20);
    assert!(!store.write(|tx| queue.acknowledge(tx, &old, 16)).unwrap());
    assert!(!store.write(|tx| queue.retry(tx, &old, 16, 19)).unwrap());
    assert!(store.write(|tx| queue.acknowledge(tx, &new, 16)).unwrap());
}

#[test]
fn retry_schedule_and_attempt_budget_survive_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    {
        let store = Store::open(&path).unwrap();
        let queue = JobQueue::<String>::open(&store, "jobs").unwrap();
        store
            .write(|tx| queue.enqueue(tx, "a", "payload".into(), 0, 2.try_into().unwrap()))
            .unwrap();
        let lease = store
            .write(|tx| queue.claim(tx, 0, 5.try_into().unwrap()))
            .unwrap()
            .unwrap();
        assert!(store.write(|tx| queue.retry(tx, &lease, 1, 0)).is_err());
        assert!(store.write(|tx| queue.retry(tx, &lease, 1, 10)).unwrap());
    }
    let store = Store::open(&path).unwrap();
    let queue = JobQueue::<String>::open(&store, "jobs").unwrap();
    assert!(
        store
            .write(|tx| queue.claim(tx, 9, 5.try_into().unwrap()))
            .unwrap()
            .is_none()
    );
    let lease = store
        .write(|tx| queue.claim(tx, 10, 5.try_into().unwrap()))
        .unwrap()
        .unwrap();
    assert_eq!(lease.attempt(), 2);
    assert!(store.write(|tx| queue.retry(tx, &lease, 11, 20)).unwrap());
    assert!(
        store
            .write(|tx| queue.claim(tx, 100, 5.try_into().unwrap()))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        store.write(|tx| queue.get(tx, "a")).unwrap().unwrap().state,
        JobState::Failed
    );
}

#[test]
fn crashed_final_attempt_exhausts_without_blocking_other_due_work() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("db")).unwrap();
    let queue = JobQueue::<u64>::open(&store, "jobs").unwrap();
    store
        .write(|tx| queue.enqueue(tx, "a", 1, 0, 1.try_into().unwrap()))
        .unwrap();
    store
        .write(|tx| queue.claim(tx, 0, 1.try_into().unwrap()))
        .unwrap()
        .unwrap();
    store
        .write(|tx| queue.enqueue(tx, "b", 2, 2, 1.try_into().unwrap()))
        .unwrap();
    let next = store
        .write(|tx| queue.claim(tx, 2, 1.try_into().unwrap()))
        .unwrap()
        .unwrap();
    assert_eq!(next.key(), "b");
    assert_eq!(
        store.write(|tx| queue.get(tx, "a")).unwrap().unwrap().state,
        JobState::Failed
    );
}

#[test]
fn duplicate_claimers_get_only_one_live_lease_and_rollback_preserves_attempt() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("db")).unwrap();
    let queue = JobQueue::<u64>::open(&store, "jobs").unwrap();
    store
        .write(|tx| queue.enqueue(tx, "a", 1, 0, 3.try_into().unwrap()))
        .unwrap();
    let aborted: Result<(), StoreError> = store.write(|tx| {
        assert!(queue.claim(tx, 0, 10.try_into().unwrap())?.is_some());
        Err(failed())
    });
    assert!(aborted.is_err());
    let claims = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|threads| {
        for _ in 0..4 {
            let (store, queue, claims) = (&store, &queue, &claims);
            threads.spawn(move || {
                if let Some(lease) = store
                    .write(|tx| queue.claim(tx, 0, 10.try_into().unwrap()))
                    .unwrap()
                {
                    assert_eq!(lease.attempt(), 1);
                    claims.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
            });
        }
    });
    assert_eq!(claims.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn leases_cannot_acknowledge_another_queue_or_database() {
    let dir = tempfile::tempdir().unwrap();
    let first = Store::open(dir.path().join("first")).unwrap();
    let second = Store::open(dir.path().join("second")).unwrap();
    let q1 = JobQueue::<u64>::open(&first, "jobs").unwrap();
    let q2 = JobQueue::<u64>::open(&first, "other").unwrap();
    let q3 = JobQueue::<u64>::open(&second, "jobs").unwrap();
    let mut leases = Vec::new();
    for (store, queue) in [(&first, &q1), (&first, &q2), (&second, &q3)] {
        store
            .write(|tx| queue.enqueue(tx, "same", 1, 0, 2.try_into().unwrap()))
            .unwrap();
        leases.push(
            store
                .write(|tx| queue.claim(tx, 0, 10.try_into().unwrap()))
                .unwrap()
                .unwrap(),
        );
    }
    assert!(!first.write(|tx| q2.acknowledge(tx, &leases[0], 1)).unwrap());
    assert!(
        !second
            .write(|tx| q3.acknowledge(tx, &leases[0], 1))
            .unwrap()
    );
    assert!(first.write(|tx| q1.acknowledge(tx, &leases[0], 1)).unwrap());
    assert!(first.write(|tx| q2.acknowledge(tx, &leases[1], 1)).unwrap());
    assert!(
        second
            .write(|tx| q3.acknowledge(tx, &leases[2], 1))
            .unwrap()
    );
}

#[test]
fn deadline_overflow_leaves_the_attempt_unclaimed() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("db")).unwrap();
    let queue = JobQueue::<u64>::open(&store, "jobs").unwrap();
    store
        .write(|tx| queue.enqueue(tx, "a", 1, u64::MAX, 1.try_into().unwrap()))
        .unwrap();
    assert!(
        store
            .write(|tx| queue.claim(tx, u64::MAX, 1.try_into().unwrap()))
            .is_err()
    );
    assert_eq!(
        store
            .write(|tx| queue.get(tx, "a"))
            .unwrap()
            .unwrap()
            .attempts,
        0
    );
}

#[test]
fn queue_names_cannot_alias_another_queues_due_table() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("db")).unwrap();
    let _queue = JobQueue::<String>::open(&store, "work").unwrap();
    assert!(JobQueue::<String>::open(&store, "work_due").is_err());
}
