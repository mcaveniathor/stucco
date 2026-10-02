//! Fragments: partial renders that keep their asset requirements (spec §4.2).

use crate::escape::escape_attr;
use crate::{AssetRequirements, Bundle, Cx, Render, behavior, is_registered};

/// A partial render: HTML plus the assets it needs.
#[derive(Debug, Clone)]
pub struct RenderedFragment {
    /// The rendered markup.
    pub html: String,
    /// Resolved, dependency-ordered requirements.
    pub assets: AssetRequirements,
}

/// Renders `r` as a fragment whose generated ids are prefixed with
/// `namespace` (which must match `[a-z0-9-]+` and be unique within the
/// document the fragment is inserted into).
///
/// # Panics
///
/// On an invalid namespace, in all builds: a bad namespace risks id
/// collisions, so it is treated as a programming error.
pub fn render_fragment(namespace: &str, r: &(impl Render + ?Sized)) -> RenderedFragment {
    let valid = !namespace.is_empty()
        && namespace
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    assert!(valid, "invalid fragment namespace: {namespace:?}");
    let mut cx = Cx::with_namespace(namespace);
    r.render(&mut cx);
    let (html, mut assets) = cx.finish();
    assets.retain(|a| {
        let ok = a.behavior.is_none() || is_registered(a);
        debug_assert!(ok, "unregistered asset in fragment: {}", a.name);
        ok
    });
    RenderedFragment { html, assets }
}

impl RenderedFragment {
    /// The HTML to send in a fragment response: an `<st-require>` element
    /// listing the behaviour modules to load (if any), then the markup.
    pub fn to_response_html(&self, bundle: &Bundle) -> String {
        let urls: Vec<&str> = self
            .assets
            .behaviors()
            .filter_map(|a| bundle.script_url(a))
            .collect();
        if urls.is_empty() {
            return self.html.clone();
        }
        let tag = behavior::REQUIRE_TAG;
        let mut out = format!("<{tag} modules=\"");
        escape_attr(&urls.join(" "), &mut out);
        out.push_str(&format!("\"></{tag}>"));
        out.push_str(&self.html);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Asset, Behavior, Cx, el};
    use stucco_theme::Preset;

    static PROBE: Asset = Asset {
        name: "probe",
        css: None,
        behavior: Some(Behavior::Js("/*probe*/")),
        deps: &[],
    };
    crate::register_asset!(PROBE);

    struct Probe;
    impl Render for Probe {
        fn render(&self, cx: &mut Cx) {
            cx.require(&PROBE);
            let id = cx.id("probe");
            el::custom("st-probe").id(id).render(cx);
        }
    }

    #[test]
    fn fragments_namespace_ids_and_keep_requirements() {
        let a = render_fragment("a", &Probe);
        let b = render_fragment("b", &Probe);
        assert_eq!(a.html, r#"<st-probe id="a-probe-1"></st-probe>"#);
        assert_eq!(b.html, r#"<st-probe id="b-probe-1"></st-probe>"#);
        assert_eq!(
            a.assets.iter().map(|x| x.name).collect::<Vec<_>>(),
            ["probe"]
        );
    }

    #[test]
    fn response_html_announces_modules() {
        let bundle = Bundle::new(Preset::Slate);
        let html = render_fragment("a", &Probe).to_response_html(&bundle);
        let url = bundle.script_url(&PROBE).unwrap();
        assert!(html.starts_with(&format!(r#"<st-require modules="{url}"></st-require>"#)));
        assert_eq!(
            render_fragment("p", &"plain").to_response_html(&bundle),
            "plain"
        );
    }

    #[test]
    #[should_panic(expected = "invalid fragment namespace")]
    fn empty_namespace_panics() {
        render_fragment("", &"x");
    }

    #[test]
    #[should_panic(expected = "invalid fragment namespace")]
    fn spaced_namespace_panics() {
        render_fragment("Row 7", &"x");
    }

    static LOOSE: Asset = Asset {
        name: "loose",
        css: None,
        behavior: Some(Behavior::Js("/*loose*/")),
        deps: &[],
    };

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "unregistered asset in fragment: loose")]
    fn unregistered_behaviours_panic_in_debug() {
        render_fragment("a", &crate::render_fn(|cx: &mut Cx| cx.require(&LOOSE)));
    }
}
