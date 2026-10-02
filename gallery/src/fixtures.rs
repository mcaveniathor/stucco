//! Browser-test fixtures: an enhanced page and fragments that require a
//! behaviour the page does not load.

use stucco::{
    Asset, Behavior, Bundle, Cx, Page, Render, behavior, el, register_asset, render_fragment,
};

/// A probe behaviour: `<st-probe>` sets `data-ready` once its module runs.
pub static PROBE: Asset = Asset {
    name: "probe",
    css: None,
    behavior: Some(Behavior::Js(include_str!("../js/probe.js"))),
    deps: &[],
};
register_asset!(PROBE);

/// Renders an `<st-probe>` and requires its module.
pub struct Probe;

impl Render for Probe {
    fn render(&self, cx: &mut Cx) {
        cx.require(&PROBE);
        let id = cx.id("probe");
        el::custom("st-probe").id(id).text("probe").render(cx);
    }
}

/// `(path, html)` for every fixture file.
pub fn fixtures(bundle: &Bundle) -> Vec<(String, String)> {
    let page = Page::new(bundle, "Enhanced fixture — stucco gallery")
        .enhanced()
        .body(
            el::main()
                .class("g-page")
                .child(el::h1().text("Enhanced fixture"))
                .child(
                    el::div()
                        .id("target")
                        .bool_attr(behavior::REGION_ATTR, true),
                ),
        )
        .render();
    vec![
        ("fixtures/enhanced.html".to_owned(), page),
        (
            "fixtures/probe-a.html".to_owned(),
            render_fragment("probe-a", &Probe).to_response_html(bundle),
        ),
        (
            "fixtures/probe-b.html".to_owned(),
            render_fragment("probe-b", &Probe).to_response_html(bundle),
        ),
    ]
}
