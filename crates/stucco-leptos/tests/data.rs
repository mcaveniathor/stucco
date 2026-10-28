#![cfg(feature = "ssr")]

mod common;

use common::assert_parity;
use leptos::prelude::*;
use stucco_core::to_html;
use stucco_leptos::{
    Avatar, Badge, DescriptionItem, DescriptionList, Size, Stat, StatChange, Tone, Trend,
};
use stucco_ui::data::{
    Avatar as StaticAvatar, Badge as StaticBadge, DescriptionList as StaticDescriptionList,
    Stat as StaticStat,
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
