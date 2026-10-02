use super::*;
use crate::{Size, Tone};
use stucco_core::to_html;

#[test]
fn heading_level_and_size_are_independent() {
    assert_eq!(
        to_html(&Heading::new(2, "Orders").size(Size::Xl)),
        r#"<h2 class="st-heading" data-size="xl">Orders</h2>"#
    );
    assert_eq!(
        to_html(&Heading::new(1, "A & B")),
        r#"<h1 class="st-heading" data-size="3xl">A &amp; B</h1>"#
    );
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "heading level")]
fn heading_levels_outside_one_to_six_panic_in_debug() {
    let _ = Heading::new(7, "x");
}

#[test]
fn text_renders_tone_weight_and_inline() {
    assert_eq!(
        to_html(&Text::new("x").tone(Tone::Muted).inline()),
        r#"<span class="st-text" data-size="md" data-tone="muted">x</span>"#
    );
    assert_eq!(
        to_html(&Text::new("<b>").weight(Weight::Bold)),
        r#"<p class="st-text" data-size="md" data-weight="bold">&lt;b&gt;</p>"#
    );
}

#[test]
fn external_links_are_safe_and_announced() {
    let html = to_html(&Link::new("Docs", "https://e.com").external());
    assert!(
        html.contains(r#"target="_blank" rel="noopener noreferrer""#),
        "{html}"
    );
    assert!(html.contains(r#"<span class="st-sr-only"> (opens in a new tab)</span>"#));
    assert!(to_html(&Link::new("x", "javascript:alert(1)")).contains(r##"href="#""##));
}

#[test]
fn code_and_kbd() {
    assert_eq!(
        to_html(&Code::new("a < b")),
        r#"<code class="st-code">a &lt; b</code>"#
    );
    assert_eq!(
        to_html(&Kbd::new(&["Ctrl", "K"])),
        r#"<kbd class="st-kbd"><kbd>Ctrl</kbd>+<kbd>K</kbd></kbd>"#
    );
}
