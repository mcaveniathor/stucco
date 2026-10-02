# Releasing stucco

The workspace contains six publishable libraries at version 0.1.0.
The gallery and application examples have `publish = false`.
This guide prepares and verifies a release; the commands in the publishing
section perform an actual registry upload.

## Verify the release candidate

Use a current stable Cargo that supports multi-package packaging and publication.
Compiler support for consumers remains Rust 1.85 for core/components/Tower and
Rust 1.90 for the redb adapter.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --workspace --release
cargo +1.85 check --workspace --exclude stucco-redb --exclude orders --locked
cargo +1.90 check -p stucco-redb -p orders --locked
cargo publish --dry-run --workspace --exclude gallery --exclude hello --exclude orders --all-features
```

Run the compiler-version checks specifically with Rust 1.85 and Rust 1.90 as
configured in CI. Run browser checks from `browser` before releasing component,
theme, or runtime changes. The package CI job performs an all-feature dry run
without uploading anything.

The multi-package dry run stages unpublished workspace dependencies locally and
verifies the extracted packages. Running a dependent crate's dry run alone before
its dependencies are on crates.io can fail even if its sources build locally.

Inspect `cargo package --list -p <crate>` and the archives under `target/package`
(multi-package dry runs may stage them in a subdirectory).
Each archive must contain its README, MIT/Apache license texts, required Rust
sources, and any embedded CSS/JavaScript. The UI archive must also include
LICENSE-LUCIDE. Generated icons are checked in; publishing does not run Node.

Update the root changelog and version, check dependency versions (including
the optional direct stucco-tower dependency in stucco-redb), and make sure
the README describes the actual implementation. Replace the unreleased notice
only when the packages really are published.

## First publication

Check name availability and ownership again immediately before publishing.
Availability during preparation does not reserve a crates.io name.
Use `cargo login` locally or an appropriately scoped registry token in your
release environment. Never commit registry credentials.

Publish from the verified commit with a clean worktree, in dependency order:

```sh
cargo publish -p stucco-theme
cargo publish -p stucco-core
cargo publish -p stucco-ui
cargo publish -p stucco-tower
cargo publish -p stucco-redb
cargo publish -p stucco
```

Wait for each package version to become available in the registry before the
next command. stucco-ui and stucco-tower both depend on core; stucco-redb also
depends on Tower through its default feature. The facade depends on theme,
core, and UI.

Confirm all six versions and their all-feature documentation on docs.rs,
then tag the published commit `v0.1.0` and create a release using the changelog.
Publishing a version is permanent; do not upload until the verified candidate
is the one you intend to release.

References: [Cargo publishing guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
and [Cargo manifest metadata](https://doc.rust-lang.org/cargo/reference/manifest.html).
