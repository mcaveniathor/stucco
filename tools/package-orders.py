#!/usr/bin/env python3
"""Copy one cargo-leptos build into a new, self-contained runtime directory."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def bundle(binary, site, output, profile):
    binary, site, output = Path(binary), Path(site), Path(output)
    if binary.is_symlink() or not binary.is_file() or not os.access(binary, os.X_OK):
        raise ValueError("The server must be a regular executable built by cargo-leptos")
    package = site / "pkg"
    if site.is_symlink() or package.is_symlink() or not package.is_dir():
        raise ValueError("The site must contain a regular pkg directory")
    if output.resolve().is_relative_to(package.resolve()):
        raise ValueError("The bundle output must be outside the generated pkg tree")
    # This reference deliberately supports the default, un-hashed cargo-leptos names.
    names = ("orders-fullstack.js", "orders-fullstack.wasm", "orders-fullstack.css")
    assets = [package / name for name in names]
    for asset in assets:
        if asset.is_symlink() or not asset.is_file():
            raise ValueError(f"Missing regular cargo-leptos output: {asset.name}")
    with assets[1].open("rb") as stream:
        if stream.read(8) != b"\0asm\x01\0\0\0":
            raise ValueError("The WASM output has an invalid header")
    for asset in package.rglob("*"):
        if asset.is_symlink() or not (asset.is_file() or asset.is_dir()):
            raise ValueError("The generated pkg tree must contain only regular files/directories")
        if any(part.startswith(".") for part in asset.relative_to(package).parts):
            raise ValueError("The generated pkg tree must not contain hidden files/directories")
        if asset.is_file() and not asset.name.endswith((".js", ".wasm", ".css", ".d.ts", ".map")):
            raise ValueError(f"Unexpected file in generated pkg tree: {asset.name}")
    output.parent.mkdir(parents=True, exist_ok=True)
    # Never replace a release, database, or directory supplied by the caller.
    output.mkdir()
    try:
        (output / "site").mkdir()
        shutil.copy2(binary, output / "orders-fullstack")
        shutil.copytree(package, output / "site/pkg")
        shutil.copy2(ROOT / "examples/orders-fullstack/DEPLOYING.md", output / "DEPLOYING.md")
        for name in ("LICENSE-MIT", "LICENSE-APACHE"):
            shutil.copy2(ROOT / name, output / name)
        revision = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
        ).strip()
        dirty = bool(subprocess.check_output(
            ["git", "status", "--porcelain"], cwd=ROOT, text=True
        ).strip())
        metadata = {
            "format": 1,
            "profile": profile,
            "source_revision": revision,
            "source_dirty": dirty,
            "cargo_lock_sha256": digest(ROOT / "Cargo.lock"),
            "required_cargo_leptos": "0.3.11",
            "layout": "root routes; site/pkg; un-hashed assets",
        }
        (output / "build.json").write_text(json.dumps(metadata, indent=2) + "\n")
        files = sorted(path for path in output.rglob("*") if path.is_file())
        (output / "SHA256SUMS").write_text("".join(
            f"{digest(path)}  {path.relative_to(output).as_posix()}\n" for path in files
        ))
    except BaseException:
        shutil.rmtree(output)
        raise
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=("release", "debug"), default="release")
    parser.add_argument("--output", type=Path, required=True, help="new directory; never overwritten")
    args = parser.parse_args()
    try:
        path = bundle(
            ROOT / "target" / args.profile / "orders-fullstack",
            ROOT / "target/orders-fullstack-site",
            args.output,
            args.profile,
        )
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        sys.exit(f"Bundle failed: {error}")
    print(f"Runtime bundle: {path}")
    print("Verify SHA256SUMS and run the bundle rehearsal before deployment.")


if __name__ == "__main__":
    main()
