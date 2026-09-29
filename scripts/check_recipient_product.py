#!/usr/bin/env python3
"""Exercise recipient encryption from an external pure-Rust consumer."""
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
    parser.add_argument('--target', required=True)
    parser.add_argument('--output', help='Write the consumer PDF for independent verification')
    args = parser.parse_args()
    repository = Path(__file__).resolve().parents[1]
    env = dict(os.environ, CC='false', CXX='false')
    env.setdefault('CARGO_TARGET_DIR', str(repository / 'target'))
    if args.output:
        env['RECIPIENT_INTEROP_OUTPUT'] = str(Path(args.output).resolve())
    with tempfile.TemporaryDirectory(prefix='oxidize-recipient-consumer-') as directory:
        root = Path(directory)
        (root / 'src').mkdir()
        (root / 'src/lib.rs').write_text('')
        (root / 'tests').mkdir()
        shutil.copyfile(repository / 'oxidize-pdf-core/tests/recipient_encryption_test.rs', root / 'tests/recipient_encryption_test.rs')
        shutil.copytree(repository / 'oxidize-pdf-core/tests/fixtures/recipient_encryption', root / 'tests/fixtures/recipient_encryption')
        shutil.copyfile(repository / 'Cargo.lock', root / 'Cargo.lock')
        core = json.dumps((repository / 'oxidize-pdf-core').as_posix())
        (root / 'Cargo.toml').write_text(
            '[package]\nname = "recipient-product-probe"\nversion = "0.0.0"\nedition = "2021"\n'
            '[features]\ndefault = ["recipient-encryption"]\nrecipient-encryption = []\n'
            '[dependencies]\n'
            f'oxidize-pdf = {{ path = {core}, default-features = false, '
            'features = ["compression", "recipient-encryption"] }\n', encoding='utf-8')
        manifest = str(root / 'Cargo.toml')
        subprocess.run(['cargo', 'metadata', '--offline', '--format-version', '1', '--manifest-path', manifest], env=env, stdout=subprocess.DEVNULL, check=True)
        subprocess.run([sys.executable, str(repository / 'scripts/check_no_native_dependencies.py'), '--offline', '--package', 'recipient-product-probe', '--features', '', '--target', args.target, '--manifest-path', manifest], env=env, check=True)
        subprocess.run(['cargo', 'test', '--locked', '--offline', '--manifest-path', manifest, '--target', args.target], env=env, check=True)


if __name__ == '__main__':
    main()
