//! The site's shared frame: header navigation, the theme menu, the footer,
//! and the head script that applies a saved theme before first paint.

use stucco::actions::{Button, ButtonLink};
use stucco::app::{AppShell, Footer};
use stucco::forms::{Field, Select};
use stucco::layout::Cluster;
use stucco::theme::Preset;
use stucco::{Bundle, Page, Raw, Render, Space, Variant, el, to_html};

/// The top-level sections, for marking the current one in the header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    /// The landing page.
    Home,
    /// The guide.
    Guide,
    /// The theme playground.
    Playground,
    /// Anything else.
    Other,
}

/// Site-wide settings: the URL base and the shared asset bundle.
#[derive(Debug)]
pub struct Site {
    base: String,
    bundle: Bundle,
}

/// localStorage key holding the visitor's site theme.
pub const THEME_KEY: &str = "stucco-site-theme";

impl Site {
    /// A site served under `base` (for example `/stucco/` on GitHub Pages).
    pub fn new(base: &str) -> Site {
        let trimmed = base.trim_matches('/');
        let base = if trimmed.is_empty() {
            "/".to_owned()
        } else {
            format!("/{trimmed}/")
        };
        let bundle = gallery::bundle().prefix(&format!("{base}_stucco"));
        Site { base, bundle }
    }

    /// The absolute URL of site path `path` (no leading slash).
    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    /// The URL base, with leading and trailing slashes.
    pub fn base(&self) -> &str {
        &self.base
    }

    /// The asset bundle shared by every page.
    pub fn bundle(&self) -> &Bundle {
        &self.bundle
    }

    /// Head markup for every page: favicon, the saved-theme script and the
    /// site module. The inline script only reads localStorage, so a theme
    /// applies before first paint without a flash.
    fn head(&self) -> String {
        format!(
            concat!(
                r#"<link rel="icon" href="{icon}" type="image/svg+xml">"#,
                "<script>try{{const s=JSON.parse(localStorage.getItem(\"{key}\")||\"null\");",
                "const r=document.documentElement;",
                "if(s&&s.css){{const e=document.createElement(\"style\");e.id=\"site-theme-css\";",
                "e.textContent=s.css;document.head.append(e);r.dataset.stTheme=\"site-custom\"}}",
                "else if(s&&s.preset){{r.dataset.stTheme=s.preset}}}}catch(_){{}}</script>",
                r#"<script type="module" src="{js}"></script>"#,
            ),
            icon = self.url("assets/favicon.svg"),
            key = THEME_KEY,
            js = self.url("assets/site.js"),
        )
    }

