//! The guide: Markdown pages from `docs/guide`, rendered into the site frame
//! with a page list and an outline of the current page.

use pulldown_cmark::{CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd, html};
use stucco::data::Card;
use stucco::layout::{Grid, Stack};
use stucco::typography::{Heading, Text};
use stucco::{Measure, Raw, Render, Size, Space, Tone, el};

use crate::shell::{Section, Site};

/// `(slug, Markdown source)` in reading order. The first line of each file
/// is its `# Title`, and the first paragraph its summary.
const PAGES: &[(&str, &str)] = &[
    (
        "getting-started",
        include_str!("../../docs/guide/getting-started.md"),
    ),
    ("components", include_str!("../../docs/guide/components.md")),
    ("forms", include_str!("../../docs/guide/forms.md")),
    (
        "collections",
        include_str!("../../docs/guide/collections.md"),
    ),
    ("theming", include_str!("../../docs/guide/theming.md")),
    ("cli", include_str!("../../docs/guide/cli.md")),
    (
        "enhancement",
        include_str!("../../docs/guide/enhancement.md"),
    ),
];

/// A parsed guide page.
struct GuidePage {
    slug: &'static str,
    title: String,
    summary: String,
    html: String,
    outline: Vec<(String, String)>,
}

/// Every guide page plus the guide index, as `(path, HTML)`.
pub fn pages(site: &Site) -> Vec<(String, String)> {
    let parsed: Vec<GuidePage> = PAGES.iter().map(|(slug, md)| parse(slug, md)).collect();
    let mut out = vec![("guide/index.html".to_owned(), index(site, &parsed))];
    for (i, page) in parsed.iter().enumerate() {
        let previous = i.checked_sub(1).map(|j| &parsed[j]);
        let next = parsed.get(i + 1);
        let pager = el::nav()
            .class("site-pager")
            .aria("label", "Guide pages")
            .child(previous.map(|p| {
                el::a()
                    .href(format!("{}.html", p.slug))
                    .attr("rel", "prev")
                    .text(format!("← {}", p.title))
            }))
            .child(next.map(|p| {
                el::a()
                    .href(format!("{}.html", p.slug))
                    .attr("rel", "next")
                    .text(format!("{} →", p.title))
            }));
        let main = el::article()
            .class("site-prose")
            .child(el::h1().text(page.title.clone()))
            .child(Raw::trusted(page.html.clone()))
            .child(pager);
        out.push((
            format!("guide/{}.html", page.slug),
            site.page(
                &page.title,
                &page.summary,
                Section::Guide,
                Some(Box::new(nav(&parsed, Some(page.slug)))),
                main,
            ),
        ));
    }
    out
}

fn index(site: &Site, pages: &[GuidePage]) -> String {
    let cards = Grid::new()
        .min(Measure::Xs)
        .space(Space::S4)
        .children(pages.iter().map(|p| {
            Card::new().class("site-link-card").child(
                Stack::new()
                    .space(Space::S2)
                    .child(
                        Heading::new(
                            2,
                            el::a()
                                .class("site-card-link")
                                .href(format!("{}.html", p.slug))
                                .text(p.title.clone()),
                        )
                        .size(Size::Lg),
                    )
                    .child(Text::new(p.summary.clone()).tone(Tone::Muted)),
            )
        }));
    site.page(
        "Guide",
        "How to build interfaces with stucco, from the first page to themes and progressive enhancement.",
        Section::Guide,
        Some(Box::new(nav(pages, None))),
        Stack::new()
            .space(Space::S6)
            .child(
                Stack::new()
                    .space(Space::S2)
                    .child(Heading::new(1, "Guide"))
                    .child(
                        Text::new(
                            "Start with Getting started, then read the topics you need. The API \
                             reference covers every type in detail.",
                        )
                        .tone(Tone::Muted)
                        .size(Size::Lg),
                    ),
            )
            .child(cards),
    )
}

