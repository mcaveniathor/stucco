#![cfg(feature = "ssr")]

mod common;

use common::assert_parity;
use leptos::prelude::*;
use stucco_core::to_html;
use stucco_leptos::{
    Avatar, Badge, Card, DescriptionItem, DescriptionList, Panel, Size, Sparkline, Stat,
    StatChange, Timeline, TimelineItem, Tone, Trend,
};
use stucco_ui::data::{
    Avatar as StaticAvatar, Badge as StaticBadge, Card as StaticCard,
    DescriptionList as StaticDescriptionList, Panel as StaticPanel, Sparkline as StaticSparkline,
    Stat as StaticStat, Timeline as StaticTimeline, TimelineEntry,
};

fn render(view: impl FnOnce() -> String) -> String {
    Owner::new().with(view)
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
fn badge_matches_static_markup_for_every_tone() {
    for (tone, static_tone) in TONES {
        let html = render(|| view! { <Badge tone=tone>"Paid"</Badge> }.to_html());
        assert_parity(&html, &to_html(&StaticBadge::new("Paid").tone(static_tone)));
    }
}

#[test]
fn badge_defaults_to_the_default_tone() {
    let html = render(|| view! { <Badge>"Draft"</Badge> }.to_html());
    assert_parity(&html, &to_html(&StaticBadge::new("Draft")));
}

#[test]
fn badge_escapes_its_label() {
    let html = render(|| view! { <Badge>"<b>&"</Badge> }.to_html());
    assert!(html.contains("&lt;b&gt;&amp;"), "{html}");
}

#[test]
fn avatar_matches_static_markup() {
    let initials = render(|| view! { <Avatar name="Ada Lovelace"/> }.to_html());
    assert_parity(&initials, &to_html(&StaticAvatar::new("Ada Lovelace")));

    let image =
        render(|| view! { <Avatar name="Ada Lovelace" image="/people/ada.jpg"/> }.to_html());
    assert_parity(
        &image,
        &to_html(&StaticAvatar::new("Ada Lovelace").image("/people/ada.jpg")),
    );

    let decorative_image = render(|| {
        view! { <Avatar name="Ada Lovelace" image="/people/ada.jpg" decorative=true/> }.to_html()
    });
    assert_parity(
        &decorative_image,
        &to_html(
            &StaticAvatar::new("Ada Lovelace")
                .image("/people/ada.jpg")
                .decorative(),
        ),
    );

    let decorative = render(|| view! { <Avatar name="Ada Lovelace" decorative=true/> }.to_html());
    assert_parity(
        &decorative,
        &to_html(&StaticAvatar::new("Ada Lovelace").decorative()),
    );
}

#[test]
fn avatar_sizes_map_like_the_static_component() {
    for (size, static_size) in [
        (Size::Xs, stucco_ui::Size::Xs),
        (Size::Sm, stucco_ui::Size::Sm),
        (Size::Md, stucco_ui::Size::Md),
        (Size::Lg, stucco_ui::Size::Lg),
        (Size::Xl, stucco_ui::Size::Xl),
    ] {
        let html = render(|| view! { <Avatar name="Ada Lovelace" size=size/> }.to_html());
        assert_parity(
            &html,
            &to_html(&StaticAvatar::new("Ada Lovelace").size(static_size)),
        );
    }
}

#[test]
fn avatar_initials_follow_the_static_rule() {
    for name in [
        "Ada Lovelace",
        "ada",
        "  ",
        "",
        "Émile Zola Jr",
        "Ada Byron Lovelace",
    ] {
        let html = render(|| {
            let name = name.to_owned();
            view! { <Avatar name=name/> }.to_html()
        });
        assert_parity(&html, &to_html(&StaticAvatar::new(name)));
    }
}

#[test]
fn avatar_image_source_is_sanitized_and_name_escaped() {
    let html = render(|| view! { <Avatar name="<Ada>" image="javascript:alert(1)"/> }.to_html());
    assert!(!html.contains("javascript"), "{html}");
    assert!(html.contains(r##"src="#""##), "{html}");
    assert!(html.contains("&lt;Ada&gt;"), "{html}");
    assert_parity(
        &html,
        &to_html(&StaticAvatar::new("<Ada>").image("javascript:alert(1)")),
    );
}

#[test]
fn description_list_matches_static_markup() {
    let html = render(|| {
        view! {
            <DescriptionList>
                <DescriptionItem term="Customer">"Ada"</DescriptionItem>
                <DescriptionItem term="Status"><Badge tone=Tone::Success>"Paid"</Badge></DescriptionItem>
            </DescriptionList>
        }
        .to_html()
    });
    let expected = to_html(&StaticDescriptionList::new().item("Customer", "Ada").item(
        "Status",
        StaticBadge::new("Paid").tone(stucco_ui::Tone::Success),
    ));
    assert_parity(&html, &expected);
}

#[test]
fn description_list_without_items_is_an_empty_dl() {
    let html = render(|| view! { <DescriptionList/> }.to_html());
    assert_parity(&html, &to_html(&StaticDescriptionList::new()));
}

#[test]
fn description_details_accept_views_and_escape_terms() {
    let html = render(|| {
        view! {
            <DescriptionList>
                <DescriptionItem term="<Link>"><a href="/o/1">"Order"</a></DescriptionItem>
            </DescriptionList>
        }
        .to_html()
    });
    assert!(html.contains("&lt;Link&gt;"), "{html}");
    assert!(html.contains(r#"<a href="/o/1">Order</a>"#), "{html}");
}

fn change(text: &str, trend: Trend) -> Option<StatChange> {
    Some(StatChange {
        text: text.to_owned(),
        trend,
    })
}

#[test]
fn stat_matches_static_markup() {
    let plain = render(|| view! { <Stat label="Orders" value="128"/> }.to_html());
    assert_parity(&plain, &to_html(&StaticStat::new("Orders", "128")));

    for (trend, static_trend) in [
        (Trend::Up, stucco_ui::Trend::Up),
        (Trend::Down, stucco_ui::Trend::Down),
        (Trend::Flat, stucco_ui::Trend::Flat),
    ] {
        let html = render(|| {
            view! {
                <Stat label="Orders" value="128" change=change("12%", trend)
                    change_tone=Tone::Success context="vs last week"/>
            }
            .to_html()
        });
        let expected = to_html(
            &StaticStat::new("Orders", "128")
                .change("12%", static_trend)
                .change_tone(stucco_ui::Tone::Success)
                .context("vs last week"),
        );
        assert_parity(&html, &expected);
    }
}

#[test]
fn stat_change_without_context_matches_static_markup() {
    let html = render(|| {
        view! { <Stat label="Orders" value="128" change=change("12%", Trend::Up)/> }.to_html()
    });
    let expected = to_html(&StaticStat::new("Orders", "128").change("12%", stucco_ui::Trend::Up));
    assert_parity(&html, &expected);
}

#[test]
fn stat_text_is_escaped() {
    let html = render(|| {
        view! {
            <Stat label="<L>" value="<V>" change=change("<C>", Trend::Down) context="<X>"/>
        }
        .to_html()
    });
    for escaped in ["&lt;L&gt;", "&lt;V&gt;", "&lt;C&gt;", "&lt;X&gt;"] {
        assert!(html.contains(escaped), "{html}");
    }
}

#[test]
fn stat_direction_text_is_translatable() {
    let html = render(|| {
        view! {
            <Stat label="Commandes" value="9" change=change("2%", Trend::Down)
                down_label="Baisse de "/>
        }
        .to_html()
    });
    assert!(html.contains("Baisse de "), "{html}");
    assert!(!html.contains("Down "), "{html}");
}

#[test]
fn card_matches_static_markup() {
    let full = render(|| view! { <Card header=|| "Head" footer=|| "Foot">"Body"</Card> }.to_html());
    let expected = to_html(
        &StaticCard::new()
            .header("Head")
            .child("Body")
            .footer("Foot"),
    );
    assert_parity(&full, &expected);

    let body_only = render(|| view! { <Card>"Body"</Card> }.to_html());
    assert_parity(&body_only, &to_html(&StaticCard::new().child("Body")));

    let empty = render(|| view! { <Card/> }.to_html());
    assert_parity(&empty, &to_html(&StaticCard::new()));
}

/// The id the static panel generated for its heading.
fn heading_id(html: &str) -> String {
    let start = html.find(" id=\"").expect("panel heading id") + 5;
    html[start..].split('"').next().unwrap().to_owned()
}

/// The Leptos panel names its heading `panel-title`; the static one generates an
/// id. Map the first to the second so the rest of the markup can be compared.
fn with_static_id(html: &str, static_html: &str) -> String {
    html.replace("panel-title", &heading_id(static_html))
}

#[test]
fn panel_matches_static_markup() {
    let static_html = to_html(
        &StaticPanel::new("Recent orders")
            .description("Latest first")
            .actions("Actions")
            .body("Body")
            .footer("Foot"),
    );
    let html = render(|| {
        view! {
            <Panel id="panel" title="Recent orders" description=|| "Latest first"
                actions=|| "Actions" footer=|| "Foot">"Body"</Panel>
        }
        .to_html()
    });
    assert_parity(&with_static_id(&html, &static_html), &static_html);
}

#[test]
fn panel_without_optional_parts_matches_static_markup() {
    let static_html = to_html(&StaticPanel::new("Bare"));
    let html = render(|| view! { <Panel id="panel" title="Bare"/> }.to_html());
    assert_parity(&with_static_id(&html, &static_html), &static_html);
}

#[test]
fn panel_heading_level_is_clamped_to_one_through_six() {
    for (level, tag) in [(0u8, "h1"), (1, "h1"), (3, "h3"), (6, "h6"), (9, "h6")] {
        let html = render(|| view! { <Panel id="panel" title="T" level=level/> }.to_html());
        assert!(html.contains(&format!("<{tag} ")), "{level}: {html}");
        let static_html = to_html(&StaticPanel::new("T").level(level));
        assert_parity(&with_static_id(&html, &static_html), &static_html);
    }
}

#[test]
fn panel_is_labelled_by_its_heading() {
    let html = render(|| view! { <Panel id="orders-panel" title="Orders"/> }.to_html());
    assert!(
        html.contains(r#"aria-labelledby="orders-panel-title""#),
        "{html}"
    );
    assert!(html.contains(r#"id="orders-panel-title""#), "{html}");
}

#[test]
fn panel_title_is_escaped() {
    let html = render(|| view! { <Panel id="p" title="<Orders> & more"/> }.to_html());
    assert!(html.contains("&lt;Orders&gt; &amp; more"), "{html}");
}

#[test]
fn timeline_items_render_semantic_times_and_arbitrary_content() {
    let html = render(|| {
        view! {
            <Timeline label="Order history">
                <TimelineItem datetime="2026-10-01T09:00" time="1 Oct, 09:00">"Created"</TimelineItem>
                <TimelineItem datetime="2026-10-02" time="2 Oct">
                    <a href="/o/1">"Shipped"</a>
                </TimelineItem>
            </Timeline>
        }
        .to_html()
    });
    assert!(
        html.contains(r#"<time datetime="2026-10-01T09:00">1 Oct, 09:00</time>"#),
        "{html}"
    );
    assert!(html.contains(r#"<a href="/o/1">Shipped</a>"#), "{html}");
}

#[test]
fn timeline_structure_matches_static_for_plain_text_entries() {
    let html = render(|| {
        view! {
            <Timeline label="History">
                <TimelineItem datetime="2026-10-01" time="1 Oct">"Created"</TimelineItem>
            </Timeline>
        }
        .to_html()
    });
    let expected = to_html(&StaticTimeline::new("History").entry(TimelineEntry::new(
        "2026-10-01",
        "1 Oct",
        "Created",
    )));
    assert_parity(&html, &expected);
}

#[test]
fn empty_timeline_shows_the_empty_item() {
    let html = render(|| view! { <Timeline label="History"/> }.to_html());
    assert_parity(&html, &to_html(&StaticTimeline::new("History")));
    assert!(html.contains("No activity yet"), "{html}");
}

#[test]
fn timeline_empty_text_is_translatable() {
    let html = render(|| {
        view! { <Timeline label="Verlauf" empty_label="Noch keine Aktivität"/> }.to_html()
    });
    assert!(html.contains("Noch keine Aktivität"), "{html}");
    assert!(!html.contains("No activity yet"), "{html}");
}

fn sparkline(values: Vec<f64>) -> String {
    render(|| view! { <Sparkline summary="Orders per day" values=values/> }.to_html())
}

fn static_sparkline(values: Vec<f64>) -> String {
    to_html(&StaticSparkline::new("Orders per day", values))
}

#[test]
fn sparkline_matches_static_markup() {
    for values in [
        vec![1.0, 2.0, 3.0],
        vec![5.0],
        vec![2.0, 2.0, 2.0],
        vec![],
        vec![3.5, -1.25, 8.0, 0.0],
    ] {
        assert_parity(&sparkline(values.clone()), &static_sparkline(values));
    }
}

#[test]
fn sparkline_drops_non_finite_samples() {
    let with_junk = vec![1.0, f64::NAN, 3.0, f64::INFINITY, f64::NEG_INFINITY];
    assert_parity(&sparkline(with_junk), &sparkline(vec![1.0, 3.0]));
    assert_parity(&sparkline(vec![f64::NAN]), &sparkline(vec![]));
}

#[test]
fn sparkline_survives_opposite_extrema() {
    let values = vec![f64::MAX, -f64::MAX];
    let html = sparkline(values.clone());
    assert!(!html.contains("NaN") && !html.contains("inf"), "{html}");
    assert_parity(&html, &static_sparkline(values));
}

#[test]
fn sparkline_lists_its_values_as_text() {
    let html = sparkline(vec![1.0, 2.5]);
    assert!(html.contains("Values in order: 1, 2.5"), "{html}");
}

#[test]
fn sparkline_text_is_translatable_and_escaped() {
    let html = render(|| {
        view! {
            <Sparkline summary="<Résumé>" values=vec![] no_data_label="Keine Daten"/>
        }
        .to_html()
    });
    assert!(html.contains("&lt;Résumé&gt;"), "{html}");
    assert!(html.contains("Keine Daten"), "{html}");
    let html = render(|| {
        view! { <Sparkline summary="s" values=vec![1.0, 2.0] values_label="Werte: "/> }.to_html()
    });
    assert!(html.contains("Werte: 1, 2"), "{html}");
}

#[test]
fn timeline_built_from_an_empty_list_shows_the_empty_item() {
    let entries: Vec<(&str, &str)> = Vec::new();
    let html = render(|| {
        view! {
            <Timeline label="History" empty=entries.is_empty()>
                {entries.into_iter()
                    .map(|(datetime, time)| view! { <TimelineItem datetime=datetime time=time>"x"</TimelineItem> })
                    .collect_view()}
            </Timeline>
        }
        .to_html()
    });
    assert_parity(&html, &to_html(&StaticTimeline::new("History")));
}

#[test]
fn timeline_with_entries_ignores_the_empty_flag_being_false() {
    let html = render(|| {
        view! {
            <Timeline label="History" empty=false>
                <TimelineItem datetime="2026-10-01" time="1 Oct">"Created"</TimelineItem>
            </Timeline>
        }
        .to_html()
    });
    assert!(!html.contains("No activity yet"), "{html}");
}
