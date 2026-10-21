use super::*;
use stucco_core::to_html;

/// The value of the first `name="…"` attribute after `after`.
fn attr<'h>(html: &'h str, after: &str, name: &str) -> &'h str {
    let start = html
        .find(after)
        .unwrap_or_else(|| panic!("no {after} in {html}"));
    let rest = &html[start..];
    let at = rest
        .find(&format!("{name}=\""))
        .unwrap_or_else(|| panic!("no {name} in {rest}"));
    let value = &rest[at + name.len() + 2..];
    &value[..value.find('"').unwrap()]
}

#[test]
fn progress_labels_its_element() {
    let html = to_html(&Progress::new("Import", 42.0, 100.0).show_value());
    let id = attr(&html, "<label", "for");
    assert_eq!(attr(&html, "<progress", "id"), id, "{html}");
    assert!(html.contains(">Import</label>"), "{html}");
    assert!(
        html.contains(r#"value="42" max="100">42%</progress>"#),
        "{html}"
    );
    assert!(
        html.contains(r#"<span class="st-progress-value">42%</span>"#),
        "{html}"
    );
}

#[test]
fn indeterminate_progress_has_no_value() {
    let html = to_html(&Progress::indeterminate("Uploading").show_value());
    assert!(!html.contains("value="), "{html}");
    assert!(!html.contains('%'), "{html}");
}

#[test]
fn progress_sanitises_inputs() {
    for (value, max, expect) in [
        (5.0, 0.0, r#"value="1" max="1""#),
        (f64::NAN, 10.0, r#"value="0" max="10""#),
        (-3.0, 10.0, r#"value="0" max="10""#),
        (20.0, 10.0, r#"value="10" max="10""#),
        (1.0, f64::INFINITY, r#"value="1" max="1""#),
    ] {
        let html = to_html(&Progress::new("x", value, max));
        assert!(html.contains(expect), "{value} {max}: {html}");
    }
}

#[test]
fn meter_sets_bands_and_text() {
    let html = to_html(
        &Meter::new("Storage", 7.2, 0.0, 10.0)
            .low(3.0)
            .high(8.0)
            .optimum(1.0)
            .value_text("7.2 of 10 GB"),
    );
    assert!(
        html.contains(r#"value="7.2" min="0" max="10" low="3" high="8" optimum="1""#),
        "{html}"
    );
    let described = attr(&html, "<meter", "aria-describedby");
    assert!(
        html.contains(&format!(
            r#"<span id="{described}" class="st-meter-text">7.2 of 10 GB</span>"#
        )),
        "{html}"
    );
    assert_eq!(
        attr(&html, "<label", "for"),
        attr(&html, "<meter", "id"),
        "{html}"
    );
}

#[test]
fn meter_fixes_an_empty_range() {
    let html = to_html(&Meter::new("x", 1.0, 3.0, 3.0).low(99.0));
    assert!(
        html.contains(r#"value="3" min="3" max="4" low="4""#),
        "{html}"
    );
    let html = to_html(&Meter::new("x", f64::NAN, f64::NAN, f64::NAN));
    assert!(html.contains(r#"value="0" min="0" max="1""#), "{html}");
}

#[test]
fn meter_without_text_has_no_description() {
    assert!(!to_html(&Meter::new("x", 1.0, 0.0, 2.0)).contains("aria-describedby"));
}

#[test]
fn meter_repairs_extreme_empty_ranges() {
    for min in [9_007_199_254_740_992.0, 1e20, -1e20, f64::MAX, -f64::MAX] {
        for max in [min, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let html = to_html(&Meter::new("x", f64::NAN, min, max));
            let repaired_min: f64 = attr(&html, "<meter", "min").parse().unwrap();
            let repaired_max: f64 = attr(&html, "<meter", "max").parse().unwrap();
            let value: f64 = attr(&html, "<meter", "value").parse().unwrap();
            assert!(
                repaired_min.is_finite() && repaired_max.is_finite(),
                "{html}"
            );
            assert!(repaired_max > repaired_min, "{html}");
            assert_eq!(value, repaired_min, "{html}");
            if min == f64::MAX {
                assert_eq!(repaired_min, f64::from_bits(f64::MAX.to_bits() - 1));
                assert_eq!(repaired_max, f64::MAX);
            } else {
                assert_eq!(repaired_min, min);
                let bits = if min > 0.0 {
                    min.to_bits() + 1
                } else {
                    min.to_bits() - 1
                };
                assert_eq!(repaired_max, f64::from_bits(bits));
            }
        }
    }
}

#[test]
fn meter_normalises_reversed_bands_independently_of_setter_order() {
    for meter in [
        Meter::new("x", 5.0, 0.0, 10.0).low(8.0).high(2.0),
        Meter::new("x", 5.0, 0.0, 10.0).high(2.0).low(8.0),
        Meter::new("x", 5.0, 0.0, 10.0).low(10.0).high(2.0).low(8.0),
        Meter::new("x", 5.0, 0.0, 10.0).high(0.0).low(8.0).high(2.0),
    ] {
        let html = to_html(&meter);
        assert_eq!(attr(&html, "<meter", "low"), "8", "{html}");
        assert_eq!(attr(&html, "<meter", "high"), "8", "{html}");
    }
}

#[test]
fn callout_prefixes_title_by_tone() {
    use crate::Tone;
    for (tone, prefix) in [
        (Tone::Warning, Some("Warning: ")),
        (Tone::Info, Some("Note: ")),
        (Tone::Success, Some("Success: ")),
        (Tone::Danger, Some("Important: ")),
        (Tone::Accent, Some("Note: ")),
        (Tone::Default, None),
    ] {
        let html = to_html(&Callout::new(tone).title("Payment overdue"));
        assert!(
            html.contains(&format!(r#"data-tone="{}""#, tone.as_str())),
            "{html}"
        );
        let title = match prefix {
            Some(p) => format!(
                r#"<p class="st-callout-title"><span class="st-sr-only">{p}</span>Payment overdue</p>"#
            ),
            None => r#"<p class="st-callout-title">Payment overdue</p>"#.to_owned(),
        };
        assert!(html.contains(&title), "{html}");
    }
}

#[test]
fn callout_without_title_prefixes_the_body() {
    let html = to_html(&Callout::new(crate::Tone::Warning).child("Check <this>"));
    let prefix = html
        .find(r#"<span class="st-sr-only">Warning: </span>"#)
        .expect("prefix");
    assert!(prefix < html.find("Check &lt;this&gt;").unwrap(), "{html}");
    assert!(!html.contains("st-callout-title"), "{html}");
}

#[test]
fn callout_has_no_role() {
    assert!(!to_html(&Callout::new(crate::Tone::Danger).title("x")).contains("role="));
}

#[test]
fn callout_actions_wrap() {
    let html = to_html(&Callout::new(crate::Tone::Info).title("x").actions("Do it"));
    assert!(
        html.contains(r#"<div class="st-callout-actions">Do it</div>"#),
        "{html}"
    );
    assert!(html.contains(r#"aria-hidden="true""#), "{html}");
}
