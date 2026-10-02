//! The stucco gallery: a static site of components, palettes and test
//! fixtures. `cargo run -p gallery -- [dir]` writes it (default
//! `target/gallery`).

use std::fs;
use std::io;
use std::path::Path;

use stucco::theme::Preset;
use stucco::{Asset, Bundle, register_asset};

pub mod fixtures;
pub mod index;
pub mod palette;

/// Layout styles for gallery pages (not part of the library).
pub static GALLERY: Asset = Asset {
    name: "gallery",
    css: Some(include_str!("../css/gallery.css")),
    behavior: None,
    deps: &[],
};
register_asset!(GALLERY);

/// The gallery bundle: Slate at the root plus every preset as a named theme.
pub fn bundle() -> Bundle {
    Preset::ALL.iter().fold(Bundle::new(Preset::Slate), |b, p| {
        b.with_theme(p.name(), *p)
    })
}

/// Writes every page and bundle file under `dir`.
pub fn write_site(dir: &Path) -> io::Result<()> {
    let bundle = bundle();
    let mut pages = vec![
        ("index.html".to_owned(), index::index_page(&bundle)),
        ("palette.html".to_owned(), palette::palette_page(&bundle)),
    ];
    pages.extend(fixtures::fixtures(&bundle));
    for (name, html) in pages {
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
