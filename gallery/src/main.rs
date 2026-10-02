//! Writes the gallery site: `cargo run -p gallery -- [dir]`.

use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    let dir = std::env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("target/gallery"), PathBuf::from);
    gallery::write_site(&dir)?;
    println!("wrote {}", dir.display());
    Ok(())
}
