//! Small closed sets of presentation values shared by data components.
//!
//! These mirror the `Tone` and `Size` values of `stucco-ui`, which is
//! not part of the WASM build, and render the same `data-*` attribute values.

/// Text and status colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Tone {
    /// Body text colour.
    #[default]
    Default,
    /// Secondary text.
    Muted,
    /// Accent colour.
    Accent,
    /// Success.
    Success,
    /// Warning.
    Warning,
    /// Danger.
    Danger,
    /// Information.
    Info,
}

impl Tone {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Muted => "muted",
            Self::Accent => "accent",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Danger => "danger",
            Self::Info => "info",
        }
    }
}

/// A visual size step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Size {
    /// Extra small.
    Xs,
    /// Small.
    Sm,
    /// Medium (body).
    #[default]
    Md,
    /// Large.
    Lg,
    /// Extra large.
    Xl,
}
