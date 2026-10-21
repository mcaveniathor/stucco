/// The severity of a one-shot application message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlashLevel {
    /// Informational feedback.
    Info,
    /// A completed action.
    Success,
    /// A condition needing attention.
    Warning,
    /// An action failed.
    Error,
}

/// Plain-text feedback carried across a redirect.
///
/// This value is independent of HTTP. `stucco-tower` transports it in a
/// signed cookie and `stucco-ui` renders it with `FlashMessages`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Flash {
    /// Message severity.
    pub level: FlashLevel,
    /// Plain text, escaped when rendered. Do not put secrets in cookie-backed messages.
    pub message: String,
}

impl Flash {
    /// Creates a message at the given level.
    pub fn new(level: FlashLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
        }
    }
}
