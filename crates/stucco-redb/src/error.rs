use crate::Revision;
use std::{error::Error, fmt};
/// Storage, serialization, or blocking-task failure, or a revision conflict.
#[derive(Debug)]
pub struct StoreError(Kind);
#[derive(Debug)]
enum Kind {
    Other(Box<dyn Error + Send + Sync>),
    Conflict(Option<Revision>),
}
impl StoreError {
    /// Wraps a diagnostic error.
    pub fn new(error: impl Error + Send + Sync + 'static) -> Self {
        Self(Kind::Other(Box::new(error)))
    }
    pub(crate) fn invalid(message: &str) -> Self {
        Self::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            message.to_owned(),
        ))
    }
    pub(crate) fn conflict(current: Option<Revision>) -> Self {
        Self(Kind::Conflict(current))
    }
    /// Whether a conditional write found a different revision.
    pub fn is_conflict(&self) -> bool {
        matches!(self.0, Kind::Conflict(_))
    }
    /// For a conflict, the record's current revision, or `Some(None)` when
    /// the record no longer exists. `None` for other errors.
    pub fn conflict_revision(&self) -> Option<Option<Revision>> {
        match self.0 {
            Kind::Conflict(current) => Some(current),
            Kind::Other(_) => None,
        }
    }
}
impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Kind::Other(error) => error.fmt(f),
            Kind::Conflict(_) => f.write_str("record changed since it was read"),
        }
    }
}
impl Error for StoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.0 {
            Kind::Other(error) => Some(error.as_ref()),
            Kind::Conflict(_) => None,
        }
    }
}
