//! Full HTML documents with collected assets (spec §4.7).

use crate::bundle::substitute_placeholders;
use crate::escape::{escape_attr, escape_text};
use crate::{Attrs, Behavior, Bundle, Cx, Href, Render, Slot, is_registered};
use stucco_theme::Scheme;

/// Content for an inline `<script>` or `<style>`: `</` becomes `<\/` so the
/// element cannot be closed early (valid in JS strings and CSS alike).
fn raw_text(s: &str) -> String {
    crate::el::neutralise_raw_text(s)
}

const THEME_INIT_JS: &str = include_str!("../js/theme_init.js");

/// How a page delivers its CSS and scripts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Delivery {
    /// Link the site-wide stylesheet and hashed modules (cacheable).
    #[default]
    Linked,
    /// Inline only the CSS and scripts this page uses (single file; no
    /// fragment support).
    Inline,
}

/// Document metadata.
#[derive(Clone, Debug, Default)]
pub struct Meta {
    description: Option<String>,
    og_title: Option<String>,
    og_image: Option<Href>,
    canonical: Option<Href>,
}

impl Meta {
    /// `<meta name="description">`.
    pub fn description(text: &str) -> Meta {
        Meta {
            description: Some(text.to_owned()),
            ..Meta::default()
        }
    }

    /// `<meta property="og:title">`.
    pub fn og_title(mut self, title: &str) -> Meta {
        self.og_title = Some(title.to_owned());
        self
    }

    /// `<meta property="og:image">`.
    pub fn og_image(mut self, url: impl Into<Href>) -> Meta {
        self.og_image = Some(url.into());
        self
    }

    /// `<link rel="canonical">`.
    pub fn canonical(mut self, url: impl Into<Href>) -> Meta {
        self.canonical = Some(url.into());
        self
    }
}

/// A complete HTML document.
#[derive(Debug)]
pub struct Page<'b, 'a> {
    bundle: &'b Bundle,
    title: String,
    lang: String,
    meta: Meta,
    head: Option<Slot<'a>>,
    body: Option<Slot<'a>>,
    body_attrs: Attrs,
    nonce: Option<String>,
    enhanced: bool,
    delivery: Delivery,
}

impl<'b, 'a> Page<'b, 'a> {
    /// A page titled `title`, using `bundle`'s assets.
    pub fn new(bundle: &'b Bundle, title: impl Into<String>) -> Page<'b, 'a> {
        Page {
            bundle,
            title: title.into(),
            lang: "en".to_owned(),
            meta: Meta::default(),
            head: None,
            body: None,
            body_attrs: Attrs::default(),
            nonce: None,
            enhanced: false,
            delivery: Delivery::Linked,
        }
    }

    /// The document language (default `"en"`).
    pub fn lang(mut self, lang: &str) -> Self {
        self.lang = lang.to_owned();
        self
    }

    /// Document metadata.
    pub fn meta(mut self, meta: Meta) -> Self {
        self.meta = meta;
        self
    }

    /// Extra `<head>` content.
    pub fn head(mut self, head: impl Render + 'a) -> Self {
        self.head = Some(Slot::new(head));
        self
    }

    /// The `<body>` content.
    pub fn body(mut self, body: impl Render + 'a) -> Self {
        self.body = Some(Slot::new(body));
        self
    }

    /// Attributes on `<body>`.
    pub fn body_attrs(mut self, attrs: Attrs) -> Self {
        self.body_attrs = attrs;
        self
    }

    /// CSP nonce added to every inline `<script>` and `<style>`.
    pub fn csp_nonce(mut self, nonce: impl Into<String>) -> Self {
        self.nonce = Some(nonce.into());
        self
    }

    /// Includes the client runtime even if no behaviour is used yet, so
    /// fragments inserted later can load modules.
    pub fn enhanced(mut self) -> Self {
        self.enhanced = true;
        self
    }

    /// CSS and script delivery mode.
    pub fn delivery(mut self, delivery: Delivery) -> Self {
        self.delivery = delivery;
        self
    }

