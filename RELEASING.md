# Releasing stucco

The workspace publishes eight crates: stucco-theme, stucco-core,
stucco-macros, stucco-ui, stucco-tower, stucco, stucco-redb and stucco-cli.
The gallery, the site and the examples have `publish = false` or are
excluded. Publishing a version is permanent: a version can be yanked, never
replaced.

## Once: the registry token

Create a crates.io API token with the `publish-new` and `publish-update`
scopes, limited to the `stucco*` crates if you like, and add it to the
repository as the `CARGO_REGISTRY_TOKEN` Actions secret (Settings → Secrets
and variables → Actions). Never commit a token.

## Prepare the release

1. Set the version in `[workspace.package]` and in every internal
   dependency in the root `Cargo.toml`, plus the optional `stucco-tower`
   dependency in `crates/stucco-redb/Cargo.toml`. Run `cargo update -w`.
2. Turn the changelog's `## Unreleased` heading into
   `## X.Y.Z — YYYY-MM-DD`, with a short summary.
3. Update version numbers in the READMEs and guides (`stucco = "X.Y"`).
4. Merge to `main` through a pull request, so CI runs the full checks: the
   all-feature publish dry run, Rust 1.85 and 1.90, semver, features and
   browser tests.

## Publish

Tag the merged commit on `main` and push the tag:

```sh
git checkout main && git pull
git tag -a vX.Y.Z -m "Release X.Y.Z"
git push origin vX.Y.Z
```

The Release workflow checks that the tag matches the workspace version, is
on `main` and has a changelog section, runs the tests, publishes every crate
in dependency order with `cargo publish --workspace`, and creates the GitHub
release from the changelog section.

If publishing stops partway, the crates already uploaded stay published.
Fix the cause, then publish the rest by hand from the tagged commit with
`cargo publish -p <crate>` in dependency order (theme, core, macros, ui,
tower, stucco, redb, cli), and create the GitHub release with
`gh release create vX.Y.Z --notes-file <section>`.

Afterwards, check each crate's page and its all-feature documentation on
docs.rs. CI's semver check skips crates that have no published version yet;
remove them from its `exclude` list once they are on crates.io.
