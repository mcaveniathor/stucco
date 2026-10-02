# stucco-macros

Derive macros for [stucco](https://github.com/mcaveniathor/stucco). Use them
through the `stucco` crate's `derive` feature rather than directly:

```toml
stucco = { version = "0.2", features = ["derive"] }
```

`#[derive(Columns)]` turns a struct's fields into data table columns. See the
`Columns` trait in `stucco::collections` for the attributes it takes.

Minimum Rust: **1.85**. Licensed under MIT or Apache-2.0.
