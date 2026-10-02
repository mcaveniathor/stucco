//! The stucco gallery: a static site of components, palettes and test
//! fixtures. `cargo run -p gallery -- [dir]` writes it (default
//! `target/gallery`).

use std::fs;
use std::io;
use std::path::Path;

use stucco::theme::Preset;
use stucco::{Asset, Bundle, register_asset};

pub mod actions_page;
pub mod collections_page;
pub mod fixtures;
pub mod forms_page;
pub mod index;
pub mod layout_page;
pub mod palette;
mod shell;
pub mod themes_page;
pub mod typography_page;

/// Layout styles for gallery pages (not part of the library).
pub static GALLERY: Asset = Asset {
    name: "gallery",
    css: Some(include_str!("../css/gallery.css")),
    behavior: None,
    deps: &[],
};
register_asset!(GALLERY);

/// The gallery bundle: Slate at the root, every preset and the seeded themes
/// as named themes.
pub fn bundle() -> Bundle {
    let presets = Preset::ALL.iter().fold(Bundle::new(Preset::Slate), |b, p| {
        b.with_theme(p.name(), *p)
    });
    themes_page::SEEDS
        .iter()
        .fold(presets, |b, (seed, name)| {
            b.with_theme(name, themes_page::seeded(*seed))
        })
        .with_theme(themes_page::MIXED, themes_page::mixed())
}

/// Every gallery page as `(file name, HTML)`, rendered with `bundle` (which
/// must contain the gallery's named themes; see [`bundle`]).
pub fn pages(bundle: &Bundle) -> Vec<(String, String)> {
    let mut pages = vec![
        ("index.html".to_owned(), index::index_page(bundle)),
        ("palette.html".to_owned(), palette::palette_page(bundle)),
        ("themes.html".to_owned(), themes_page::page(bundle)),
        ("layout.html".to_owned(), layout_page::page(bundle)),
        ("typography.html".to_owned(), typography_page::page(bundle)),
        ("actions.html".to_owned(), actions_page::page(bundle)),
        ("forms.html".to_owned(), forms_page::page(bundle)),
        (
            "collections.html".to_owned(),
            collections_page::page(bundle),
        ),
        ("app.html".to_owned(), collections_page::app_page(bundle)),
    ];
    pages.extend(fixtures::fixtures(bundle));
    pages
}

/// Writes every page and bundle file under `dir`.
pub fn write_site(dir: &Path) -> io::Result<()> {
    let bundle = bundle();
    for (name, html) in pages(&bundle) {
        write(&dir.join(name), html.as_bytes())?;
    }
    for path in bundle.paths() {
        let file = bundle.get(&path).expect("listed paths are served");
        write(&dir.join(path.trim_start_matches('/')), &file.bytes)?;
    }
    Ok(())
}

fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)
}
