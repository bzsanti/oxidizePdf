#!/usr/bin/env python3
"""Inspect the product graph (normal/build edges), including transitive sources."""
import argparse
import hashlib
import json
import os
import re
from pathlib import Path
import subprocess
import sys
import tomllib

NATIVE_SUFFIXES = {".c", ".cc", ".cpp", ".cxx", ".s", ".asm", ".a", ".lib", ".so", ".dll", ".dylib", ".o", ".obj"}
COMPILERS = {"cc", "cmake", "bindgen", "autotools", "meson"}
# These C files are parsed with include_str! by Rust tests, never compiled.
DATA_FIXTURES = {
    ("fancy-regex", "0.17.0", "tests/oniguruma/test_utf8.c"): "1e4813ac64703cfdbcdca359b82856c5c6778abf977e577992f6b34ab26b92de",
    ("fancy-regex", "0.17.0", "tests/oniguruma/test_utf8_ignore.c"): "6415e8361b25c8ac701e933638d441f0eef39908cbd8edc5247fceaf89348f04",
}
RUST_LINKS = {("rayon-core", "1.13.0", "rayon-core"): "Cargo singleton marker; build.rs only emits cargo:rerun-if-changed"}
WINDOWS_IMPORTS = {"windows_i686_gnu", "windows_i686_gnullvm", "windows_i686_msvc", "windows_x86_64_gnu", "windows_x86_64_gnullvm", "windows_x86_64_msvc", "windows_aarch64_gnullvm", "windows_aarch64_msvc", "winapi-i686-pc-windows-gnu", "winapi-x86_64-pc-windows-gnu"}
FORBIDDEN = {"ring", "aws-lc-sys", "openssl-sys", "libz-sys", "zstd-sys"}


def inspect_package(package, python_binding=False):
    errors = []
    name = package["name"]
    if name in COMPILERS | FORBIDDEN:
        errors.append(f"{name}: native dependency/toolchain")
    # Windows import libraries describe the platform ABI; they contain no
    # third-party C implementation. System FFI is distinct from bundled C.
    platform_import = name in WINDOWS_IMPORTS
    interpreter_abi = python_binding and (name, package["version"], package.get("links")) == ("pyo3-ffi", "0.28.3", "python")
    if package.get("links") and not interpreter_abi and (name, package["version"], package["links"]) not in RUST_LINKS:
        errors.append(f"{name}: native links={package['links']}")
    root = Path(package["manifest_path"]).parent
    for source in root.rglob("*"):
        if source.is_file() and source.suffix.lower() in NATIVE_SUFFIXES:
            relative = source.relative_to(root)
            if any(part in {"target", ".git"} for part in relative.parts):
                continue
            if platform_import and source.suffix.lower() in {".a", ".lib"} and "lib" in relative.parts:
                continue
            fixture = DATA_FIXTURES.get((name, package["version"], relative.as_posix()))
            if fixture and hashlib.sha256(source.read_bytes()).hexdigest() == fixture:
                continue
            errors.append(f"{name}: native source {relative}")
    for target in package.get("targets", []):
        if "custom-build" in target["kind"]:
            script = Path(target["src_path"]).read_text(encoding="utf-8")
            if re.search(r'Command::new\(\s*"(?:cc|gcc|clang|g\+\+|clang\+\+|cmake|make)"', script):
                errors.append(f"{name}: native compiler invocation in build script")
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest-path", default="Cargo.toml")
    parser.add_argument("--package", default="oxidize-pdf")
    parser.add_argument("--features", default="signatures")
    parser.add_argument("--target", default="x86_64-unknown-linux-gnu")
    parser.add_argument("--no-default-features", action="store_true")
    parser.add_argument("--all-features", action="store_true")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--python-binding", action="store_true", help="recognize the reviewed PyO3 host-interpreter ABI, not a native crypto provider")
    parser.add_argument("--build", action="store_true", help="build the selected library with C/C++ compilers disabled")
    parser.add_argument("--product-features", action="store_true", help="all core features except optional external OCR integration")
    args = parser.parse_args()
    common = ["--locked", "--manifest-path", args.manifest_path]
    if args.offline:
        common.append("--offline")
    if args.product_features:
        manifest = Path(__file__).resolve().parents[1] / "oxidize-pdf-core/Cargo.toml"
        features = tomllib.loads(manifest.read_text(encoding="utf-8"))["features"]
        args.features = ",".join(sorted(set(features) - {"ocr-full", "ocr-tesseract"}))
    selection = ["--all-features"] if args.all_features else ["--features", args.features]
    if args.no_default_features:
        selection.append("--no-default-features")
    tree = subprocess.check_output(["cargo", "tree", *common, "-p", args.package,
        *selection, "--target", args.target, "--edges", "normal,build", "--prefix", "none",
        "--format", "{p}", "--no-dedupe"], text=True, encoding="utf-8")
    selected = {tuple(line.split()[:2]) for line in tree.splitlines()}
    metadata = json.loads(subprocess.check_output(["cargo", "metadata", *common,
        *selection, "--format-version", "1"], text=True, encoding="utf-8"))
    packages = [p for p in metadata["packages"] if (p["name"], "v" + p["version"]) in selected]
    if not packages:
        raise RuntimeError("empty product graph")
    errors = [error for p in packages for error in inspect_package(p, args.python_binding)]
    print(json.dumps({"target": args.target, "features": selection,
        "packages": sorted(p["name"] + "@" + p["version"] for p in packages),
        "errors": errors}, indent=2))
    if not errors and args.build:
        env = dict(os.environ, CC="false", CXX="false")
        subprocess.run(["cargo", "build", *common, "-p", args.package, *selection,
            "--target", args.target, "--lib"], env=env, check=True)
    return bool(errors)


if __name__ == "__main__":
    sys.exit(main())
