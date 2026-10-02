# Command line

The `stucco` command generates themes in a terminal or a build script, using the same engine as your app and the playground.

## Install

```sh
cargo install stucco-cli
```

This installs a binary called `stucco`. It needs Rust 1.85 or later.

## Generate a theme

`stucco theme` prints a theme's token stylesheet. Start from a preset, a seed, a name or a random seed, and override any option:

```sh
stucco theme --preset pine
stucco theme --seed 42 --radius round --table striped
stucco theme --name acme -o static/theme.css
stucco theme --random
```

`--random` prints the seed it picked to standard error, so you can rebuild the theme you liked with `--seed`. `--name` hashes a name into a seed, like `Theme::seeded_str`. Run `stucco options` to list every option and its choices, and `stucco presets` to list the presets.

## Choose a format

`--format` (or `-f`) picks the output:

| Format | Output |
| --- | --- |
| `css` (default) | The full token stylesheet. Add `--scope brand` to scope it to `[data-st-theme="brand"]`. |
| `rust` | The `Theme` builder calls that produce the theme. |
| `json` | Every colour role's light and dark value. |
| `query` | The theme as a query string. |
| `summary` | The base and each option's choice, marking the ones you set. |

```sh
stucco theme --seed 42 --radius round -f rust
```

```rust
use stucco::Bundle;
use stucco::theme::{Radius, Theme};

// seed=42&radius=round
let theme = Theme::seeded(42)
    .radius(Radius::Round);
let bundle = Bundle::new(theme.build().expect("the theme passes its contrast checks"));
```

## Queries

A theme can be written as a query string such as `seed=42&radius=round`. The [playground](../playground.html) puts it in its address, the CLI takes it as an argument, and `stucco::theme::spec::ThemeSpec` parses it in Rust, so you can paste a playground link's query into any of them:

```sh
stucco theme 'seed=42&radius=round&buttons=pill'
```

Flags override the query. The base is one of `preset`, `seed` or `name`, and the other keys are the option names `stucco options` lists. In the playground, type a name into the Seed field to get `name=`.

```rust
use stucco::theme::spec::ThemeSpec;

let spec: ThemeSpec = std::env::var("THEME")
    .unwrap_or_default()
    .parse()?; // unknown keys or values are errors
let bundle = stucco::Bundle::new(spec.theme().build()?);
```

`ThemeSpec::parse_lossy` skips anything it doesn't recognise instead, for input from a URL.
