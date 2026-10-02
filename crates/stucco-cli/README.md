# stucco-cli

The `stucco` command: create [stucco](https://github.com/mcaveniathor/stucco)
apps, and generate, inspect and export themes.

```sh
cargo install stucco-cli

stucco new my-app --name my-app             # an axum app with a derived theme

stucco theme --preset pine                  # Pine's tokens as CSS
stucco theme --seed 42 --radius round -f rust
stucco theme 'seed=42&radius=round' -o theme.css
stucco theme --random -f summary
stucco options                              # every option and its choices
```

Formats: `css` (default, `--scope NAME` for a named theme), `rust`, `json`,
`query` and `summary`. Queries are the same ones the
[theme playground](https://mcaveniathor.github.io/stucco/playground.html)
puts in its address. See the
[guide](https://mcaveniathor.github.io/stucco/guide/cli.html) for more.

Minimum Rust: **1.85**. Licensed under MIT or Apache-2.0.
