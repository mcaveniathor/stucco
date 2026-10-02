use crate::{Codec, IndexTable, KeyCodec, PostcardCodec, ScanRequest, U64Key};
use std::sync::Arc;
use stucco_core::{Capabilities, CollectionPage, CollectionQuery, ColumnSpec};
use stucco_tower::{CollectionSource, RequestContext, SourceError};
/// Maps an application's query semantics onto physical ordering and predicates.
pub trait CollectionMapping<V>: Send + Sync + 'static {
    /// Supported operations.
    fn capabilities(&self) -> Capabilities;
    /// Column filter declarations.
    fn columns(&self) -> Vec<ColumnSpec>;
    /// Physical order and cursor scope.
    fn scan_request(&self, q: &CollectionQuery) -> ScanRequest;
    /// Tests one record. This runs on the blocking pool.
    fn matches(&self, value: &V, q: &CollectionQuery) -> bool;
}
/// A bounded blocking-pool source. Started redb scans cannot be forcibly
/// cancelled; their semaphore permit remains held until the scan exits.
pub struct RedbCollection<K, V, KC = U64Key, VC = PostcardCodec<V>, M = ()> {
    table: IndexTable<K, V, KC, VC>,
    mapping: Arc<M>,
    permits: Arc<tokio::sync::Semaphore>,
}
impl<K, V, KC, VC, M> Clone for RedbCollection<K, V, KC, VC, M> {
    fn clone(&self) -> Self {
        Self {
            table: self.table.clone(),
            mapping: self.mapping.clone(),
            permits: self.permits.clone(),
        }
    }
}
impl<K, V, KC, VC, M> RedbCollection<K, V, KC, VC, M> {
    /// Creates a source with at most four running scans.
    pub fn new(table: IndexTable<K, V, KC, VC>, mapping: M) -> Self {
        Self {
            table,
            mapping: Arc::new(mapping),
            permits: Arc::new(tokio::sync::Semaphore::new(4)),
        }
    }
    /// Sets maximum concurrency before sharing the source. Zero means one.
    pub fn concurrency(mut self, limit: usize) -> Self {
        self.permits = Arc::new(tokio::sync::Semaphore::new(limit.max(1)));
        self
    }
}
impl<K: 'static, V: Send + 'static, KC: KeyCodec<K>, VC: Codec<V>, M: CollectionMapping<V>>
    CollectionSource for RedbCollection<K, V, KC, VC, M>
{
    type Row = V;
    fn capabilities(&self) -> Capabilities {
        self.mapping.capabilities()
    }
    async fn query(
        &self,
        q: &CollectionQuery,
        _: &RequestContext,
    ) -> Result<CollectionPage<V>, SourceError> {
        let permit = self
            .permits
            .clone()
            .acquire_owned()
            .await
            .map_err(SourceError::new)?;
        let table = self.table.clone();
        let mapping = self.mapping.clone();
        let q = q.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            table.scan(&mapping.scan_request(&q), |v| mapping.matches(v, &q))
        })
        .await
        .map_err(SourceError::new)?
        .map_err(SourceError::new)
    }
}
