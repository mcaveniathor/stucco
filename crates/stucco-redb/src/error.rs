use std::{error::Error, fmt};
/// Storage, serialization, or blocking-task failure.
#[derive(Debug)]
pub struct StoreError(Box<dyn Error + Send + Sync>);
impl StoreError {
    /// Wraps a diagnostic error.
    pub fn new(error: impl Error + Send + Sync + 'static) -> Self {
        Self(Box::new(error))
    }
    pub(crate) fn invalid(message: &str) -> Self {
        Self::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            message.to_owned(),
        ))
    }
}
impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl Error for StoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.0.as_ref())
    }
}
