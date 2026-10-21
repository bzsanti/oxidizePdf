#!/usr/bin/env python3
"""Reuse only successful jobs from the same tree and resolved hosted environment.

The artifact name is a content-addressed receipt, not a cargo build cache. The
GitHub API remains the authority for repository, workflow, job and run success.
Receipts are never renewed on a hit. API failures result in running validation.
"""
import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request

MAX_AGE = dt.timedelta(hours=24)
RECORD_STEP = 'Record validation evidence'


def command(*args):
    return subprocess.check_output(args, text=True, stderr=subprocess.STDOUT).strip()


def fingerprint(role, job, extra):
    image = {key: os.environ.get(key, '') for key in ('ImageOS', 'ImageVersion', 'RUNNER_ARCH')}
    if not all(image.values()):
        raise ValueError('Hosted runner image identity unavailable')
    inputs = {
        'schema': 1,
        'tree': command('git', 'rev-parse', 'HEAD^{tree}'),
        'role': role,
        'job': job,
        'image': image,
        'rust': command('rustc', '-Vv'),
        'cargo': command('cargo', '-V'),
        'python': sys.version,
        'extra': extra,
        'environment': {key: os.environ.get(key, '') for key in (
            'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTDOCFLAGS',
            'CARGO_BUILD_TARGET', 'CARGO_PROFILE_RELEASE_LTO',
        )},
    }
    digest = hashlib.sha256(json.dumps(inputs, sort_keys=True).encode()).hexdigest()
    return inputs, f'ci-evidence-{digest}'


class GitHub:
    def __init__(self, repository, token):
        if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', repository):
            raise ValueError('Invalid repository')
        self.repository = repository
        self.token = token

    def get(self, path):
        request = urllib.request.Request(
            f'https://api.github.com/repos/{self.repository}/{path}',
            headers={'Authorization': f'Bearer {self.token}',
                     'Accept': 'application/vnd.github+json',
                     'X-GitHub-Api-Version': '2022-11-28'},
        )
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.load(response)

    def pages(self, path, key):
        separator = '&' if '?' in path else '?'
        # Bound API work; failure to enumerate fully must never grant reuse.
        for page in range(1, 11):
            items = self.get(f'{path}{separator}per_page=100&page={page}')[key]
            yield from items
            if len(items) < 100:
                return
        raise ValueError('API pagination limit exceeded')


def fresh(timestamp, now):
    age = now - dt.datetime.fromisoformat(timestamp.replace('Z', '+00:00'))
    return dt.timedelta(0) <= age <= MAX_AGE


def find_evidence(api, name, inputs, workflow, now):
    path = f'actions/artifacts?name={urllib.parse.quote(name)}'
    for artifact in api.pages(path, 'artifacts'):
        if (artifact['name'] != name or artifact['expired']
                or not fresh(artifact['created_at'], now)):
            continue
        run_id = artifact['workflow_run']['id']
        if str(run_id) == os.environ.get('GITHUB_RUN_ID'):
            continue
        run = api.get(f'actions/runs/{run_id}')
        if (run['status'] != 'completed' or run['conclusion'] != 'success'
                or run['path'] != f'.github/workflows/{workflow}'
                or run['event'] not in ('push', 'pull_request', 'workflow_dispatch')
                or run['head_repository']['full_name'] != api.repository
                or not fresh(run['run_started_at'], now)):
            continue
        # Do not trust a receipt from a workflow edited on a different source
        # tree. In particular, fork workflows cannot mint trusted receipts.
        commit = api.get(f"git/commits/{run['head_sha']}")
        if commit['tree']['sha'] != inputs['tree']:
            continue
        jobs = list(api.pages(f'actions/runs/{run_id}/jobs?filter=latest', 'jobs'))
        matching = [job for job in jobs if job['name'] == inputs['job']]
        if len(matching) != 1:
            continue
        job = matching[0]
        records = [s for s in job['steps'] if s['name'] == RECORD_STEP]
        if (job['conclusion'] == 'success' and len(records) == 1
                and records[0]['conclusion'] == 'success'):
            return {'run_id': run_id, 'job_id': job['id'],
                    'artifact_id': artifact['id'], 'url': run['html_url']}
    return None


def probe(args):
    hit = None
    name = ''
    try:
        inputs, name = fingerprint(args.role, args.job, args.extra)
        # Save provenance for inspection. A hit never publishes a new receipt.
        Path('.ci-evidence').mkdir(exist_ok=True)
        Path('.ci-evidence/inputs.json').write_text(json.dumps(inputs, indent=2))
        api = GitHub(os.environ['GITHUB_REPOSITORY'], os.environ['GH_TOKEN'])
        hit = find_evidence(api, name, inputs, args.workflow, dt.datetime.now(dt.timezone.utc))
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print(f'Evidence unavailable; run validation: {type(error).__name__}: {error}')
    output = f"hit={'true' if hit else 'false'}\nartifact={name}\n"
    with open(os.environ['GITHUB_OUTPUT'], 'a') as stream:
        stream.write(output)
    summary = f'Reused validation: {hit}' if hit else 'No equivalent completed evidence: execute validation.'
    print(summary)
    with open(os.environ['GITHUB_STEP_SUMMARY'], 'a') as stream:
        stream.write(f'\n{args.job}: {summary}\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--role', required=True)
    parser.add_argument('--job', required=True)
    parser.add_argument('--workflow', required=True)
    parser.add_argument('--extra', default='')
    probe(parser.parse_args())
