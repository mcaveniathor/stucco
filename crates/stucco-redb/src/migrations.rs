//! Explicit application migrations executed before serving traffic.
use crate::{PostcardCodec, Store, StoreError, Table, U64Key, WriteTx};

type Apply<'a> = Box<dyn Fn(&mut WriteTx) -> Result<(), StoreError> + 'a>;

/// One versioned application transformation. Versions start at one and are
/// contiguous. Identity is an immutable revision/checksum chosen by the author.
///
/// Open all table handles before running migrations. The callback must only use
/// its supplied transaction: opening tables or nested writes would deadlock.
/// Record codecs, index changes and authorization remain application concerns.
pub struct Migration<'a> {
    version: u64,
    identity: &'a str,
    apply: Apply<'a>,
}

impl<'a> Migration<'a> {
    /// Defines a step. Changing the identity of an already-applied step is
    /// rejected; use a new version instead. Callbacks must do no external I/O.
    pub fn new(
        version: u64,
        identity: &'a str,
        apply: impl Fn(&mut WriteTx) -> Result<(), StoreError> + 'a,
    ) -> Self {
        Self {
            version,
            identity,
            apply: Box::new(apply),
        }
    }
}

/// Opt-in migrate-on-open coordinator, requiring the `migrations` feature.
///
/// Call [`run`](Self::run) during startup, before exposing application handles.
/// All pending steps and their history commit in one transaction; error/panic
/// rolls everything back. No data is decoded or transformed automatically.
/// Retain the full immutable migration list in every newer application build.
#[derive(Clone, Debug)]
pub struct Migrations {
    store: Store,
    history: Table<u64, Vec<String>>,
}

impl Migrations {
    /// Opens a dedicated history table. Do not share its name with app tables.
    pub fn open(store: &Store, name: &str) -> Result<Self, StoreError> {
        Ok(Self {
            store: store.clone(),
            history: store.table(name, U64Key, PostcardCodec::default())?,
        })
    }

    /// Applies pending contiguous steps, returning their count. Rejects future
    /// histories, changed identities and invalid plans before invoking callbacks.
    /// Work is synchronous and holds the writer lock: run off an async executor.
    pub fn run(&self, steps: &[Migration<'_>]) -> Result<usize, StoreError> {
        for (index, step) in steps.iter().enumerate() {
            if step.version != index as u64 + 1 || step.identity.is_empty() {
                return Err(StoreError::invalid(
                    "migration versions must be contiguous from one with nonempty identities",
                ));
            }
        }
        self.store.write(|tx| {
            let mut history = tx.get(&self.history, &0)?.unwrap_or_default();
            if history.len() > steps.len() {
                return Err(StoreError::invalid(
                    "database migration version is newer than this build",
                ));
            }
            if history
                .iter()
                .zip(steps)
                .any(|(saved, step)| saved != step.identity)
            {
                return Err(StoreError::invalid("applied migration identity changed"));
            }
            let count = steps.len() - history.len();
            for step in &steps[history.len()..] {
                (step.apply)(tx)?;
                history.push(step.identity.to_owned());
            }
            if count != 0 {
                tx.put(&self.history, &0, &history)?;
            }
            Ok(count)
        })
    }

    /// The number of committed steps. Reading this does not apply migrations.
    pub fn version(&self) -> Result<u64, StoreError> {
        Ok(self
            .history
            .get(&0)?
            .map_or(0, |history| history.len() as u64))
    }
}