/// The guide's page list, with the current page's outline under it.
fn nav(pages: &[GuidePage], current: Option<&str>) -> impl Render + 'static {
    el::nav()
        .class("site-guide-nav")
        .aria("label", "Guide")
        .child(el::ul().children(pages.iter().map(|p| {
            let is_current = Some(p.slug) == current;
            let link = el::a()
                .href(format!("{}.html", p.slug))
                .text(p.title.clone());
            el::li()
                .child(if is_current {
                    link.aria("current", "page")
                } else {
                    link
                })
                .child(is_current.then(|| {
                    el::ul()
                        .class("site-outline")
                        .children(p.outline.iter().map(|(id, text)| {
                            el::li().child(el::a().href(format!("#{id}")).text(text.clone()))
                        }))
                }))
        })))
}

/// Splits off the title, renders the rest, gives every `##`/`###` heading an
/// id from its text, and collects the `##` headings as an outline.
fn parse(slug: &'static str, source: &str) -> GuidePage {
    let (first, body) = source.split_once('\n').unwrap_or((source, ""));
    let title = first
        .strip_prefix("# ")
        .unwrap_or_else(|| panic!("{slug}.md must start with a `# ` title"))
        .trim()
        .to_owned();
    let summary = body
        .trim_start()
        .split("\n\n")
        .next()
        .unwrap_or_default()
        .replace('\n', " ")
        .replace('`', "");
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let events: Vec<Event> = Parser::new_ext(body, options).collect();
    let mut outline = Vec::new();
    let mut used = Vec::<String>::new();
    let mut out = Vec::with_capacity(events.len());
    let mut i = 0;
    while i < events.len() {
        if let Event::Start(Tag::Heading { level, .. }) = &events[i] {
            let level = *level;
            let mut text = String::new();
            let mut j = i + 1;
            while !matches!(events[j], Event::End(TagEnd::Heading(_))) {
                if let Event::Text(t) | Event::Code(t) = &events[j] {
                    text.push_str(t);
                }
                j += 1;
            }
            let mut id = slugify(&text);
            let base = id.clone();
            let mut n = 2;
            while used.contains(&id) {
                id = format!("{base}-{n}");
                n += 1;
            }
            used.push(id.clone());
            if level == HeadingLevel::H2 {
                outline.push((id.clone(), text));
            }
            out.push(Event::Start(Tag::Heading {
                level,
                id: Some(CowStr::from(id)),
                classes: Vec::new(),
                attrs: Vec::new(),
            }));
            i += 1;
            continue;
        }
        out.push(events[i].clone());
        i += 1;
    }
    let mut html_out = String::new();
    html::push_html(&mut html_out, out.into_iter());
    // Wide code blocks scroll, so keyboard users must be able to reach them.
    let html_out = html_out.replace("<pre>", r#"<pre tabindex="0">"#);
    GuidePage {
        slug,
        title,
        summary,
        html: html_out,
        outline,
    }
}

/// `Forms & validation` → `forms-validation`.
fn slugify(text: &str) -> String {
    let mut slug = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_get_unique_ids_and_h2s_form_the_outline() {
        let page = parse(
            "x",
            "# Title\n\nSummary line\nwrapped.\n\n## First `step`\n\ntext\n\n### Detail\n\n## First step\n",
        );
        assert_eq!(page.title, "Title");
        assert_eq!(page.summary, "Summary line wrapped.");
        assert!(
            page.html
                .contains(r#"<h2 id="first-step">First <code>step</code></h2>"#)
        );
        assert!(page.html.contains(r#"<h3 id="detail">Detail</h3>"#));
        assert!(page.html.contains(r#"<h2 id="first-step-2">"#));
        assert_eq!(
            page.outline,
            [
                ("first-step".to_owned(), "First step".to_owned()),
                ("first-step-2".to_owned(), "First step".to_owned()),
            ]
        );
    }

    #[test]
    fn every_guide_page_parses() {
        for (slug, md) in PAGES {
            let page = parse(slug, md);
            assert!(!page.summary.is_empty(), "{slug} has no summary");
            assert!(!page.outline.is_empty(), "{slug} has no sections");
        }
    }
}
