use std::fmt;

const MAX_KEY: usize = 512;

/// A validated blob key: `/`-separated segments of ASCII letters, digits, `.`,
/// `_` and `-`. No segment is empty or starts with a dot, so a key can never
/// climb out of a directory, whatever a backend does with it.
///
/// Build keys from your own ids, not from user input such as file names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlobKey(String);

impl BlobKey {
    /// Checks `key`.
    pub fn new(key: impl Into<String>) -> Result<Self, InvalidKey> {
        let key = key.into();
        let valid = !key.is_empty() && key.len() <= MAX_KEY && key.split('/').all(valid_segment);
        if valid {
            Ok(Self(key))
        } else {
            Err(InvalidKey)
        }
    }

    /// The key as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('/')
    }

    /// Whether `self` is `prefix` or sits below it.
    pub(crate) fn has_prefix(&self, prefix: &BlobKey) -> bool {
        self.0
            .strip_prefix(prefix.as_str())
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    }
}

fn valid_segment(segment: &str) -> bool {
    !segment.is_empty()
        && !segment.starts_with('.')
        && segment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

impl fmt::Display for BlobKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A string that is not a valid [`BlobKey`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidKey;

impl fmt::Display for InvalidKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid blob key")
    }
}

impl std::error::Error for InvalidKey {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_stay_inside_their_store() {
        for good in ["a", "a/b.c/d_e-f", "tenants/7/files/12/original"] {
            assert!(BlobKey::new(good).is_ok(), "{good}");
        }
        for bad in [
            "", "/", "/a", "a/", "a//b", "..", "../a", "a/../b", ".tmp", "a/.tmp/b", "a b", "a\\b",
            "a\0b", "é",
        ] {
            assert_eq!(BlobKey::new(bad), Err(InvalidKey), "{bad:?}");
        }
        assert!(BlobKey::new("a".repeat(MAX_KEY)).is_ok());
        assert!(BlobKey::new("a".repeat(MAX_KEY + 1)).is_err());
    }

    #[test]
    fn prefixes_match_whole_segments() {
        let key = BlobKey::new("a/b/c").unwrap();
        for (prefix, expected) in [("a", true), ("a/b", true), ("a/b/c", true), ("a/", false)] {
            let Ok(prefix) = BlobKey::new(prefix) else {
                continue;
            };
            assert_eq!(key.has_prefix(&prefix), expected);
        }
        assert!(
            !BlobKey::new("a/bc")
                .unwrap()
                .has_prefix(&BlobKey::new("a/b").unwrap())
        );
    }
}
