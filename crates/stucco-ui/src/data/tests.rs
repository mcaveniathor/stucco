use super::*;
use crate::{Icon, Size, Tone};
use stucco_core::to_html;

#[test]
fn badge_renders_label_and_tone() {
    let html = to_html(&Badge::new("Paid <b>").tone(Tone::Success));
    assert!(
        html.contains(r#"class="st-badge" data-tone="success""#),
        "{html}"
    );
    assert!(html.contains("Paid &lt;b&gt;"), "{html}");
}

#[test]
fn badge_default_tone_is_default() {
    assert!(to_html(&Badge::new("Draft")).contains(r#"data-tone="default""#));
}

#[test]
fn badge_icon_is_hidden() {
    let html = to_html(&Badge::new("New").icon(Icon::custom("x", r#"<path d="M0 0"/>"#)));
    let icon = html.find(r#"aria-hidden="true""#).expect("hidden icon");
    assert!(icon < html.find("New").unwrap(), "{html}");
}

#[test]
fn initials_rules() {
    for (name, expected) in [
        ("", ""),
        ("  ", ""),
        ("Ada", "A"),
        ("Ada Lovelace", "AL"),
        ("ada  byron lovelace", "AL"),
        ("élodie Ünal", "ÉÜ"),
        ("🦀 Ferris", "🦀F"),
    ] {
        assert_eq!(avatar::initials(name), expected, "{name:?}");
    }
}

#[test]
fn avatar_labels_initials() {
    let html = to_html(&Avatar::new("Ada Lovelace"));
    assert!(html.contains(r#"role="img""#), "{html}");
    assert!(html.contains(r#"aria-label="Ada Lovelace""#), "{html}");
    assert!(
        html.contains(r#"<span class="st-avatar-initials" aria-hidden="true">AL</span>"#),
        "{html}"
    );
    assert!(html.contains(r#"data-size="md""#), "{html}");
}

#[test]
fn avatar_image_uses_alt() {
    let html = to_html(&Avatar::new("Ada Lovelace").image("/a.png"));
    assert!(
        html.contains(r#"<img src="/a.png" alt="Ada Lovelace""#),
        "{html}"
    );
    assert!(!html.contains(r#"role="img""#), "{html}");
}

#[test]
fn decorative_avatar_is_hidden() {
    let html = to_html(&Avatar::new("Ada Lovelace").decorative().image("/a.png"));
    assert!(html.starts_with(r#"<span class="st-avatar""#), "{html}");
    assert!(html.contains(r#"aria-hidden="true""#), "{html}");
    assert!(html.contains(r#"alt="""#), "{html}");
    assert!(!html.contains("aria-label"), "{html}");
}

#[test]
fn avatar_size_maps_to_nearest() {
    assert!(to_html(&Avatar::new("A").size(Size::Xs)).contains(r#"data-size="sm""#));
    assert!(to_html(&Avatar::new("A").size(Size::Xl)).contains(r#"data-size="lg""#));
    assert!(to_html(&Avatar::new("A").size(Size::Lg)).contains(r#"data-size="lg""#));
}
