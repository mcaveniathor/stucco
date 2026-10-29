#![cfg(feature = "ssr")]

mod common;

use common::{align_ids, assert_parity};
use leptos::prelude::*;
use stucco_core::to_html;
use stucco_leptos::{Callout, CalloutActions, EmptyState, Meter, Progress, Tone};
use stucco_ui::feedback::{
    Callout as StaticCallout, EmptyState as StaticEmptyState, Meter as StaticMeter,
    Progress as StaticProgress,
};

fn render(view: impl FnOnce() -> String) -> String {
    Owner::new().with(view)
}

fn assert_aligned(leptos: &str, stucco: &str) {
    assert_parity(&align_ids(leptos, stucco), stucco);
}

#[test]
fn determinate_progress_matches_static_markup() {
    for show_value in [false, true] {
        let html = render(|| {
            view! { <Progress id="p" label="Import" value=42.0 max=100.0 show_value=show_value/> }
                .to_html()
        });
        let mut expected = StaticProgress::new("Import", 42.0, 100.0);
        if show_value {
            expected = expected.show_value();
        }
        assert_aligned(&html, &to_html(&expected));
    }
}

#[test]
fn indeterminate_progress_matches_static_markup() {
    let html = render(|| view! { <Progress id="p" label="Import"/> }.to_html());
    assert_aligned(&html, &to_html(&StaticProgress::indeterminate("Import")));
    assert!(!html.contains("value="), "{html}");
}

#[test]
fn progress_clamps_and_sanitizes_like_the_static_component() {
    for (value, max) in [
        (150.0, 100.0),
        (-5.0, 100.0),
        (f64::NAN, 100.0),
        (f64::INFINITY, 100.0),
        (3.0, 0.0),
        (3.0, -1.0),
        (3.0, f64::NAN),
        (12.5, 50.0),
        (-0.0, 10.0),
    ] {
        let html = render(|| {
            view! { <Progress id="p" label="Import" value=value max=max show_value=true/> }
                .to_html()
        });
        let expected = to_html(&StaticProgress::new("Import", value, max).show_value());
        assert_aligned(&html, &expected);
    }
}

#[test]
fn progress_label_is_escaped() {
    let html = render(|| view! { <Progress id="p" label="<Import>" value=1.0 max=2.0/> }.to_html());
    assert!(html.contains("&lt;Import&gt;"), "{html}");
}

fn meter(value: f64, min: f64, max: f64) -> String {
    render(|| view! { <Meter id="m" label="Disk" value=value min=min max=max/> }.to_html())
}

#[test]
fn meter_matches_static_markup_including_degenerate_ranges() {
    for (value, min, max) in [
        (5.0, 0.0, 10.0),
        (50.0, 0.0, 10.0),
        (-3.0, 0.0, 10.0),
        (f64::NAN, 0.0, 10.0),
        (5.0, 10.0, 0.0),
        (5.0, 3.0, 3.0),
        (5.0, f64::NAN, 10.0),
        (1.0, f64::MAX, f64::MAX),
        (1.0, -f64::MAX, -f64::MAX),
    ] {
        let expected = to_html(&StaticMeter::new("Disk", value, min, max));
        assert_aligned(&meter(value, min, max), &expected);
    }
}

