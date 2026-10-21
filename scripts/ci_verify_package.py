#!/usr/bin/env python3
"""Require the registry's exact version checksum to equal the local package."""
import hashlib
import json
import os
from pathlib import Path
import re
import time
import urllib.error
import urllib.request


def verify(data, version, checksum):
    published = data['version']
    if (published['num'] != version or published['yanked']
            or published['checksum'] != checksum):
        raise ValueError('Registry package version/checksum differs or is yanked')


def main():
    version = os.environ['PACKAGE_VERSION']
    if not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?', version):
        raise ValueError('Invalid package version')
    package = Path(f'target/package/oxidize-pdf-{version}.crate')
    checksum = hashlib.sha256(package.read_bytes()).hexdigest()
    for attempt in range(12):
        try:
            with urllib.request.urlopen(
                f'https://crates.io/api/v1/crates/oxidize-pdf/{version}', timeout=30
            ) as response:
                verify(json.load(response), version, checksum)
            print(f'Confirmed oxidize-pdf {version}: sha256:{checksum}')
            return
        except urllib.error.HTTPError as error:
            if error.code != 404 or attempt == 11:
                raise
            time.sleep(10)
    raise ValueError('Published package unavailable')


if __name__ == '__main__':
    main()
