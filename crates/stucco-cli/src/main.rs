//! `stucco`: generate, inspect and export stucco themes from the command line.

#![forbid(unsafe_code)]

mod new;

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::builder::PossibleValuesParser;
use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command, value_parser};
use stucco_theme::spec::{Base, OPTIONS, OptionInfo, ThemeSpec, tokens_json};
use stucco_theme::{Preset, Scope, random_seed};

const AFTER_THEME_HELP: &str = "\
Examples:
  stucco theme --preset pine                 Pine's tokens as CSS
  stucco theme --seed 42 --radius round      a seeded theme with round corners
  stucco theme 'seed=42&radius=round'        the same, as a playground query
  stucco theme --random --format rust        Rust code for a fresh random theme
  stucco theme --name acme -o theme.css      a theme derived from a name, saved

Flags override the query, and option flags override the base's choices.";

fn cli() -> Command {
    let presets: Vec<&'static str> = Preset::ALL.iter().map(|p| p.name()).collect();
    let mut theme = Command::new("theme")
        .about("Print a theme as CSS, Rust, JSON or a query string")
        .after_help(AFTER_THEME_HELP)
        .next_help_heading("Start from")
        .arg(
            Arg::new("spec")
                .value_name("QUERY")
                .help("A theme query, as the playground's links use: 'seed=42&radius=round'"),
        )
        .arg(
            Arg::new("preset")
                .long("preset")
                .short('p')
                .value_name("NAME")
                .value_parser(PossibleValuesParser::new(presets))
                .help("Start from a built-in preset"),
        )
        .arg(
            Arg::new("seed")
                .long("seed")
                .short('s')
                .value_name("N")
                .value_parser(value_parser!(u64))
                .help("Start from the theme seeded by N"),
        )
        .arg(
            Arg::new("name")
                .long("name")
                .short('n')
                .value_name("TEXT")
                .help("Start from the theme seeded by a name, such as your product's"),
        )
        .arg(
            Arg::new("random")
                .long("random")
                .short('r')
                .action(ArgAction::SetTrue)
                .help("Start from a random seed (printed to stderr, so you can keep it)"),
        )
        .group(ArgGroup::new("base").args(["preset", "seed", "name", "random"]))
        .next_help_heading("Theme options");
    for option in OPTIONS {
        let values: Vec<&'static str> = option.choices.iter().map(|c| c.value).collect();
        theme = theme.arg(
            Arg::new(option.key)
                .long(option.key)
                .value_name("VALUE")
                .value_parser(PossibleValuesParser::new(values))
                .help(format!("{} ({})", option.label, option.ty)),
        );
    }
    let theme = theme
        .next_help_heading("Output")
        .arg(
            Arg::new("format")
                .long("format")
                .short('f')
                .value_name("FORMAT")
                .value_parser(["css", "rust", "json", "query", "summary"])
                .default_value("css")
                .help("css: token stylesheet; rust: builder code; json: colour roles; query: canonical query; summary: every option's choice"),
        )
        .arg(
            Arg::new("scope")
                .long("scope")
                .value_name("NAME")
                .value_parser(scope_name)
                .help("Scope the CSS to [data-st-theme=\"NAME\"] instead of the whole page"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .value_name("FILE")
                .value_parser(value_parser!(PathBuf))
                .help("Write to FILE instead of standard output"),
        );
    Command::new("stucco")
        .about("Create stucco apps, and generate, inspect and export themes")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(new::command())
        .subcommand(theme)
        .subcommand(Command::new("presets").about("List the built-in presets"))
        .subcommand(
            Command::new("options")
                .about("List the options a theme can override, and their choices"),
        )
}

fn main() -> ExitCode {
    let matches = cli().get_matches();
    let result = match matches.subcommand() {
        Some(("new", m)) => new::run(m),
        Some(("theme", m)) => theme(m),
        Some(("presets", _)) => presets(),
        Some(("options", _)) => options(),
        _ => unreachable!("clap requires a known subcommand"),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        // A closed pipe (`stucco theme | head`) isn't worth reporting.
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn theme(m: &ArgMatches) -> io::Result<()> {
    let mut spec = match m.get_one::<String>("spec") {
        Some(query) => query
            .parse::<ThemeSpec>()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?,
        None => ThemeSpec::default(),
    };
    if let Some(name) = m.get_one::<String>("preset") {
        let preset = Preset::ALL.into_iter().find(|p| p.name() == name);
        spec.set_base(Base::Preset(preset.expect("clap checks preset names")));
    } else if let Some(&seed) = m.get_one::<u64>("seed") {
        spec.set_base(Base::Seed(seed));
    } else if let Some(name) = m.get_one::<String>("name") {
        spec.set_base(Base::Name(name.clone()));
    } else if m.get_flag("random") {
        let seed = random_seed();
        eprintln!("seed: {seed}");
        spec.set_base(Base::Seed(seed));
    }
    for option in OPTIONS {
        if let Some(value) = m.get_one::<String>(option.key) {
            spec.set(option.key, value)
                .expect("clap checks option values");
        }
    }

    let built = spec.theme().build().map_err(|report| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("the theme fails its contrast checks:\n{report}"),
        )
    })?;
    let mut text = match m.get_one::<String>("format").map(String::as_str) {
        Some("rust") => spec.rust(),
        Some("json") => tokens_json(&built),
        Some("query") => spec.to_query(),
        Some("summary") => summary(&spec),
        _ => match m.get_one::<String>("scope") {
            Some(scope) => built.css(Scope::Named(scope)),
            None => built.css(Scope::Root),
        },
    };
    if !text.ends_with('\n') {
        text.push('\n');
    }
    match m.get_one::<PathBuf>("output") {
        Some(path) => fs::write(path, text).map_err(|e| {
            io::Error::new(e.kind(), format!("couldn't write {}: {e}", path.display()))
        }),
        None => io::stdout().lock().write_all(text.as_bytes()),
    }
}

/// A `data-st-theme` value: lowercase letters, digits and `-`.
fn scope_name(name: &str) -> Result<String, String> {
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if valid {
        Ok(name.to_owned())
    } else {
        Err("use lowercase letters, digits and -".to_owned())
    }
}

/// The base, the query, and each option's effective choice, aligned.
fn summary(spec: &ThemeSpec) -> String {
    let mut out = format!("{}\n{}\n\n", spec.base(), spec.to_query());
    let width = OPTIONS.iter().map(|o| o.label.len()).max().unwrap_or(0);
    for (key, value) in spec.summary() {
        let option = OptionInfo::find(key).expect("summaries use known keys");
        let choice = option.choice(value).expect("summaries use known values");
        let set = if spec.overrides().iter().any(|(k, _)| *k == key) {
            "  (set)"
        } else {
            ""
        };
        out.push_str(&format!("{:width$}  {}{set}\n", option.label, choice.label));
    }
    out
}

fn presets() -> io::Result<()> {
    let mut out = io::stdout().lock();
    for preset in Preset::ALL {
        writeln!(out, "{}", preset.name())?;
    }
    Ok(())
}

fn options() -> io::Result<()> {
    let mut out = io::stdout().lock();
    let width = OPTIONS.iter().map(|o| o.key.len()).max().unwrap_or(0);
    for option in OPTIONS {
        let values: Vec<&str> = option.choices.iter().map(|c| c.value).collect();
        writeln!(out, "{:width$}  {}", option.key, values.join(", "))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_command_is_well_formed() {
        super::cli().debug_assert();
    }
}
