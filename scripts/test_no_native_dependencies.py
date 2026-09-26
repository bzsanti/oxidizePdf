"""Regression tests for the dependency policy, independent of crate names."""
import tempfile
import hashlib
from unittest.mock import patch
from pathlib import Path
import unittest
from check_no_native_dependencies import DATA_FIXTURES, inspect_package


class NativeDependencyPolicyTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.package = {"name": "innocent-name", "version": "1.0.0",
                        "manifest_path": str(self.root / "Cargo.toml")}

    def test_unknown_transitive_c_source_is_rejected(self):
        (self.root / "arithmetic.c").write_text("int add(int a, int b) { return a+b; }")
        self.assertIn("native source arithmetic.c", inspect_package(self.package)[0])

    def test_precompiled_native_archive_is_rejected(self):
        (self.root / "crypto.a").write_bytes(b"!<arch>\n")
        self.assertIn("native source crypto.a", inspect_package(self.package)[0])

    def test_compiler_build_dependency_is_rejected(self):
        self.package["name"] = "cc"
        self.assertIn("cc: native dependency/toolchain", inspect_package(self.package))

    def test_native_links_are_rejected(self):
        self.package["links"] = "external_crypto"
        self.assertEqual(inspect_package(self.package), ["innocent-name: native links=external_crypto"])

    def test_rust_only_source_passes(self):
        (self.root / "lib.rs").write_text("pub fn add(a: u32, b: u32) -> u32 { a+b }")
        self.assertEqual(inspect_package(self.package), [])

    def test_direct_compiler_invocation_is_rejected(self):
        script = self.root / "build.rs"
        script.write_text('fn main() { Command::new("clang").arg("input"); }')
        self.package["targets"] = [{"kind": ["custom-build"], "src_path": str(script)}]
        self.assertIn("native compiler invocation", inspect_package(self.package)[0])

    def test_windows_prefix_is_not_a_blanket_exemption(self):
        self.package["name"] = "windows_custom_crypto"
        (self.root / "crypto.c").write_text("int x;")
        self.assertTrue(inspect_package(self.package))

    def test_windows_import_package_c_source_is_still_rejected(self):
        self.package["name"] = "windows_x86_64_gnu"
        (self.root / "crypto.c").write_text("int x;")
        self.assertIn("native source crypto.c", inspect_package(self.package)[0])

    def test_c_source_under_tests_is_not_a_blanket_exemption(self):
        directory = self.root / "tests"
        directory.mkdir()
        (directory / "crypto.c").write_text("int x;")
        self.assertTrue(inspect_package(self.package))

    def test_reviewed_text_fixture_requires_matching_hash(self):
        source = self.root / "input.c"
        source.write_bytes(b"test data")
        key = (self.package["name"], self.package["version"], "input.c")
        with patch.dict(DATA_FIXTURES, {key: hashlib.sha256(b"test data").hexdigest()}):
            self.assertEqual(inspect_package(self.package), [])
            source.write_bytes(b"changed input")
            self.assertIn("native source input.c", inspect_package(self.package)[0])

    def test_rayon_marker_requires_reviewed_version(self):
        self.package.update(name="rayon-core", version="1.13.0", links="rayon-core")
        self.assertEqual(inspect_package(self.package), [])
        self.package["links"] = "unexpected_native_library"
        self.assertTrue(inspect_package(self.package))
        self.package["links"] = "rayon-core"
        self.package["version"] = "1.14.0"
        self.assertTrue(inspect_package(self.package))


if __name__ == "__main__":
    unittest.main()
