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
        ("ß", "S"),
        ("ß Smith", "SS"),
        ("ﬃ", "F"),
        ("ﬃ Foo", "FF"),
        ("Ada ß", "AS"),
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

#[test]
fn description_list_pairs_terms_and_details() {
    let html = to_html(
        &DescriptionList::new()
            .item("Customer", "Ada <x>")
            .item("Status", Badge::new("Paid")),
    );
    assert!(html.starts_with(r#"<dl class="st-dl">"#), "{html}");
    assert_eq!(
        html.matches(r#"<div class="st-dl-item"><dt>"#).count(),
        2,
        "{html}"
    );
    assert!(
        html.contains("<dt>Customer</dt><dd>Ada &lt;x&gt;</dd>"),
        "{html}"
    );
    assert!(html.contains(r#"<dd><span class="st-badge""#), "{html}");
}

#[test]
fn empty_description_list_is_an_empty_dl() {
    assert_eq!(
        to_html(&DescriptionList::new()),
        r#"<dl class="st-dl"></dl>"#
    );
}

#[test]
fn stat_without_change_omits_the_change_line() {
    let html = to_html(&Stat::new("Revenue", "$12,400"));
    assert!(
        html.contains(r#"<p class="st-stat-label">Revenue</p>"#),
        "{html}"
    );
    assert!(
        html.contains(r#"<p class="st-stat-value">$12,400</p>"#),
        "{html}"
    );
    assert!(!html.contains("st-stat-change"), "{html}");
}

#[test]
fn stat_change_has_hidden_direction() {
    let html = to_html(
        &Stat::new("Orders", "128")
            .change("12%", Trend::Up)
            .context("vs last week"),
    );
    assert!(
        html.contains(r#"<span class="st-sr-only">Up </span>12%"#),
        "{html}"
    );
    assert!(html.contains(r#"data-trend="up""#), "{html}");
    assert!(html.contains(r#"data-tone="default""#), "{html}");
    assert!(html.contains("vs last week"), "{html}");
    let down = to_html(
        &Stat::new("x", "1")
            .change("3", Trend::Down)
            .change_tone(Tone::Success),
    );
    assert!(
        down.contains(r#"<span class="st-sr-only">Down </span>3"#),
        "{down}"
    );
    assert!(down.contains(r#"data-tone="success""#), "{down}");
    let flat = to_html(&Stat::new("x", "1").change("0%", Trend::Flat));
    assert!(
        flat.contains(r#"<span class="st-sr-only">No change </span>0%"#),
        "{flat}"
    );
}

#[test]
fn stat_escapes_text() {
    let html = to_html(
        &Stat::new("<i>label", "<i>value")
            .change("<i>", Trend::Up)
            .context("<i>ctx"),
    );
    assert!(!html.contains("<i>"), "{html}");
    assert_eq!(html.matches("&lt;i&gt;").count(), 4, "{html}");
}

#[test]
fn sparkline_handles_degenerate_and_extreme_samples_without_invalid_svg() {
    let empty = to_html(&Sparkline::new("Empty", [f64::NAN, f64::INFINITY]));
    assert!(empty.contains("No data"));
    assert!(!empty.contains("<svg"));
    let single = to_html(&Sparkline::new("One", [3.]));
    assert!(single.contains("<circle"));
    assert!(single.contains("Values in order: 3"));
    let constant = to_html(&Sparkline::new("Constant", [2., 2., 2.]));
    assert!(constant.contains("4.00,30.00 100.00,30.00 196.00,30.00"));
    let extreme = to_html(&Sparkline::new("<script>trend", [-f64::MAX, 0., f64::MAX]));
    assert!(extreme.contains("4.00,56.00 100.00,30.00 196.00,4.00"));
    assert!(extreme.contains("&lt;script&gt;trend"));
    assert!(!extreme.contains("NaN"));
    assert!(!extreme.contains("Infinity"));
}

#[test]
fn timeline_preserves_supplied_order_time_and_text() {
    let html = to_html(
        &Timeline::new("Activity")
            .entry(TimelineEntry::new(
                "2026-10-21T14:30:00Z",
                "Today",
                "First <event>",
            ))
            .entry(
                TimelineEntry::new("2026-10-20", "Yesterday", "Second event")
                    .icon(Icon::custom("check", "<path d=\"M1 1h1\"/>")),
            ),
    );
    assert!(html.contains("<ol"));
    assert!(html.contains("datetime=\"2026-10-21T14:30:00Z\""));
    assert!(html.contains("First &lt;event&gt;"));
    assert!(html.find("First").unwrap() < html.find("Second").unwrap());
    assert!(html.contains("aria-hidden=\"true\""));
    assert!(to_html(&Timeline::new("Empty")).contains("No activity yet"));
}
