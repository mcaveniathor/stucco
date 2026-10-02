use crate::{Codec, IndexTable, KeyCodec, StoreError};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use redb::ReadableDatabase;
use std::{
    hash::{Hash, Hasher},
    ops::Bound,
};
use stucco_core::{CollectionPage, Cursor, Direction, Window};

/// Scan configuration; offsets are an explicitly selected O(n) mode.
#[derive(Clone, Debug)]
pub struct ScanRequest {
    /// Secondary index name, or primary-key order.
    pub index: Option<String>,
    /// Display order.
    pub direction: Direction,
    /// Exclusive cursor or numbered position.
    pub window: Window,
    /// Number of matching rows, clamped to 1–100.
    pub per_page: u16,
    /// Canonical filter/search/sort/page-size identity, excluding position.
    pub scope: String,
    /// Whether to full-scan and compute totals for numbered pagination.
    pub offset_mode: bool,
}
impl ScanRequest {
    fn fingerprint(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.scope.hash(&mut h);
        self.per_page.hash(&mut h);
        h.finish()
    }
    fn encode(&self, table: &str, anchor: &[u8]) -> Result<Cursor, StoreError> {
        let bytes = postcard::to_allocvec(&(
            1_u8,
            table,
            self.index.as_deref(),
            self.direction == Direction::Desc,
            self.fingerprint(),
            anchor,
        ))
        .map_err(StoreError::new)?;
        Cursor::new(&URL_SAFE_NO_PAD.encode(bytes))
            .ok_or_else(|| StoreError::invalid("cursor key too large"))
    }
    fn anchor(&self, table: &str) -> Option<Vec<u8>> {
        let cursor = match &self.window {
            Window::After(c) | Window::Before(c) => c,
            _ => return None,
        };
        let bytes = URL_SAFE_NO_PAD.decode(cursor.as_str()).ok()?;
        let (version, t, index, desc, scope, anchor): (
            u8,
            String,
            Option<String>,
            bool,
            u64,
            Vec<u8>,
        ) = postcard::from_bytes(&bytes).ok()?;
        (version == 1
            && t == table
            && index == self.index
            && desc == (self.direction == Direction::Desc)
            && scope == self.fingerprint())
        .then_some(anchor)
    }
}
impl<K, V, KC: KeyCodec<K>, VC: Codec<V>> IndexTable<K, V, KC, VC> {
    /// Reads matching rows using exclusive physical ranges. Cursor mode
    /// materializes at most limit+1 matches; filters may examine many rows.
    /// Offset mode intentionally scans all matches.
    pub fn scan(
        &self,
        req: &ScanRequest,
        keep: impl Fn(&V) -> bool,
    ) -> Result<CollectionPage<V>, StoreError> {
        if req.index.as_ref().is_some_and(|i| i != &self.key) {
            return Err(StoreError::invalid("unknown index"));
        }
        let size = if req.per_page == 0 {
            25
        } else {
            usize::from(req.per_page.min(100))
        };
        if req.offset_mode {
            let mut all = self
                .all(req.index.is_some())?
                .into_iter()
                .filter(|(_, v)| keep(v))
                .collect::<Vec<_>>();
            if req.direction == Direction::Desc {
                all.reverse();
            }
            let total = all.len() as u64;
            let page = match req.window {
                Window::Offset { page } => page.max(1),
                _ => 1,
            };
            let start = page
                .saturating_sub(1)
                .checked_mul(size as u64)
                .and_then(|n| usize::try_from(n).ok())
                .unwrap_or(usize::MAX);
            let rows = all
                .into_iter()
                .skip(start)
                .take(size)
                .map(|(_, v)| v)
                .collect();
            return Ok(CollectionPage {
                rows,
                next: None,
                prev: None,
                total: Some(total),
            });
        }
        let tx = self.table.store.db.begin_read().map_err(StoreError::new)?;
        let data = tx
            .open_table(redb::TableDefinition::<&[u8], &[u8]>::new(&self.table.name))
            .map_err(StoreError::new)?;
        let order = tx
            .open_table(redb::TableDefinition::<&[u8], &[u8]>::new(
                if req.index.is_some() {
                    &self.name
                } else {
                    &self.table.name
                },
            ))
            .map_err(StoreError::new)?;
        let anchor = req.anchor(&self.table.name);
        let backwards = anchor.is_some() && matches!(req.window, Window::Before(_));
        let desc = (req.direction == Direction::Desc) ^ backwards;
        let matching = |anchor: Option<&[u8]>,
                        desc: bool,
                        limit: usize|
         -> Result<Vec<(Vec<u8>, V)>, StoreError> {
            let bounds = match anchor {
                Some(a) if desc => (Bound::Unbounded, Bound::Excluded(a)),
                Some(a) => (Bound::Excluded(a), Bound::Unbounded),
                None => (Bound::Unbounded, Bound::Unbounded),
            };
            let mut iter = order.range::<&[u8]>(bounds).map_err(StoreError::new)?;
            let mut rows = Vec::new();
            while let Some(entry) = if desc { iter.next_back() } else { iter.next() } {
                let (k, v) = entry.map_err(StoreError::new)?;
                let value = if req.index.is_some() {
                    let value = data
                        .get(v.value())
                        .map_err(StoreError::new)?
                        .ok_or_else(|| StoreError::invalid("index points to missing record"))?;
                    self.table.values.decode(value.value())?
                } else {
                    self.table.values.decode(v.value())?
                };
                if keep(&value) {
                    rows.push((k.value().to_vec(), value));
                    if rows.len() >= limit {
                        break;
                    }
                }
            }
            Ok(rows)
        };
        let mut selected = matching(anchor.as_deref(), desc, size + 1)?;
        selected.truncate(size);
        if backwards {
            selected.reverse();
        }
        let (prev, next) = if let (Some(first), Some(last)) = (selected.first(), selected.last()) {
            let display_desc = req.direction == Direction::Desc;
            let prev = if !matching(Some(&first.0), !display_desc, 1)?.is_empty() {
                Some(req.encode(&self.table.name, &first.0)?)
            } else {
                None
            };
            let next = if !matching(Some(&last.0), display_desc, 1)?.is_empty() {
                Some(req.encode(&self.table.name, &last.0)?)
            } else {
                None
            };
            (prev, next)
        } else {
            (None, None)
        };
        Ok(CollectionPage {
            rows: selected.into_iter().map(|(_, v)| v).collect(),
            next,
            prev,
            total: None,
        })
    }
}
