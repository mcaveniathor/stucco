#![cfg(feature = "ssr")]

mod common;

use common::assert_parity;
use leptos::prelude::*;
use stucco_core::to_html;
use stucco_leptos::{Avatar, Badge, Size, Tone};
use stucco_ui::data::{Avatar as StaticAvatar, Badge as StaticBadge};

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
