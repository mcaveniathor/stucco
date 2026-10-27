#!/usr/bin/env python3
"""Rehearse a copied Orders bundle with a disposable redb database and account."""

import argparse
import hashlib
from html.parser import HTMLParser
import http.cookiejar
import os
from pathlib import Path
import re
import secrets
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request


class Forms(HTMLParser):
    def __init__(self, text):
        super().__init__()
        self.forms, self.links, self.current = {}, [], None
        self.feed(text)

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == "form":
            self.current = {}
            self.forms[attrs.get("action", "")] = self.current
        elif tag == "input" and self.current is not None and "name" in attrs:
            if attrs.get("type") not in ("file", "submit"):
                self.current[attrs["name"]] = attrs.get("value", "")
        elif tag == "a" and "href" in attrs:
            self.links.append(attrs["href"])

    def handle_endtag(self, tag):
        if tag == "form":
            self.current = None


def verify(bundle):
    verified = set()
    for line in (bundle / "SHA256SUMS").read_text().splitlines():
        expected, name = line.split("  ", 1)
        if name in verified:
            raise ValueError("Duplicate checksum entry")
        verified.add(name)
        path = bundle / name
        if path.is_symlink() or not path.resolve().is_relative_to(bundle):
            raise ValueError("Checksum entry leaves the runtime bundle")
        with path.open("rb") as stream:
            actual = hashlib.file_digest(stream, "sha256").hexdigest()
        if actual != expected:
            raise ValueError(f"Checksum mismatch: {name}")
    required = {
        "orders-fullstack", "DEPLOYING.md", "build.json",
        "LICENSE-MIT", "LICENSE-APACHE",
        "site/pkg/orders-fullstack.js", "site/pkg/orders-fullstack.wasm",
        "site/pkg/orders-fullstack.css",
    }
    actual_files = {
        path.relative_to(bundle).as_posix()
        for path in bundle.rglob("*") if path.is_file() and path.name != "SHA256SUMS"
    }
    if not required <= verified or actual_files != verified:
        raise ValueError("Checksum manifest must cover every runtime file")


