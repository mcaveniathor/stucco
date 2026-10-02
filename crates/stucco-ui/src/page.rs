//! Shortcuts for filling a page's body with the landmarks every page needs.

use stucco_core::WithBody;
#[cfg(feature = "layout")]
use stucco_core::{Render, el};

#[cfg(feature = "app")]
use crate::app::AppShell;
#[cfg(feature = "layout")]
use crate::layout::SkipLink;

/// Body shortcuts for [`Page`](stucco_core::Page) and other documents.
///
/// [`main`](PageExt::main) wraps content in the page's main landmark,
/// `<main id="main">`, after a skip link to it, so a simple page is
/// accessible without knowing the convention. `app` fills the body with an
/// `AppShell`, which brings its own skip link and landmark.
pub trait PageExt<'a>: WithBody<'a> {
    /// Fills the body with a skip link and `<main id="main">` holding
    /// `content`.
    ///
    /// ```
    /// use stucco_core::{Bundle, Page, el};
    /// use stucco_ui::PageExt;
    ///
    /// let bundle = Bundle::new(stucco_theme::Preset::Slate);
    /// let html = Page::new(&bundle, "Hello")
    ///     .main(el::h1().text("Hello"))
    ///     .render();
    /// assert!(html.contains(r##"href="#main""##));
    /// assert!(html.contains(r#"<main id="main"><h1>Hello</h1></main>"#));
    /// ```
    #[cfg(feature = "layout")]
    fn main(self, content: impl Render + 'a) -> Self {
        self.with_body((SkipLink::new(), el::main().id("main").child(content)))
    }

    /// Fills the body with an application shell, which brings its own skip
    /// link and main landmark.
    #[cfg(feature = "app")]
    fn app(self, shell: AppShell<'a>) -> Self {
        self.with_body(shell)
    }
}

impl<'a, T: WithBody<'a>> PageExt<'a> for T {}
