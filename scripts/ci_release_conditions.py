#!/usr/bin/env python3
"""Validate the tag, manifest version and merged-main provenance before release."""
import os
import re
import subprocess
import tomllib
from ci_evidence import GitHub


def validate(tag, version, sha, pulls):
    if not re.fullmatch(r'v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?', tag):
        raise ValueError('Invalid release tag')
    if tag != 'v' + version:
        raise ValueError('Tag differs from package version')
    if not any(p['merged_at'] and p['base']['ref'] == 'main'
               and p['merge_commit_sha'] == sha for p in pulls):
        raise ValueError('Tag must identify a merged release PR targeting main')


def main():
    sha = os.environ['GITHUB_SHA']
    tag = os.environ['GITHUB_REF_NAME']
    with open('Cargo.toml', 'rb') as source:
        version = tomllib.load(source)['workspace']['package']['version']
    subprocess.run(['git', 'merge-base', '--is-ancestor', sha, 'origin/main'], check=True)
    api = GitHub(os.environ['GITHUB_REPOSITORY'], os.environ['GH_TOKEN'])
    pulls = api.get(f'commits/{sha}/pulls?per_page=100')
    validate(tag, version, sha, pulls)
    with open(os.environ['GITHUB_OUTPUT'], 'a') as output:
        output.write(f'should_release=true\nversion_tag={tag}\n')


if __name__ == '__main__':
    main()