#[test]
fn meter_thresholds_and_text_match_static_markup() {
    let html = render(|| {
        view! {
            <Meter id="m" label="Disk" value=7.0 min=0.0 max=10.0 low=3.0 high=8.0 optimum=1.0
                value_text="70% used"/>
        }
        .to_html()
    });
    let expected = to_html(
        &StaticMeter::new("Disk", 7.0, 0.0, 10.0)
            .low(3.0)
            .high(8.0)
            .optimum(1.0)
            .value_text("70% used"),
    );
    assert_aligned(&html, &expected);
    assert!(html.contains(r#"aria-describedby="m-text""#), "{html}");
}

#[test]
fn meter_thresholds_are_clamped_and_ordered_like_the_static_component() {
    let html = render(|| {
        view! { <Meter id="m" label="Disk" value=5.0 min=0.0 max=10.0 low=8.0 high=2.0 optimum=99.0/> }
            .to_html()
    });
    let expected = to_html(
        &StaticMeter::new("Disk", 5.0, 0.0, 10.0)
            .low(8.0)
            .high(2.0)
            .optimum(99.0),
    );
    assert_aligned(&html, &expected);
}

const TONES: [(Tone, stucco_ui::Tone); 7] = [
    (Tone::Default, stucco_ui::Tone::Default),
    (Tone::Muted, stucco_ui::Tone::Muted),
    (Tone::Accent, stucco_ui::Tone::Accent),
    (Tone::Success, stucco_ui::Tone::Success),
    (Tone::Warning, stucco_ui::Tone::Warning),
    (Tone::Danger, stucco_ui::Tone::Danger),
    (Tone::Info, stucco_ui::Tone::Info),
];

#[test]
fn callout_matches_static_markup_for_every_tone() {
    for (tone, static_tone) in TONES {
        let html = render(|| {
            view! {
                <Callout tone=tone title="Saved">
                    <CalloutActions slot>"Undo"</CalloutActions>
                    "Your changes are live."
                </Callout>
            }
            .to_html()
        });
        let expected = to_html(
            &StaticCallout::new(static_tone)
                .title("Saved")
                .child("Your changes are live.")
                .actions("Undo"),
        );
        assert_parity(&html, &expected);
    }
}

#[test]
fn callout_without_title_or_actions_matches_static_markup() {
    for (tone, static_tone) in TONES {
        let html = render(|| view! { <Callout tone=tone>"Body"</Callout> }.to_html());
        assert_parity(
            &html,
            &to_html(&StaticCallout::new(static_tone).child("Body")),
        );
    }
}

#[test]
fn callout_prefixes_are_translatable_and_text_is_escaped() {
    let html = render(|| {
        view! {
            <Callout tone=Tone::Warning title="<Achtung>" warning_label="Warnung: ">"x"</Callout>
        }
        .to_html()
    });
    assert!(html.contains("Warnung: "), "{html}");
    assert!(!html.contains("Warning: "), "{html}");
    assert!(html.contains("&lt;Achtung&gt;"), "{html}");
}

#[test]
fn empty_state_matches_static_markup() {
    let html = render(|| {
        view! {
            <EmptyState title="No orders" description=|| "Try another filter"
                actions=|| view! { <a href="/orders">"Reset"</a> }/>
        }
        .to_html()
    });
    let expected = to_html(
        &StaticEmptyState::new("No orders")
            .description("Try another filter")
            .actions(stucco_core::el::a().href("/orders").text("Reset")),
    );
    assert_parity(&html, &expected);

    let bare = render(|| view! { <EmptyState title="Nothing"/> }.to_html());
    assert_parity(&bare, &to_html(&StaticEmptyState::new("Nothing")));
}

#[test]
fn empty_state_title_is_escaped() {
    let html = render(|| view! { <EmptyState title="<None>"/> }.to_html());
    assert!(html.contains("&lt;None&gt;"), "{html}");
}

#[test]
fn meter_description_follows_a_reactive_value_text() {
    let html = render(|| {
        let used = RwSignal::new(70.0);
        view! {
            <Meter id="m" label="Disk" value=used min=0.0 max=100.0
                value_text=Signal::derive(move || Some(format!("{}% used", used.get())))/>
        }
        .to_html()
    });
    assert!(html.contains("70% used"), "{html}");
    assert!(html.contains(r#"aria-describedby="m-text""#), "{html}");
}

#[test]
fn meter_without_value_text_has_no_description() {
    let html =
        render(|| view! { <Meter id="m" label="Disk" value=1.0 min=0.0 max=2.0/> }.to_html());
    assert!(!html.contains("aria-describedby"), "{html}");
    assert!(!html.contains("m-text"), "{html}");
}
