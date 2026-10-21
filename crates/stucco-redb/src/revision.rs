use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use std::{fmt, str::FromStr};

/// A record's revision: a hash of its stored bytes.
///
/// Read one with `get_with_revision`, carry it in the edit form as a hidden
/// input, and pass it back to `update_if` or `remove_if`. A record that
/// changed in between is a conflict. Revisions detect concurrent edits; they
/// are not a security boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Revision([u8; 16]);

impl Revision {
    pub(crate) fn of(bytes: &[u8]) -> Revision {
        Revision(xxhash_rust::xxh3::xxh3_128(bytes).to_be_bytes())
    }
}

impl fmt::Display for Revision {
    /// 22 URL-safe characters.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&URL_SAFE_NO_PAD.encode(self.0))
    }
}

impl FromStr for Revision {
    type Err = InvalidRevision;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = URL_SAFE_NO_PAD.decode(s).map_err(|_| InvalidRevision)?;
        bytes.try_into().map(Revision).map_err(|_| InvalidRevision)
    }
}

/// A string that is not a revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidRevision;

impl fmt::Display for InvalidRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid revision")
    }
}

impl std::error::Error for InvalidRevision {}
