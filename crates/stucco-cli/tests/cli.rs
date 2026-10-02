//! Runs the built `stucco` binary.

use std::process::{Command, Output};

fn stucco(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_stucco"))
        .args(args)
        .output()
        .expect("the binary runs")
}

fn stdout(args: &[&str]) -> String {
    let out = stucco(args);
    assert!(
        out.status.success(),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn css_is_the_default_format() {
    let css = stdout(&["theme", "--preset", "pine"]);
    assert!(css.starts_with("@layer stucco.tokens {"));
    assert!(css.contains(":root {"));
    assert!(css.ends_with("}\n"));
    let scoped = stdout(&["theme", "--seed", "3", "--scope", "brand"]);
    assert!(scoped.contains("[data-st-theme=\"brand\"]"));
    assert!(!scoped.contains(":root {"));
}

#[test]
fn flags_and_queries_describe_the_same_theme() {
    let from_flags = stdout(&["theme", "--seed", "42", "--radius", "round", "-f", "query"]);
    assert_eq!(from_flags, "seed=42&radius=round\n");
    let from_query = stdout(&["theme", "radius=round&seed=42", "-f", "query"]);
    assert_eq!(from_query, from_flags);
    // Flags win over the query.
    let mixed = stdout(&[
        "theme",
        "seed=42&radius=round",
        "--preset",
        "sol",
        "--radius",
        "sharp",
        "-f",
        "query",
    ]);
    assert_eq!(mixed, "preset=sol&radius=sharp\n");
}

#[test]
fn every_format_renders() {
    let rust = stdout(&["theme", "--name", "acme", "--table", "open", "-f", "rust"]);
    assert!(rust.contains("Theme::seeded_str(\"acme\")\n    .table_style(TableStyle::Open);"));
    let json = stdout(&["theme", "-f", "json"]);
    assert!(json.starts_with("{\n  \"light\": {"));
    let summary = stdout(&[
        "theme",
        "--preset",
        "slate",
        "--buttons",
        "pill",
        "-f",
        "summary",
    ]);
    assert!(summary.starts_with("Slate\npreset=slate&buttons=pill\n"));
    assert!(summary.contains("Buttons    Pill  (set)\n"));
    assert!(summary.contains("Elevation  Outlined\n"));
}

#[test]
fn random_themes_report_their_seed() {
    let out = stucco(&["theme", "--random", "-f", "query"]);
    assert!(out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    let seed = stderr
        .trim()
        .strip_prefix("seed: ")
        .expect("the seed is reported");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("seed={seed}\n")
    );
}

#[test]
fn output_files_are_written() {
    let dir = std::env::temp_dir().join(format!("stucco-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("theme.css");
    assert_eq!(stdout(&["theme", "-o", path.to_str().unwrap()]), "");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), stdout(&["theme"]));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn mistakes_fail_with_a_message() {
    let bad = |args: &[&str], code: i32, message: &str| {
        let out = stucco(args);
        assert_eq!(out.status.code(), Some(code), "{args:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(stderr.contains(message), "{args:?}: {stderr}");
    };
    bad(&["theme", "seed=1&bogus=2"], 1, "unknown key `bogus`");
    bad(
        &["theme", "seed=1&preset=sol"],
        1,
        "only one of preset, seed and name",
    );
    bad(&["theme", "--preset", "nope"], 2, "possible values");
    bad(
        &["theme", "--seed", "1", "--preset", "sol"],
        2,
        "cannot be used with",
    );
    bad(&["theme", "--scope", "Brand!"], 2, "lowercase letters");
}

#[test]
fn lists_presets_and_options() {
    let presets = stdout(&["presets"]);
    assert!(presets.starts_with("slate\n"));
    assert_eq!(presets.lines().count(), 14);
    let options = stdout(&["options"]);
    assert!(options.contains("radius     sharp, soft, round\n"));
}