    /// Renders the document.
    pub fn render(self) -> String {
        debug_assert!(
            !(self.enhanced && self.delivery == Delivery::Inline),
            "Inline pages do not support fragments"
        );
        let mut cx = Cx::new();
        if let Some(body) = &self.body {
            body.render(&mut cx);
        }
        let body_html = cx.take_output();
        if let Some(head) = &self.head {
            head.render(&mut cx);
        }
        let head_html = cx.take_output();
        self.body_attrs.render(&mut cx);
        let body_attrs = cx.take_output();
        let (_, required) = cx.finish();

        let nonce = self.nonce.as_deref().map_or(String::new(), |n| {
            let mut s = String::from(" nonce=\"");
            escape_attr(n, &mut s);
            s.push('"');
            s
        });
        let mut out = String::from("<!doctype html><html lang=\"");
        escape_attr(&self.lang, &mut out);
        out.push_str("\"><head><meta charset=\"utf-8\">");
        out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");
        for (scheme, media) in [(Scheme::Light, "light"), (Scheme::Dark, "dark")] {
            if let Some(bg) = self.bundle.root.role(scheme, "bg") {
                out.push_str(&format!(
                    "<meta name=\"theme-color\" media=\"(prefers-color-scheme: {media})\" content=\"{}\">",
                    bg.css()
                ));
            }
        }
        out.push_str("<title>");
        escape_text(&self.title, &mut out);
        out.push_str("</title>");
        self.write_meta(&mut out);
        out.push_str(&format!(
            "<script{nonce}>{}</script>",
            raw_text(&substitute_placeholders(THEME_INIT_JS))
        ));
        let inline_script = |out: &mut String, js: &str| {
            out.push_str(&format!(
                "<script type=\"module\"{nonce}>{}</script>",
                raw_text(&substitute_placeholders(js))
            ));
        };
        let module_src = |out: &mut String, url: &str| {
            out.push_str("<script type=\"module\" src=\"");
            escape_attr(url, out);
            out.push_str(&format!("\"{nonce}></script>"));
        };
        match self.delivery {
            Delivery::Linked => {
                out.push_str("<link rel=\"stylesheet\" href=\"");
                escape_attr(self.bundle.stylesheet_url(), &mut out);
                out.push_str(&format!("\"{nonce}>"));
                let unregistered_css: String = required
                    .iter()
                    .filter(|a| !is_registered(a))
                    .filter_map(|a| a.css)
                    .collect();
                if !unregistered_css.is_empty() {
                    out.push_str(&format!(
                        "<style{nonce}>{}</style>",
                        raw_text(&unregistered_css)
                    ));
                }
                if self.enhanced || required.behaviors().next().is_some() {
                    module_src(&mut out, self.bundle.runtime_url());
                }
                for asset in required.behaviors() {
                    match (self.bundle.script_url(asset), asset.behavior) {
                        (Some(url), _) => module_src(&mut out, url),
                        (None, Some(Behavior::Js(js))) => inline_script(&mut out, js),
                        (None, None) => {}
                    }
                }
            }
            Delivery::Inline => {
                out.push_str(&format!(
                    "<style{nonce}>{}</style>",
                    raw_text(&self.bundle.css_for(&required))
                ));
                for asset in required.behaviors() {
                    if let Some(Behavior::Js(js)) = asset.behavior {
                        inline_script(&mut out, js);
                    }
                }
            }
        }
        out.push_str(&head_html);
        out.push_str("</head><body");
        out.push_str(&body_attrs);
        out.push('>');
        out.push_str(&body_html);
        out.push_str("</body></html>");
        out
    }

