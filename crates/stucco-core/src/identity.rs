//! Id generation and duplicate detection (spec §4.3).

use std::collections::{HashMap, HashSet};

/// Per-render identity state: the namespace, per-prefix counters and every id
/// emitted so far.
#[derive(Debug, Default)]
pub(crate) struct Identity {
    pub(crate) namespace: String,
    counters: HashMap<String, u32>,
    claimed: HashSet<String>,
}

impl Identity {
    /// `"{prefix}-{n}"`, or `"{namespace}-{prefix}-{n}"` when namespaced.
    pub(crate) fn generate(&mut self, prefix: &str) -> String {
        let n = self.counters.entry(prefix.to_owned()).or_insert(0);
        *n += 1;
        let id = if self.namespace.is_empty() {
            format!("{prefix}-{n}")
        } else {
            format!("{}-{prefix}-{n}", self.namespace)
        };
        self.claim(&id);
        id
    }

    /// Records `id`; duplicates panic in debug builds and are kept in release.
    pub(crate) fn claim(&mut self, id: &str) {
        let fresh = self.claimed.insert(id.to_owned());
        debug_assert!(fresh, "duplicate id: {id}");
    }
}
