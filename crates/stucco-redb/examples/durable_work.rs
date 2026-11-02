//! cargo run -p stucco-redb --features migrations,jobs --example durable_work
use stucco_redb::{JobQueue, Migration, Migrations, PostcardCodec, Store, StoreError, U64Key};

fn main() -> Result<(), StoreError> {
    let dir = tempfile::tempdir().map_err(StoreError::new)?;
    let store = Store::open(dir.path().join("example.redb"))?;
    let records = store.table::<u64, String, _, _>("records", U64Key, PostcardCodec::default())?;
    Migrations::open(&store, "migrations")?.run(&[Migration::new(
        1,
        "initial-records-v1",
        |tx| tx.put(&records, &1, &"created".to_owned()),
    )])?;
    let jobs = JobQueue::<String>::open(&store, "jobs")?;
    store.write(|tx| {
        tx.put(&records, &1, &"queued".to_owned())?;
        jobs.enqueue(
            tx,
            "tenant-1:operation-1",
            "do idempotent work".into(),
            100,
            3.try_into().unwrap(),
        )?;
        Ok::<_, StoreError>(())
    })?;
    let lease = store
        .write(|tx| jobs.claim(tx, 100, 10.try_into().unwrap()))?
        .unwrap();
    // Do idempotent external I/O here, outside the writer lock. A lease cannot
    // fence those effects: application retries/reconciliation must handle them.
    println!("{}: {}", lease.key(), lease.payload());
    store.write(|tx| {
        if !jobs.acknowledge(tx, &lease, 101)? {
            return Err(StoreError::new(std::io::Error::other("stale job attempt")));
        }
        tx.put(&records, &1, &"finished".to_owned())?;
        Ok::<_, StoreError>(())
    })?;
    Ok(())
}