    fn write_meta(&self, out: &mut String) {
        let mut tag = |open: &str, value: &str| {
            out.push_str(open);
            escape_attr(value, out);
            out.push_str("\">");
        };
        if let Some(d) = &self.meta.description {
            tag("<meta name=\"description\" content=\"", d);
        }
        if let Some(t) = &self.meta.og_title {
            tag("<meta property=\"og:title\" content=\"", t);
        }
        if let Some(i) = &self.meta.og_image {
            tag("<meta property=\"og:image\" content=\"", i.as_str());
        }
        if let Some(c) = &self.meta.canonical {
            tag("<link rel=\"canonical\" href=\"", c.as_str());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Asset, Behavior, Cx, el};
    use stucco_theme::Preset;

    static WIDGET: Asset = Asset {
        name: "widget",
        css: Some("@layer stucco.components { .st-widget { color: var(--st-text); } }"),
        behavior: Some(Behavior::Js("/*widget*/")),
        deps: &[],
    };
    crate::register_asset!(WIDGET);
    static LOOSE: Asset = Asset {
        name: "loose",
        css: None,
        behavior: Some(Behavior::Js("/*loose*/")),
        deps: &[],
    };

    struct Widget;
    impl Render for Widget {
        fn render(&self, cx: &mut Cx) {
            cx.require(&WIDGET);
            el::div().class("st-widget").render(cx);
        }
    }
    struct Loose;
    impl Render for Loose {
        fn render(&self, cx: &mut Cx) {
            cx.require(&LOOSE);
        }
    }

    #[test]
    fn linked_page_links_css_runtime_and_used_modules() {
        let b = Bundle::new(Preset::Slate);
        let html = Page::new(&b, "A <b> title")
            .meta(Meta::description("d"))
            .body(Widget)
            .render();
        assert!(
            html.starts_with("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">")
        );
        assert!(html.contains("<title>A &lt;b&gt; title</title>"));
        assert!(html.contains("<meta name=\"description\" content=\"d\">"));
        assert!(html.contains(&format!(
            "<link rel=\"stylesheet\" href=\"{}\">",
            b.stylesheet_url()
        )));
        assert!(html.contains(&format!(
            "<script type=\"module\" src=\"{}\"></script>",
            b.runtime_url()
        )));
        assert!(html.contains(&format!(
            "<script type=\"module\" src=\"{}\"></script>",
            b.script_url(&WIDGET).unwrap()
        )));
        assert!(html.ends_with("<body><div class=\"st-widget\"></div></body></html>"));
    }

    #[test]
    fn plain_pages_have_no_runtime_unless_enhanced() {
        let b = Bundle::new(Preset::Slate);
        let plain = Page::new(&b, "t").body("plain").render();
        assert!(!plain.contains("type=\"module\""));
        let enhanced = Page::new(&b, "t").enhanced().body("plain").render();
        assert!(enhanced.contains(b.runtime_url()));
    }

    #[test]
    fn unregistered_behaviours_are_inlined() {
        let b = Bundle::new(Preset::Slate);
        let html = Page::new(&b, "t").body(Loose).render();
        assert!(html.contains("<script type=\"module\">/*loose*/</script>"));
    }

    #[test]
    fn inline_page_ships_only_used_css_and_nonces_everything() {
        let b = Bundle::new(Preset::Slate);
        let html = Page::new(&b, "t")
            .delivery(Delivery::Inline)
            .csp_nonce("n0nce")
            .body(Widget)
            .render();
        assert!(html.contains(".st-widget"));
        assert!(!html.contains("<link rel=\"stylesheet\"") && !html.contains(b.runtime_url()));
        let inline_tags = html.matches("<script").count() + html.matches("<style").count();
        assert_eq!(html.matches("nonce=\"n0nce\"").count(), inline_tags);
        assert!(html.contains("localStorage") && html.contains("stucco-theme"));
    }

    static UNSTYLED: Asset = Asset {
        name: "unstyled",
        css: Some("@layer stucco.components { .st-unstyled { color: var(--st-text); } }"),
        behavior: None,
        deps: &[],
    };

    #[test]
    fn unregistered_css_is_inlined_on_linked_pages() {
        let b = Bundle::new(Preset::Slate);
        let html = Page::new(&b, "t")
            .csp_nonce("n0nce")
            .body(crate::render_fn(|cx: &mut Cx| cx.require(&UNSTYLED)))
            .render();
        assert!(html.contains("<style nonce=\"n0nce\">@layer stucco.components { .st-unstyled"));
    }

    static HOSTILE: Asset = Asset {
        name: "hostile",
        css: Some("/* </style><script>alert(1)</script> */"),
        behavior: Some(Behavior::Js("// </script><script>alert(2)</script>")),
        deps: &[],
    };

    #[test]
    fn inline_style_and_script_cannot_be_closed_early() {
        let b = Bundle::new(Preset::Slate);
        let html = Page::new(&b, "t")
            .delivery(Delivery::Inline)
            .body(crate::render_fn(|cx: &mut Cx| cx.require(&HOSTILE)))
            .render();
        assert!(!html.contains("</style><script>alert(1)"), "{html}");
        assert!(!html.contains("</script><script>alert(2)"), "{html}");
    }

    #[test]
    fn linked_page_nonces_every_script_and_stylesheet() {
        let b = Bundle::new(Preset::Slate);
        let html = Page::new(&b, "t").csp_nonce("n0nce").body(Widget).render();
        let tags = html.matches("<script").count()
            + html.matches("<style").count()
            + html.matches("<link rel=\"stylesheet\"").count();
        assert_eq!(html.matches("nonce=\"n0nce\"").count(), tags);
    }

    #[test]
    fn body_attrs_and_lang_are_applied() {
        let b = Bundle::new(Preset::Slate);
        let html = Page::new(&b, "t")
            .lang("de")
            .body_attrs(Attrs::default().class("app"))
            .render();
        assert!(html.contains("<html lang=\"de\">") && html.contains("<body class=\"app\">"));
    }
}
