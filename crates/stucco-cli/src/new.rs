//! `stucco new`: a starter axum app with a theme, a shared layout, a home
//! page and a table.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::builder::PossibleValuesParser;
use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command, value_parser};
use stucco_theme::{Preset, random_seed};

const MAIN: &str = include_str!("templates/main.rs.txt");
const MANIFEST: &str = include_str!("templates/Cargo.toml.txt");
const README: &str = include_str!("templates/README.md.txt");
const GITIGNORE: &str = include_str!("templates/gitignore.txt");
const REPOSITORY: &str = "https://github.com/mcaveniathor/stucco";

pub(crate) fn command() -> Command {
    let presets: Vec<&'static str> = Preset::ALL.iter().map(|p| p.name()).collect();
    Command::new("new")
        .about("Create a new stucco app: axum, a theme, a layout, a page and a table")
        .after_help(
            "Examples:\n  \
             stucco new my-app                    Slate, with stucco from crates.io\n  \
             stucco new my-app --name my-app      a theme derived from the app's name\n  \
             stucco new my-app --git              stucco from its GitHub repository",
        )
        .arg(
            Arg::new("path")
                .value_name("PATH")
                .required(true)
                .value_parser(value_parser!(PathBuf))
                .help("The directory to create; its last part names the crate"),
        )
        .next_help_heading("Theme (default: the Slate preset)")
        .arg(
            Arg::new("preset")
                .long("preset")
                .short('p')
                .value_name("NAME")
                .value_parser(PossibleValuesParser::new(presets))
                .help("A built-in preset"),
        )
        .arg(
            Arg::new("seed")
                .long("seed")
                .short('s')
                .value_name("N")
                .value_parser(value_parser!(u64))
                .help("The theme seeded by N"),
        )
        .arg(
            Arg::new("name")
                .long("name")
                .short('n')
                .value_name("TEXT")
                .help("The theme seeded by a name, such as the app's"),
        )
        .arg(
            Arg::new("random")
                .long("random")
                .short('r')
                .action(ArgAction::SetTrue)
                .help("A random seed, written into the app so it stays the same"),
        )
        .group(ArgGroup::new("base").args(["preset", "seed", "name", "random"]))
        .next_help_heading("Where stucco comes from (default: crates.io)")
        .arg(
            Arg::new("git")
                .long("git")
                .action(ArgAction::SetTrue)
                .help("Stucco's GitHub repository, for features not yet released"),
        )
        .arg(
            Arg::new("stucco-path")
                .long("stucco-path")
                .value_name("DIR")
                .value_parser(value_parser!(PathBuf))
                .help("A local stucco checkout"),
        )
        .group(ArgGroup::new("source").args(["git", "stucco-path"]))
}

pub(crate) fn run(m: &ArgMatches) -> io::Result<()> {
    let path = m.get_one::<PathBuf>("path").expect("clap requires a path");
    let name = crate_name(path)?;
    if fs::read_dir(path).is_ok_and(|mut entries| entries.next().is_some()) {
        return Err(invalid(format!(
            "{} already exists and isn't empty",
            path.display()
        )));
    }
    let theme = if let Some(preset) = m.get_one::<String>("preset") {
        format!("Preset::{}", title(preset))
    } else if let Some(seed) = m.get_one::<u64>("seed") {
        format!("Theme::seeded({seed})")
    } else if let Some(text) = m.get_one::<String>("name") {
        format!("Theme::seeded_str({text:?})")
    } else if m.get_flag("random") {
        format!("Theme::seeded({})", random_seed())
    } else {
        "Preset::Slate".to_owned()
    };
    let stucco = if m.get_flag("git") {
        format!("git = \"{REPOSITORY}\"")
    } else if let Some(dir) = m.get_one::<PathBuf>("stucco-path") {
        format!("path = {:?}", facade_dir(dir)?.display().to_string())
    } else {
        let version = env!("CARGO_PKG_VERSION");
        let minor = version.rsplit_once('.').map_or(version, |(head, _)| head);
        format!("version = \"{minor}\"")
    };
    let fill = |template: &str| {
        template
            .replace("{{name}}", &name)
            .replace("{{title}}", &title(&name))
            .replace("{{theme}}", &theme)
            .replace("{{stucco}}", &stucco)
    };
    fs::create_dir_all(path.join("src"))?;
    for (file, template) in [
        ("Cargo.toml", MANIFEST),
        ("src/main.rs", MAIN),
        ("README.md", README),
        (".gitignore", GITIGNORE),
    ] {
        fs::write(path.join(file), fill(template))?;
    }
    println!(
        "Created {name} in {}\n\n  cd {}\n  cargo run\n\nthen open http://127.0.0.1:3000",
        path.display(),
        path.display()
    );
    Ok(())
}

/// The crate name: the path's last part, which Cargo accepts as a package
/// name.
fn crate_name(path: &Path) -> io::Result<String> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| invalid(format!("{} has no name to give the crate", path.display())))?;
    let valid = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if valid {
        Ok(name.to_owned())
    } else {
        Err(invalid(format!(
            "{name:?} can't name a crate: start with a letter, then use letters, digits, - and _"
        )))
    }
}

/// The facade crate in a stucco checkout: `dir` itself, or its
/// `crates/stucco`.
fn facade_dir(dir: &Path) -> io::Result<PathBuf> {
    let dir = fs::canonicalize(dir)
        .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", dir.display())))?;
    for candidate in [dir.join("crates/stucco"), dir.clone()] {
        let manifest = fs::read_to_string(candidate.join("Cargo.toml")).unwrap_or_default();
        if manifest.contains("name = \"stucco\"") {
            return Ok(candidate);
        }
    }
    Err(invalid(format!(
        "{} isn't a stucco checkout (no crates/stucco)",
        dir.display()
    )))
}

/// `my-app` → `My app`.
fn title(name: &str) -> String {
    let words = name.replace(['-', '_'], " ");
    let mut chars = words.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
