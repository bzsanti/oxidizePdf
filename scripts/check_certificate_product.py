#!/usr/bin/env python3
"""Run real certificate validation from a consumer without dev-dependencies."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True)
    args = parser.parse_args()
    repository = Path(__file__).resolve().parents[1]
    env = dict(os.environ, CC="false", CXX="false")
    env["CARGO_TARGET_DIR"] = str(repository / "target")
    with tempfile.TemporaryDirectory(prefix="oxidize-certificate-consumer-") as directory:
        root = Path(directory)
        (root / "src").mkdir()
        shutil.copyfile(repository / "scripts/certificate_product_probe.rs", root / "src/main.rs")
        shutil.copyfile(repository / "Cargo.lock", root / "Cargo.lock")
        core = json.dumps((repository / "oxidize-pdf-core").as_posix())
        (root / "Cargo.toml").write_text(
            '[package]\nname = "certificate-product-probe"\nversion = "0.0.0"\n'
            'edition = "2021"\n[dependencies]\n'
            f'oxidize-pdf = {{ path = {core}, default-features = false, '
            'features = ["compression", "signatures"] }\n', encoding="utf-8")
        # Add the consumer to a copy of the pinned lock, leaving the repo lock intact.
        manifest = str(root / "Cargo.toml")
        subprocess.run(["cargo", "metadata", "--offline", "--format-version", "1",
                        "--manifest-path", manifest], env=env, stdout=subprocess.DEVNULL, check=True)
        subprocess.run([sys.executable, str(repository / "scripts/check_no_native_dependencies.py"),
                        "--offline", "--package", "certificate-product-probe", "--features", "",
                        "--target", args.target, "--manifest-path", manifest], env=env, check=True)
        subprocess.run(["cargo", "run", "--locked", "--offline", "--manifest-path", manifest,
                        "--target", args.target, "--",
                        str(repository / "oxidize-pdf-core/tests/fixtures/signatures")],
                       env=env, check=True)


if __name__ == "__main__":
    main()