    /// A full site page: header, optional sidebar, `main`, footer.
    pub fn page<'a>(
        &self,
        title: &str,
        description: &str,
        section: Section,
        sidebar: Option<Box<dyn Render + 'a>>,
        main: impl Render + 'a,
    ) -> String {
        self.page_with_head(title, description, section, sidebar, String::new(), main)
    }

    /// [`Site::page`] with extra trusted `<head>` markup.
    pub fn page_with_head<'a>(
        &self,
        title: &str,
        description: &str,
        section: Section,
        sidebar: Option<Box<dyn Render + 'a>>,
        extra_head: String,
        main: impl Render + 'a,
    ) -> String {
        let title = if title == "stucco" {
            "stucco: server-rendered UI for Rust".to_owned()
        } else {
            format!("{title} · stucco")
        };
        let mut shell = AppShell::new()
            .header(self.header(section))
            .main(main)
            .footer(self.footer());
        if let Some(sidebar) = sidebar {
            shell = shell.sidebar(sidebar);
        }
        Page::new(&self.bundle, title)
            .meta(stucco::Meta::description(description))
            .head(Raw::trusted(self.head() + &extra_head))
            .body(shell)
            .render()
    }

    /// Adds the site's head markup and a floating theme menu to a page
    /// rendered elsewhere (the gallery).
    pub fn with_chrome(&self, html: &str) -> String {
        let head_end = html.find("</head>").expect("pages have a head");
        let body_end = html.rfind("</body>").expect("pages have a body");
        let mut out = String::with_capacity(html.len() + 4096);
        out.push_str(&html[..head_end]);
        out.push_str(&self.head());
        out.push_str(&html[head_end..body_end]);
        // Last in the body, so the page's skip link stays the first stop.
        out.push_str(&to_html(
            &el::aside()
                .aria("label", "Site theme")
                .child(theme_menu(self, true)),
        ));
        out.push_str(&html[body_end..]);
        out
    }

    fn header(&self, section: Section) -> impl Render + 'static {
        let link = |label: &'static str, path: &str, current: bool| {
            let a = el::a().href(self.url(path)).text(label);
            if current {
                a.aria("current", "page")
            } else {
                a
            }
        };
        el::div()
            .class("site-header")
            .child(link("stucco", "", section == Section::Home).class("site-brand"))
            .child(
                el::nav()
                    .class("site-nav")
                    .aria("label", "Site")
                    .child(link("Guide", "guide/", section == Section::Guide))
                    .child(link(
                        "Playground",
                        "playground.html",
                        section == Section::Playground,
                    ))
                    .child(link("Gallery", "gallery/", false))
                    .child(link("API", "api/stucco/", false))
                    .child(
                        el::a()
                            .href("https://github.com/mcaveniathor/stucco")
                            .text("GitHub"),
                    ),
            )
            .child(theme_menu(self, false))
    }

    fn footer(&self) -> impl Render + 'static {
        Footer::new().child(
            el::div()
                .class("site-footer")
                .child(el::p().text("stucco is dual-licensed under MIT and Apache-2.0."))
                .child(
                    el::nav()
                        .aria("label", "Elsewhere")
                        .child(
                            el::a()
                                .href("https://crates.io/crates/stucco")
                                .text("crates.io"),
                        )
                        .child(el::a().href("https://docs.rs/stucco").text("docs.rs"))
                        .child(
                            el::a()
                                .href("https://github.com/mcaveniathor/stucco")
                                .text("Source"),
                        )
                        .child(
                            el::a()
                                .href(
                                    "https://github.com/mcaveniathor/stucco/blob/main/CHANGELOG.md",
                                )
                                .text("Changelog"),
                        ),
                ),
        )
    }
}

/// The theme menu: a disclosure with the theme and scheme pickers. Hidden
/// until the site script runs, because it needs JavaScript to work.
pub fn theme_menu(site: &Site, floating: bool) -> impl Render + 'static {
    let presets = Preset::ALL.map(|p| (p.name().to_owned(), title_case(p.name())));
    el::details()
        .class(if floating {
            "site-theme site-theme-floating"
        } else {
            "site-theme"
        })
        .bool_attr("hidden", true)
        .data("site-theme", "menu")
        .child(el::summary().text("Theme"))
        .child(
            el::div()
                .class("site-theme-panel")
                .child(Field::new(
                    "Theme",
                    Select::new("site-theme")
                        .id("site-theme-preset")
                        .options(presets)
                        .option("custom", "Custom"),
                ))
                .child(Field::new(
                    "Colour scheme",
                    Select::new("site-scheme").id("site-theme-scheme").options([
                        ("system", "Match the system"),
                        ("light", "Light"),
                        ("dark", "Dark"),
                    ]),
                ))
                .child(
                    Cluster::new()
                        .space(Space::S2)
                        .child(
                            Button::new("Random theme")
                                .variant(Variant::Primary)
                                .data("site-action", "random"),
                        )
                        .child(Button::new("Reset").data("site-action", "reset")),
                )
                .child(
                    el::p()
                        .class("site-theme-status")
                        .aria("live", "polite")
                        .data("site-theme", "status"),
                )
                .child(
                    ButtonLink::new("Edit in the playground", site.url("playground.html"))
                        .variant(Variant::Ghost)
                        .data("site-action", "edit"),
                ),
        )
}

/// `slate` → `Slate`.
pub fn title_case(name: &str) -> String {
    let mut chars = name.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}
