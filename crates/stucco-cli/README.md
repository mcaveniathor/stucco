# stucco-cli

The `stucco` command: generate, inspect and export
[stucco](https://github.com/mcaveniathor/stucco) themes.

```sh
cargo install stucco-cli

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
