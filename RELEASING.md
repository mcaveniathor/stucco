# Releasing stucco

The stable workspace release publishes eight crates: stucco-theme, stucco-core,
stucco-macros, stucco-ui, stucco-tower, stucco, stucco-redb and stucco-cli.
The gallery, the site and the examples have `publish = false` or are
excluded. Publishing a version is permanent: a version can be yanked, never
replaced.

The opt-in `stucco-leptos` preview has its own prerelease version and release
procedure below. It remains excluded from the stable publication workflow and
the stable published-package semver checks. The `orders-fullstack` reference
has `publish = false` and remains checkout-based: it is an application, not a
library, and is built from the workspace. Its separate SSR,
hydration, storage and browser checks run in CI.

## Once: trusted publishing

The Release workflow publishes with crates.io
[trusted publishing](https://crates.io/docs/trusted-publishing): the job
exchanges its GitHub OIDC identity for a crates.io token that expires after
30 minutes. No registry token is stored in the repository.

Each published crate, the stable eight and `stucco-leptos`, has one trusted
publisher on crates.io (the crate's Settings → Trusted Publishing):

| Field | Value |
| --- | --- |
| Repository owner | `mcaveniathor` |
| Repository name | `stucco` |
| Workflow filename | `release.yml` |
| Environment | `crates-io` |

The repository's `crates-io` environment (Settings → Environments) only
accepts deployments from `v*` and `stucco-leptos-v*` tags. A new crate's first
version must be published by hand with an API token, after which you add its
trusted publisher. Trusted publishing can't create a crate.

## Prepare the release

1. Set the version in `[workspace.package]` and in every internal
   dependency in the root `Cargo.toml`, plus the optional `stucco-tower`
   dependency in `crates/stucco-redb/Cargo.toml`. Run `cargo update -w`.
2. Turn the changelog's `## Unreleased` heading into
   `## X.Y.Z — YYYY-MM-DD`, with a short summary.
3. Update version numbers in the READMEs and guides (`stucco = "X.Y"`).
4. Merge to `main` through a pull request, so CI runs the full checks: the
   all-feature publish dry run, Rust 1.85, 1.88 and 1.90, semver, features,
   packaged Leptos consumers and browser tests.

## Publish

Tag the merged commit on `main` and push the tag:

```sh
git checkout main && git pull
git tag -a vX.Y.Z -m "Release X.Y.Z"
git push origin vX.Y.Z
```

The Release workflow checks that the tag matches the workspace version, is
on `main` and has a changelog section, runs the tests, publishes the stable crates
in dependency order with the preview and examples excluded, and creates the GitHub
release from the changelog section. Creating the release on GitHub instead
also creates the tag and starts the same workflow; it then keeps your
release as it is.

If publishing stops partway, the crates already uploaded stay published.
Fix the cause, then publish the rest by hand from the tagged commit with
`cargo publish -p <crate>` (after `cargo login` with your own token) in
dependency order (theme, core, macros, ui,
tower, stucco, redb, cli), and create the GitHub release with
`gh release create vX.Y.Z --notes-file <section>`.

Afterwards, check each stable crate's page and its all-feature documentation
on docs.rs. When adding a new stable crate, remove it from CI's semver
`exclude` list after its first publication. The independently versioned
preview follows the policy below.

## Release the opt-in Leptos preview

`stucco-leptos` is a separately versioned component integration, currently
prepared as `0.1.0-alpha.1`. Preparing and verifying its archive does not mean
that version has been published. Keep the stable eight-crate workflow's
`stucco-leptos` exclusion; a preview release does not bump the workspace
version or publish the Orders application.

1. Set the independent version in `crates/stucco-leptos/Cargo.toml`. Keep its
   exact path dependency version in `examples/orders-fullstack/Cargo.toml`
   and the component README's installation examples in sync, then refresh
   the workspace lockfile. Record API changes in the changelog. Prerelease
   API changes require an explicit new preview version.
2. Check that every registry dependency is already published. The preview's
   style dependencies currently use published `stucco-core`, `stucco-theme`
   and `stucco-ui` 0.5.1; its Leptos dependency is pinned to 0.8.22. The
   archive consumer must pass against those registry dependencies, even
   when the checkout's copies have additional changes.
3. Merge the reviewed release preparation to `main`. Require CI, including
   the Rust 1.88 SSR and WASM checks, packaged consumer check, documentation
   and three-engine browser tests. From the clean merged commit, run:

   ```sh
   rustup target add wasm32-unknown-unknown --toolchain 1.88.0
   cargo +1.88.0 test -p stucco-leptos --locked --no-default-features --features ssr
   RUSTDOCFLAGS="-D warnings" cargo +1.88.0 doc -p stucco-leptos --locked --no-default-features --features ssr --no-deps
   python3 tools/check-leptos-package.py --toolchain 1.88.0
   cargo publish -p stucco-leptos --dry-run --locked --no-default-features --features ssr
   ```

   The Python 3.11+ script packages the crate, checks its normalized manifest
   and required files, then extracts it outside the workspace. An independent
   consumer renders the public components and shared CSS with registry style
   dependencies and compiles the same view for `wasm32-unknown-unknown`.
   Resolved sources and separate SSR/hydration features are checked so a
   local path patch cannot silently substitute for a registry dependency.
   This proves archive consumption, not a registry installation or browser
   hydration; the browser suite verifies interactive behavior separately.

   For local uncommitted work only, pass `--allow-dirty`. Use `--work-dir`
   with a new or empty directory outside the repository to retain the archive,
   consumer, lockfile and build outputs. `--offline` requires an already
   populated registry cache. The default compilation limit is two jobs.
4. After the explicit preview release decision, tag the same reviewed commit
   using `stucco-leptos-v0.1.0-alpha.1`, substituting the prepared version,
   and push the tag:

   ```sh
   git tag -a stucco-leptos-v0.1.0-alpha.1 -m "Release stucco-leptos 0.1.0-alpha.1"
   git push origin stucco-leptos-v0.1.0-alpha.1
   ```

   This prefix does not match the stable `v*` tags. The Release workflow's
   preview job checks the tag against the crate version and `main`, repeats
   the Rust 1.88 tests, documentation and packaged-consumer checks, and then
   publishes only `stucco-leptos` with the `ssr` feature set. It creates no
   GitHub release; keep preview release notes distinct from stable releases.

   Trusted publishing can't create a crate. The first preview version must be
   published by hand from the tagged commit, with your own token. Add the
   crate's trusted publisher once that version exists:

   ```sh
   cargo publish -p stucco-leptos --locked --no-default-features --features ssr
   ```

   Publish versions are permanent.
5. Once crates.io has indexed the upload, prove an exact registry installation:

   ```sh
   python3 tools/check-leptos-package.py --toolchain 1.88.0 --registry-version 0.1.0-alpha.1
   ```

   This mode uses no path dependency for `stucco-leptos`. Check the published
   README, licenses, example and SSR documentation on docs.rs, then update
   installation wording that still marks the version as pending publication.
   Establish a separate preview semver baseline when its release policy is
   stable; do not add it to the eight-crate stable release merely to enable
   semver checking.

GitHub Pages builds the preview's SSR API reference alongside the stable
crates. That reference reflects merged source and can precede publication;
use docs.rs for a published version's API.

## Verify the Orders deployment bundle

The fullstack CI job builds the release server and WASM assets with
`cargo-leptos`, tests the packaging safeguards, and copies them into
`$RUNNER_TEMP/orders-release`, outside the cached Cargo target directory. It
rehearses that copied bundle with a disposable
redb database, including native login, order creation/reads, uploads, exports, graceful stop,
backup/restore and session invalidation. The three-engine Playwright suite
then runs against the same bundle through `STUCCO_E2E_BUNDLE`, including the
hydrated and no-JavaScript application paths.

After those checks pass, CI uploads `orders-fullstack-release-bundle` with
seven-day retention. Its `orders-release.tar.gz` preserves the server's
executable permissions and includes the assets, build metadata, checksums
and deployment guide. This is a runtime artifact built from the checkout;
it does not publish the Orders example or deploy a service. Follow
[the Orders deployment guide](examples/orders-fullstack/DEPLOYING.md) to
verify the bundle and configure the production origin, secret, persistent
storage and reverse proxy. Build for the intended host platform before
deploying; CI's Linux binary is not a cross-platform distribution.