def rehearse(bundle):
    verify(bundle)
    binary = bundle / "orders-fullstack"
    with tempfile.TemporaryDirectory(prefix="stucco-bundle-rehearsal-") as directory:
        directory = Path(directory)
        database, restored = directory / "orders.redb", directory / "restored.redb"
        backup = directory / "snapshot.redb"
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            port = sock.getsockname()[1]
        origin = f"http://127.0.0.1:{port}"
        env = dict(os.environ)
        # Explicit values keep the rehearsal independent of local application settings.
        for key in ("SERVER_FN_PREFIX", "DISABLE_SERVER_FN_HASH", "SERVER_FN_MOD_PATH"):
            env.pop(key, None)
        env.update(
            LEPTOS_ENV="PROD", LEPTOS_SITE_ROOT=str(bundle / "site"),
            LEPTOS_SITE_ADDR=f"127.0.0.1:{port}", LEPTOS_SITE_PKG_DIR="pkg",
            LEPTOS_OUTPUT_NAME="orders-fullstack", LEPTOS_HASH_FILES="false",
            ORDERS_ORIGIN=origin, STUCCO_SECRET_KEY=secrets.token_hex(32),
            ORDERS_SESSION_SECONDS="3600",
        )
        password = secrets.token_urlsafe(24)

        def cli(db, *args, password=None):
            subprocess.run(
                [str(binary), "--database", str(db), *args],
                input=password, text=True, env=env, cwd=bundle, check=True,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30,
            )

        cli(database, "user-add", "--tenant", "rehearsal", "--username", "operator",
            "--display-name", "Bundle operator", password=password + "\n")
        cookies = http.cookiejar.CookieJar()
        client = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(cookies))

        def request(path, form=None, data=None, content_type=None):
            headers = {}
            if form is not None:
                data = urllib.parse.urlencode(form).encode()
                content_type = "application/x-www-form-urlencoded"
            if data is not None:
                headers.update(Origin=origin, **{"Content-Type": content_type})
            with client.open(urllib.request.Request(origin + path, data=data, headers=headers), timeout=15) as reply:
                return urllib.parse.urlparse(reply.url).path, reply.headers, reply.read()

        process = None
        log = (directory / "server.log").open("wb")

        def start(db):
            nonlocal process
            process = subprocess.Popen(
                [str(binary), "--database", str(db), "serve"], cwd=bundle,
                env=env, stdout=log, stderr=log,
            )
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                if process.poll() is not None:
                    raise RuntimeError("Bundle server exited before readiness")
                try:
                    request("/ready")
                    return
                except (OSError, urllib.error.URLError):
                    time.sleep(0.1)
            raise RuntimeError("Bundle readiness timed out")

        def stop():
            nonlocal process
            if process is not None:
                process.terminate()
                try:
                    code = process.wait(timeout=15)
                    if code != 0:
                        raise RuntimeError(f"Bundle did not exit cleanly after SIGTERM (exit {code})")
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
                    raise RuntimeError("Bundle did not stop gracefully")
                finally:
                    process = None

        def login():
            _, headers, html = request("/login")
            assert headers["Cache-Control"] == "private, no-store"
            assert "'nonce-" in headers["Content-Security-Policy"]
            form = Forms(html.decode()).forms["/login"]
            form.update(username="operator", password=password, return_to="/orders")
            path, _, _ = request("/login", form=form)
            assert path == "/orders"

        try:
            start(database)
            login()
            for asset, mime in (("orders-fullstack.js", "javascript"), ("orders-fullstack.wasm", "application/wasm")):
                _, headers, body = request("/pkg/" + asset)
                assert mime in headers["Content-Type"]
                assert headers["Cache-Control"] == "no-cache"
                assert body == (bundle / "site/pkg" / asset).read_bytes()
            _, headers, css = request("/theme.css")
            assert headers["Cache-Control"] == "no-cache" and b".st-button" in css
            _, _, html = request("/orders/new")
            form = Forms(html.decode()).forms["/orders/new"]
            form.update(customer="Bundle rehearsal", quantity_one="2", quantity_two="3")
            order, _, _ = request("/orders/new", form=form)
            assert re.fullmatch(r"/orders/\d+", order)
            _, _, html = request(order)
            form = Forms(html.decode()).forms[order + "/attachments"]
            generation = form["expected_generation"]
            boundary = "rehearsal-" + secrets.token_hex(12)
            pieces = [
                f'--{boundary}\r\nContent-Disposition: form-data; name="{name}"\r\n\r\n{value}\r\n'.encode()
                for name, value in form.items()
            ]
            payload = b"Attachment survives a deployed-bundle backup and restore.\n"
            pieces.append(
                f'--{boundary}\r\nContent-Disposition: form-data; name="attachment"; filename="rehearsal.txt"\r\nContent-Type: text/plain\r\n\r\n'.encode()
                + payload + f"\r\n--{boundary}--\r\n".encode()
            )
            _, _, html = request(order + "/attachments", data=b"".join(pieces), content_type=f"multipart/form-data; boundary={boundary}")
            attachment = next(link for link in Forms(html.decode()).links if re.fullmatch(order + r"/attachments/\d+", link))
            assert request(attachment)[2] == payload
            _, _, html = request("/exports")
            export_form = Forms(html.decode()).forms["/exports"]
            export, _, _ = request("/exports", form=export_form)
            assert re.fullmatch(r"/exports/[0-9a-f]{32}", export)
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                _, _, html = request(export)
                if export + "/download" in Forms(html.decode()).links:
                    break
                time.sleep(0.1)
            else:
                raise RuntimeError("Export did not finish")
            assert b"Bundle rehearsal" in request(export + "/download")[2]
            stop()
            cli(database, "backup", "--destination", str(backup))
            cli(restored, "restore", "--source", str(backup))
            start(restored)
            # Reuse the old cookie jar: restored databases must reject its session.
            assert request(order)[0] == "/login"
            login()
            _, _, html = request(order)
            assert b"Bundle rehearsal" in html
            new_form = Forms(html.decode()).forms[order + "/attachments"]
            assert new_form["expected_generation"] != generation
            assert request(attachment)[2] == payload
            try:
                request(export + "/download")
                raise AssertionError("A derived export survived restore")
            except urllib.error.HTTPError as error:
                assert error.code == 404
            stop()
        finally:
            stop()
            log.close()
    print("PASS: copied bundle, native login/create/read/upload/export, assets, graceful stop, backup/restore, session invalidation")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("bundle", type=Path)
    rehearse(parser.parse_args().bundle.resolve(strict=True))
