"""Runtime packaging and checksum boundaries, without executing the fixture binary."""

import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("package_orders", Path(__file__).with_name("package-orders.py"))
packager = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packager)
smoke_spec = importlib.util.spec_from_file_location("smoke_orders", Path(__file__).with_name("smoke-orders-bundle.py"))
smoke = importlib.util.module_from_spec(smoke_spec)
smoke_spec.loader.exec_module(smoke)


class BundleTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.binary = self.root / "server"
        self.binary.write_bytes(b"runtime fixture")
        self.binary.chmod(0o700)
        self.site = self.root / "site"
        self.pkg = self.site / "pkg"
        self.pkg.mkdir(parents=True)
        (self.pkg / "orders-fullstack.js").write_text("export {};")
        (self.pkg / "orders-fullstack.css").write_text("\n")
        (self.pkg / "orders-fullstack.wasm").write_bytes(b"\0asm\x01\0\0\0")
        self.output = self.root / "bundle"

    def create(self):
        return packager.bundle(self.binary, self.site, self.output, "debug")

    def test_copies_complete_runtime_tree_without_adjacent_secrets(self):
        (self.pkg / "snippets").mkdir()
        (self.pkg / "snippets/extra.js").write_text("export const extra = true;")
        (self.site / ".env").write_text("fixture-secret")
        (self.root / "orders.redb").write_bytes(b"fixture-database")
        self.create()
        self.assertTrue((self.output / "orders-fullstack").is_file())
        self.assertTrue((self.output / "site/pkg/snippets/extra.js").is_file())
        self.assertFalse((self.output / "site/.env").exists())
        self.assertFalse((self.output / "orders.redb").exists())
        manifest = (self.output / "SHA256SUMS").read_text()
        self.assertIn("site/pkg/snippets/extra.js", manifest)
        self.assertIn("orders-fullstack", manifest)
        smoke.verify(self.output)

    def test_existing_output_is_never_modified(self):
        self.output.mkdir()
        marker = self.output / "keep"
        marker.write_text("original")
        with self.assertRaises(FileExistsError):
            self.create()
        self.assertEqual(list(self.output.iterdir()), [marker])
        self.assertEqual(marker.read_text(), "original")

    def test_symlink_asset_is_rejected_before_output_creation(self):
        secret = self.root / "private"
        secret.write_text("not-an-asset")
        (self.pkg / "linked.js").symlink_to(secret)
        with self.assertRaises(ValueError):
            self.create()
        self.assertFalse(self.output.exists())

    def test_recursive_destination_is_rejected(self):
        self.output = self.pkg / "nested-bundle"
        with self.assertRaises(ValueError):
            self.create()
        self.assertFalse(self.output.exists())

    def test_unexpected_or_hidden_pkg_files_are_rejected(self):
        for name in (".env", "orders.redb", "credentials.txt"):
            extra = self.pkg / name
            extra.write_text("must not be packaged")
            with self.assertRaises(ValueError):
                self.create()
            self.assertFalse(self.output.exists())
            extra.unlink()

    def test_missing_or_invalid_wasm_never_produces_a_bundle(self):
        wasm = self.pkg / "orders-fullstack.wasm"
        for missing in (False, True):
            if missing:
                wasm.unlink()
            else:
                wasm.write_bytes(b"not a WASM module")
            with self.assertRaises(ValueError):
                self.create()
            self.assertFalse(self.output.exists())

    def test_checksum_manifest_must_cover_the_server_and_assets(self):
        self.create()
        manifest = self.output / "SHA256SUMS"
        lines = manifest.read_text().splitlines(keepends=True)
        for omitted in (None, "orders-fullstack", "site/pkg/orders-fullstack.wasm"):
            with self.subTest(omitted=omitted):
                manifest.write_text("" if omitted is None else "".join(
                    line for line in lines if line.split("  ", 1)[1].strip() != omitted
                ))
                with self.assertRaisesRegex(ValueError, "cover every runtime file"):
                    smoke.verify(self.output)

    def test_changed_runtime_bytes_fail_checksum_verification(self):
        self.create()
        (self.output / "orders-fullstack").write_bytes(b"changed server fixture")
        with self.assertRaisesRegex(ValueError, "Checksum mismatch: orders-fullstack"):
            smoke.verify(self.output)

    def test_unlisted_runtime_files_fail_checksum_verification(self):
        self.create()
        (self.output / "site/pkg/extra.js").write_text("export const unexpected = true;")
        with self.assertRaisesRegex(ValueError, "cover every runtime file"):
            smoke.verify(self.output)

    def test_duplicate_and_escaping_checksum_entries_are_rejected(self):
        self.create()
        manifest = self.output / "SHA256SUMS"
        original = manifest.read_text()
        manifest.write_text(original + original.splitlines(keepends=True)[0])
        with self.assertRaisesRegex(ValueError, "Duplicate checksum entry"):
            smoke.verify(self.output)
        outside = self.root / "outside"
        outside.write_text("outside the bundle")
        manifest.write_text(original + f"{packager.digest(outside)}  ../outside\n")
        with self.assertRaisesRegex(ValueError, "leaves the runtime bundle"):
            smoke.verify(self.output)

    def test_a_checksummed_server_cannot_be_replaced_with_a_symlink(self):
        self.create()
        server = self.output / "orders-fullstack"
        server.unlink()
        server.symlink_to(self.binary)
        with self.assertRaisesRegex(ValueError, "leaves the runtime bundle"):
            smoke.verify(self.output)


if __name__ == "__main__":
    unittest.main()
