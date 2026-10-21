use crate::{AsyncTable, Codec, KeyCodec, PostcardCodec, ScanRequest, U64Key};
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
/// A collection source over an [`AsyncTable`]. Scans share the table's
/// concurrency limit with its writes. Started redb scans cannot be forcibly
/// cancelled; their permit remains held until the scan exits.
pub struct RedbCollection<K, V, KC = U64Key, VC = PostcardCodec<V>, M = ()> {
    table: AsyncTable<K, V, KC, VC>,
    mapping: Arc<M>,
}
impl<K, V, KC, VC, M> Clone for RedbCollection<K, V, KC, VC, M> {
    fn clone(&self) -> Self {
        Self {
            table: self.table.clone(),
            mapping: self.mapping.clone(),
        }
    }
}
impl<K, V, KC, VC, M> RedbCollection<K, V, KC, VC, M> {
    /// Creates a source over `table`, sharing its concurrency limit.
    pub fn new(table: AsyncTable<K, V, KC, VC>, mapping: M) -> Self {
        Self {
            table,
            mapping: Arc::new(mapping),
        }
    }
    /// Creates another query mapping over the same table and limit.
    pub fn with_mapping<N>(&self, mapping: N) -> RedbCollection<K, V, KC, VC, N> {
        RedbCollection {
            table: self.table.clone(),
            mapping: Arc::new(mapping),
        }
    }
    /// The table, for writes and single-record reads.
    pub fn table(&self) -> &AsyncTable<K, V, KC, VC> {
        &self.table
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
            .table
            .permits()
            .clone()
            .acquire_owned()
            .await
            .map_err(SourceError::new)?;
        let table = self.table.blocking().clone();
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
