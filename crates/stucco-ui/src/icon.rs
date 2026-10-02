//! Icons: the [`Icon`] type (always available) and, with the `icons` feature,
//! the vendored Lucide set as constants (`icon::ARROW_RIGHT`).

use stucco_core::escape::escape_attr;
use stucco_core::{Asset, Cx, Raw, Render, register_asset};

#[cfg(feature = "icons")]
#[rustfmt::skip]
mod lucide;
#[cfg(feature = "icons")]
pub use lucide::*;

/// Icon sizing and alignment.
pub static ICON: Asset = Asset {
    name: "st-icon",
    css: Some(include_str!("../css/icon.css")),
    behavior: None,
    deps: &[],
};
register_asset!(ICON);

const SVG_OPEN: &str = r#"<svg class="st-icon" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round""#;

/// An inline SVG icon drawn on a 24×24 grid with Lucide's stroke style.
/// Decorative by default (hidden from assistive technology); give it a label
/// with [`Icon::label`] when it carries meaning on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Icon {
    name: &'static str,
    body: &'static str,
}

impl Icon {
    /// An icon from trusted inner SVG markup for a `0 0 24 24` viewBox
    /// (paths, circles…). The body is written verbatim.
    pub const fn custom(name: &'static str, body: &'static str) -> Icon {
        Icon { name, body }
    }

    /// The icon's name.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The inner SVG markup.
    pub fn body(&self) -> &'static str {
        self.body
    }

    /// This icon as a meaningful image with an accessible name.
    pub fn label(self, label: impl Into<String>) -> LabelledIcon {
        LabelledIcon {
            icon: self,
            label: label.into(),
        }
    }
}

/// An icon with an accessible name (`role="img"`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabelledIcon {
    icon: Icon,
    label: String,
}

impl Render for Icon {
    fn render(&self, cx: &mut Cx) {
        cx.require(&ICON);
        Raw::trusted(format!(
            r#"{SVG_OPEN} aria-hidden="true" focusable="false">{}</svg>"#,
            self.body
        ))
        .render(cx);
    }
}

impl Render for LabelledIcon {
    fn render(&self, cx: &mut Cx) {
        cx.require(&ICON);
        let mut label = String::new();
        escape_attr(&self.label, &mut label);
        Raw::trusted(format!(
            r#"{SVG_OPEN} role="img" aria-label="{label}" focusable="false">{}</svg>"#,
            self.icon.body
        ))
        .render(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stucco_core::to_html;

    #[test]
    fn decorative_icons_are_hidden_from_assistive_technology() {
        let html = to_html(&Icon::custom(
            "dot",
            "<circle cx=\"12\" cy=\"12\" r=\"2\"/>",
        ));
        assert!(
            html.starts_with("<svg class=\"st-icon\" viewBox=\"0 0 24 24\""),
            "{html}"
        );
        assert!(html.contains("aria-hidden=\"true\" focusable=\"false\""));
        assert!(html.ends_with("<circle cx=\"12\" cy=\"12\" r=\"2\"/></svg>"));
    }

    #[test]
    fn labelled_icons_are_images_with_escaped_labels() {
        let html = to_html(&Icon::custom("dot", "").label("Done & <ok>"));
        assert!(
            html.contains("role=\"img\" aria-label=\"Done &amp; &lt;ok&gt;\""),
            "{html}"
        );
        assert!(!html.contains("aria-hidden"));
    }

    #[test]
    fn the_allowlist_rejects_anything_but_plain_shapes() {
        assert!(is_plain_shape_markup(
            r#"<path d="M5 12h14" /><circle cx="1" cy="2" r="3" />"#
        ));
        for bad in [
            "<animate/onbegin=alert(1) attributeName=x dur=1s>",
            r#"<path d="x" onclick="y" />"#,
            r#"<a href="javascript:x"><path d="x" /></a>"#,
            "<style>*{}</style>",
            "<foreignObject></foreignObject>",
            r#"<path d="x" style="fill:red" />"#,
            "<path d=x>",
        ] {
            assert!(!is_plain_shape_markup(bad), "{bad}");
        }
    }

    #[cfg(feature = "icons")]
    #[test]
    fn the_lucide_set_is_complete_and_safe() {
        assert!(ALL.len() > 1500, "{}", ALL.len());
        assert_eq!(ARROW_RIGHT.name(), "arrow-right");
        for icon in ALL {
            assert!(
                is_plain_shape_markup(icon.body()),
                "{}: {}",
                icon.name(),
                icon.body()
            );
        }
    }

    /// Whether `body` is only self-closing plain shape elements with
    /// geometry attributes and double-quoted values — the allowlist the Lucide
    /// generator enforces (tools/gen-icons.mjs).
    fn is_plain_shape_markup(body: &str) -> bool {
        const ELEMENTS: &[&str] = &[
            "path", "circle", "rect", "line", "polyline", "polygon", "ellipse",
        ];
        const ATTRIBUTES: &[&str] = &[
            "d", "cx", "cy", "r", "rx", "ry", "x", "y", "x1", "x2", "y1", "y2", "width", "height",
            "points", "fill",
        ];
        let mut rest = body.trim();
        while !rest.is_empty() {
            let Some(tag) = rest.strip_prefix('<') else {
                return false;
            };
            let Some(end) = tag.find("/>") else {
                return false;
            };
            let inner = &tag[..end];
            rest = tag[end + 2..].trim_start();
            let mut parts = inner.splitn(2, ' ');
            let name = parts.next().unwrap_or("");
            if !ELEMENTS.contains(&name) {
                return false;
            }
            let mut attrs = parts.next().unwrap_or("").trim();
            while !attrs.is_empty() {
                let Some((attr, after)) = attrs.split_once("=\"") else {
                    return false;
                };
                let Some((value, next)) = after.split_once('"') else {
                    return false;
                };
                if !ATTRIBUTES.contains(&attr) || value.contains('<') || value.contains('>') {
                    return false;
                }
                attrs = next.trim_start();
            }
        }
        true
    }
}
