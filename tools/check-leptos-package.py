#!/usr/bin/env python3
"""Verify the packaged Leptos preview outside this Cargo workspace.

Requires Python 3.11+, Cargo, and the wasm32-unknown-unknown target. This command
never publishes. The default mode consumes a fresh local .crate archive and
registry dependencies. --registry-version checks a version already published to
crates.io, with no path dependency on stucco-leptos.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "crates/stucco-leptos"
REGISTRY_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"
WASM_TARGET = "wasm32-unknown-unknown"
REQUIRED_FILES = (
    "Cargo.toml", "README.md", "LICENSE-MIT", "LICENSE-APACHE", "src/lib.rs",
    "src/data.rs", "src/href.rs", "src/inputs.rs", "src/navigation.rs", "src/overlay.rs", "src/table.rs", "src/vocab.rs", "examples/native_form.rs",
)

CONSUMER_SOURCE = r'''use leptos::prelude::*;
use stucco_leptos::{
    Autocomplete, AutocompleteOption, Dialog, ErrorSummary, FileInput,
    PageHeader, RepeatedField, RepeatedValue, StatusMessage, SubmitButton, TextField,
};

// This same component is compiled for native SSR and the actual WASM target.
// Browser interaction is covered by the repository's independent browser suite.
#[component]
pub fn ConsumerForm() -> impl IntoView {
    let name = RwSignal::new("Ada".to_owned());
    let city = RwSignal::new("London".to_owned());
    let values = RwSignal::new(vec![RepeatedValue { id: 1, value: "ada@example.com".into() }]);
    let next = RwSignal::new(2_u64);
    view! {
        <PageHeader heading="Package consumer"/>
        <form action="/search" method="get">
            <TextField id="name" name="name" label="Name" value=name
                on_input=Callback::new(move |value| name.set(value))/>
            <Autocomplete id="city" name="city" label="City" value=city
                on_input=Callback::new(move |value| city.set(value))
                options=Signal::derive(|| vec![AutocompleteOption { value: "London".into(), label: None }])/>
            <RepeatedField id="recipients" name="recipient" label="Recipients" values=values
                on_input=Callback::new(move |(id, value)| values.update(|rows| {
                    if let Some(row) = rows.iter_mut().find(|row| row.id == id) { row.value = value; }
                }))
                on_add=Callback::new(move |()| {
                    let id = next.get_untracked();
                    next.set(id + 1);
                    values.update(|rows| rows.push(RepeatedValue { id, value: String::new() }));
                })
                on_remove=Callback::new(move |id| values.update(|rows| rows.retain(|row| row.id != id)))/>
            <SubmitButton label="Search"/>
        </form>
        <FileInput id="file" name="file" label="File"/>
        <Dialog id="help" title="Help" trigger_label="Open help" fallback_href="/help"><p>"Help text"</p></Dialog>
        <ErrorSummary id="errors"/>
        <StatusMessage message=Signal::derive(|| "Ready".to_owned())/>
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    #[test]
    fn published_dependencies_render_the_public_components_and_shared_css() {
        let owner = Owner::new();
        let html = owner.with(|| view! { <ConsumerForm/> }.to_html());
        assert!(html.contains("value=\"Ada\""));
        assert!(html.contains("name=\"recipient\""));
        assert!(html.contains("<datalist"));
        assert!(html.contains("<dialog"));
        assert!(html.contains("type=\"file\""));
        assert!(html.contains("method=\"get\""));
        let css = stucco_leptos::stylesheet(stucco_theme::Preset::Slate);
        assert!(css.contains(".st-field"));
        assert!(css.contains(".st-dialog"));
        assert!(css.contains(".st-button"));
    }
}
'''


class VerificationError(Exception):
    pass


def command(args: list[str], *, cwd: Path, capture: bool = False) -> str:
    print("+ " + " ".join(args), flush=True)
    result = subprocess.run(args, cwd=cwd, text=True, stdout=subprocess.PIPE if capture else None)
    if result.returncode:
        raise VerificationError(f"Command failed with exit code {result.returncode}: {args[0]} {args[1]}")
    return result.stdout if capture else ""


def read_toml(path: Path) -> dict:
    with path.open("rb") as source:
        return tomllib.load(source)


def check_normalized_manifest(manifest: dict) -> None:
    if any(key in manifest for key in ("workspace", "patch", "replace")):
        raise VerificationError("Packaged manifest contains workspace, patch, or replace configuration")
    if any(isinstance(value, dict) and "workspace" in value for value in manifest["package"].values()):
        raise VerificationError("Packaged manifest still inherits workspace package metadata")
    if "workspace" in manifest["package"]:
        raise VerificationError("Packaged manifest names a workspace")
    tables = [manifest, *manifest.get("target", {}).values()]
    for table in tables:
        for kind in ("dependencies", "dev-dependencies", "build-dependencies"):
            for name, dependency in table.get(kind, {}).items():
                if isinstance(dependency, dict) and any(
                    key in dependency for key in ("workspace", "path", "git", "registry", "registry-index")
                ):
                    raise VerificationError(f"Packaged dependency {name} is not an ordinary registry dependency")


def extract_archive(archive: Path, destination: Path, version: str) -> Path:
    root_name = f"stucco-leptos-{version}"
    with tarfile.open(archive, "r:gz") as package:
        for member in package.getmembers():
            path = PurePosixPath(member.name)
            if path.is_absolute() or ".." in path.parts or not path.parts or path.parts[0] != root_name:
                raise VerificationError("Archive contains an unexpected path")
            if not (member.isfile() or member.isdir()):
                raise VerificationError("Archive contains a link or special file")
            target = destination.joinpath(*path.parts)
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
            else:
                if member.size > 16 * 1024 * 1024:
                    raise VerificationError("Archive member exceeds the package inspection bound")
                target.parent.mkdir(parents=True, exist_ok=True)
                stream = package.extractfile(member)
                if stream is None:
                    raise VerificationError("Archive file could not be read")
                with stream, target.open("xb") as output:
                    shutil.copyfileobj(stream, output)
    crate = destination / root_name
    for required in REQUIRED_FILES:
        if not (crate / required).is_file():
            raise VerificationError(f"Archive is missing {required}")
    if (crate / ".cargo").exists():
        raise VerificationError("Archive unexpectedly contains Cargo configuration")
    check_normalized_manifest(read_toml(crate / "Cargo.toml"))
    return crate


def verify_graph(metadata: dict, consumer: Path, archive_crate: Path | None, mode: str, version: str) -> None:
    packages = {package["id"]: package for package in metadata["packages"]}
    nodes = metadata["resolve"]["nodes"]
    nodes_by_id = {node["id"]: node for node in nodes}
    root = metadata["resolve"]["root"]
    reachable: set[str] = set()
    todo = [root]
    while todo:
        identity = todo.pop()
        if identity in reachable:
            continue
        reachable.add(identity)
        todo.extend(dep["pkg"] for dep in nodes_by_id[identity]["deps"])
    preview = []
    registry_stucco = set()
    for identity in reachable:
        package = packages[identity]
        name = package["name"]
        manifest_path = Path(package["manifest_path"]).resolve()
        if manifest_path == consumer / "Cargo.toml":
            continue
        if name == "stucco-leptos":
            preview.append(package)
            if package["version"] != version:
                raise VerificationError("Consumer resolved a different preview version")
            if archive_crate is not None:
                if package["source"] is not None or manifest_path != archive_crate / "Cargo.toml":
                    raise VerificationError("Consumer did not resolve the extracted preview archive")
                continue
        if package["source"] != REGISTRY_SOURCE:
            # Check actual resolved sources instead of reading user credentials
            # or rewriting Cargo home. This catches local/global path patches.
            raise VerificationError(f"Consumer dependency {name} has an unexpected source")
        if name.startswith("stucco-") and name != "stucco-leptos":
            registry_stucco.add(name)
    if len(preview) != 1:
        raise VerificationError("Consumer must resolve exactly one stucco-leptos package")
    if mode == "ssr" and registry_stucco != {"stucco-core", "stucco-theme", "stucco-ui"}:
        raise VerificationError("SSR consumer did not resolve exactly the three published style dependencies")
    if mode == "hydrate" and registry_stucco:
        raise VerificationError("Hydration consumer unexpectedly includes Stucco server dependencies")
    enabled = set(nodes_by_id[preview[0]["id"]]["features"])
    if mode not in enabled or ({"hydrate", "ssr"} - {mode}) & enabled:
        raise VerificationError("SSR and hydration features leaked into the same preview build")


def verify(args: argparse.Namespace, work: Path) -> None:
    crate_manifest = read_toml(CRATE / "Cargo.toml")
    local_version = crate_manifest["package"]["version"]
    if not isinstance(local_version, str):
        raise VerificationError("The preview requires an explicit independent version")
    version = args.registry_version or local_version
    cargo = ["cargo"]
    if args.toolchain:
        cargo.append("+" + args.toolchain)
    offline = ["--offline"] if args.offline else []
    archive_crate = None
    if args.registry_version:
        print(f"Checking published stucco-leptos {version} from crates.io; no preview path override.")
    else:
        print(f"Checking local stucco-leptos {version} archive with registry dependencies; this is not a published installation.")
        package_args = cargo + ["package", "-p", "stucco-leptos", "--locked", "--no-verify", "--no-default-features", "--features", "ssr", "--target-dir", str(work / "package-target")]
        if args.allow_dirty:
            package_args.append("--allow-dirty")
        command(package_args + offline, cwd=ROOT)
        archive = work / "package-target/package" / f"stucco-leptos-{version}.crate"
        archive_crate = extract_archive(archive, work / "archive", version)

    consumer = work / "consumer"
    (consumer / "src").mkdir(parents=True)
    preview_dependency = f'version = "={version}", default-features = false'
    if archive_crate is not None:
        preview_dependency += ", path = " + json.dumps(str(archive_crate))
    leptos_version = crate_manifest["dependencies"]["leptos"]["version"]
    sibling = read_toml(ROOT / "Cargo.toml")["workspace"]["dependencies"]
    sibling_dependencies = "\n".join(
        f'{name} = {{ version = "={sibling[name]["version"]}", optional = true }}'
        for name in ("stucco-core", "stucco-theme", "stucco-ui")
    )
    (consumer / "Cargo.toml").write_text(f'''[package]
name = "stucco-leptos-package-consumer"
version = "0.0.0"
edition = "2024"
rust-version = "1.88"
publish = false

[workspace]
resolver = "3"

[features]
default = []
ssr = ["leptos/ssr", "stucco-leptos/ssr", "dep:stucco-core", "dep:stucco-theme", "dep:stucco-ui"]
hydrate = ["leptos/hydrate", "stucco-leptos/hydrate"]

[dependencies]
leptos = {{ version = "{leptos_version}", default-features = false }}
stucco-leptos = {{ {preview_dependency} }}
{sibling_dependencies}
''', encoding="utf-8")
    (consumer / "src/lib.rs").write_text(CONSUMER_SOURCE, encoding="utf-8")
    command(cargo + ["generate-lockfile"] + offline, cwd=consumer)
    target_dir = ["--target-dir", str(work / "consumer-target")]
    for mode in ("ssr", "hydrate"):
        target = ["--target", WASM_TARGET] if mode == "hydrate" else []
        platform = ["--filter-platform", WASM_TARGET] if mode == "hydrate" else []
        metadata = json.loads(command(cargo + ["metadata", "--format-version", "1", "--locked", "--no-default-features", "--features", mode] + platform + offline, cwd=consumer, capture=True))
        verify_graph(metadata, consumer, archive_crate, mode, version)
        operation = ["test", "--lib"] if mode == "ssr" else ["check", "--lib"]
        command(cargo + operation + ["--locked", "--no-default-features", "--features", mode, "--jobs", str(args.jobs)] + target + target_dir + offline, cwd=consumer)
    print(f"Verified {'published registry package' if args.registry_version else 'local package archive'} {version}: SSR rendering/CSS and WASM hydration compilation; sources checked.")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--registry-version", help="Check an already-published exact version instead of creating a local archive")
    parser.add_argument("--allow-dirty", action="store_true", help="Permit packaging uncommitted local changes (never used in release checks)")
    parser.add_argument("--offline", action="store_true", help="Use only the local registry cache")
    parser.add_argument("--toolchain", help="Rustup toolchain, for example 1.88.0; default is the selected Cargo toolchain")
    parser.add_argument("--jobs", type=int, default=2, help="Maximum Cargo compilation jobs (default: 2)")
    parser.add_argument("--work-dir", type=Path, help="New or empty directory outside the repository; retained for inspection")
    args = parser.parse_args()
    if args.jobs < 1:
        parser.error("--jobs must be positive")
    if args.registry_version and not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?", args.registry_version):
        parser.error("--registry-version must be an exact semantic version")
    if args.registry_version and args.allow_dirty:
        parser.error("--allow-dirty only applies to local archive checks")
    temporary = None
    try:
        if args.work_dir:
            work = args.work_dir.resolve()
            if work == ROOT or ROOT in work.parents:
                raise VerificationError("--work-dir must be outside the repository")
            if work.exists() and any(work.iterdir()):
                raise VerificationError("--work-dir must be empty so stale build artifacts cannot pass verification")
            work.mkdir(parents=True, exist_ok=True)
        else:
            temporary = tempfile.TemporaryDirectory(prefix="stucco-leptos-package-")
            work = Path(temporary.name).resolve()
        if work == ROOT or ROOT in work.parents:
            raise VerificationError("Verification directory must be outside the repository; choose --work-dir or an external TMPDIR")
        print(f"Verification directory: {work}", flush=True)
        verify(args, work)
        return 0
    except (VerificationError, OSError, tarfile.TarError, tomllib.TOMLDecodeError) as error:
        print(f"Package verification failed: {error}", file=sys.stderr)
        return 1
    finally:
        if temporary is not None:
            temporary.cleanup()


if __name__ == "__main__":
    sys.exit(main())
