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
    /// `"{prefix}-{n}"`, or `"{namespace}-{prefix}-{n}"` when namespaced. Ids
    /// are claimed when emitted (`Attrs::render`), not when generated.
    ///
    /// Prefixes must match `[a-z0-9]+`: with no hyphen in the prefix, the id
    /// parses uniquely, so different namespaces can never produce the same id.
    /// Invalid characters panic in debug builds and become `_` in release.
    pub(crate) fn generate(&mut self, prefix: &str) -> String {
        let valid = !prefix.is_empty()
            && prefix
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
        debug_assert!(valid, "invalid id prefix: {prefix:?} (use [a-z0-9]+)");
        let sanitised: String;
        let prefix = if valid {
            prefix
        } else {
            sanitised = prefix
                .chars()
                .map(|c| {
                    if c.is_ascii_lowercase() || c.is_ascii_digit() {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            &sanitised
        };
        let n = self.counters.entry(prefix.to_owned()).or_insert(0);
        *n += 1;
        if self.namespace.is_empty() {
            format!("{prefix}-{n}")
        } else {
            format!("{}-{prefix}-{n}", self.namespace)
        }
    }

    /// Records `id`; duplicates panic in debug builds and are kept in release.
    pub(crate) fn claim(&mut self, id: &str) {
        let fresh = self.claimed.insert(id.to_owned());
        debug_assert!(fresh, "duplicate id: {id}");
    }
}
