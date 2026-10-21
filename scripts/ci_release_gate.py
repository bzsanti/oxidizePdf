#!/usr/bin/env python3
"""Require complete, successful current-main coverage before publishing a tag.

Job-level evidence reuse makes these push runs inexpensive. They still resolve
current toolchains/images and rerun validation on every evidence miss. Never
infer completeness from an empty list of check-runs, or from a different SHA.
"""
import datetime as dt
import json
import os
from pathlib import Path
import sys
import time
from ci_evidence import GitHub, fresh


def coverage_status(api, sha, required, now):
    if not required or any(not jobs for jobs in required.values()):
        raise ValueError('Empty release coverage policy')
    pending = []
    # Push coverage is necessary, but cannot override a failed or still running
    # scheduled/manual validation of the same release candidate. Rerun that
    # execution successfully or validate a corrected commit before publication.
    for run in api.pages(f'actions/runs?head_sha={sha}', 'workflow_runs'):
        if (run['head_sha'] != sha or run['head_branch'] != 'main'
                or run['event'] not in ('schedule', 'workflow_dispatch')
                or run['head_repository']['full_name'] != api.repository
                or run['path'] not in {f'.github/workflows/{name}' for name in required}):
            continue
        if run['status'] != 'completed':
            pending.append(f'Additional validation {run["id"]}: {run["status"]}')
        elif run['conclusion'] != 'success':
            raise ValueError(f'Additional validation {run["id"]}: {run["conclusion"]}')
    for workflow, expected in required.items():
        runs = api.get(f'actions/workflows/{workflow}/runs?event=push&head_sha={sha}&per_page=100')['workflow_runs']
        candidates = [r for r in runs if r['head_sha'] == sha
                      and r['event'] == 'push' and r['head_branch'] == 'main'
                      and r['head_repository']['full_name'] == api.repository
                      and r['path'] == f'.github/workflows/{workflow}']
        if not candidates:
            pending.append(f'{workflow}: no main validation')
            continue
        run = max(candidates, key=lambda r: r['id'])
        if not fresh(run['run_started_at'], now):
            raise ValueError(f'{workflow}: validation older than 24 hours; rerun main CI')
        if run['status'] != 'completed':
            pending.append(f'{workflow}: {run["status"]}')
            continue
        if run['conclusion'] != 'success':
            raise ValueError(f'{workflow}: {run["conclusion"]}, run {run["id"]}')
        jobs = list(api.pages(f'actions/runs/{run["id"]}/jobs?filter=latest', 'jobs'))
        for name in expected:
            matches = [job for job in jobs if job['name'] == name]
            if len(matches) != 1 or matches[0]['conclusion'] != 'success':
                raise ValueError(f'{workflow}: missing, duplicate or unsuccessful required job {name}')
        print(f'Validated {workflow}: {run["html_url"]}')
    return pending


def main():
    api = GitHub(os.environ['GITHUB_REPOSITORY'], os.environ['GH_TOKEN'])
    required = json.loads(Path('scripts/ci_required_jobs.json').read_text())
    sha = os.environ['GITHUB_SHA']
    deadline = time.monotonic() + 1800
    while True:
        pending = coverage_status(api, sha, required, dt.datetime.now(dt.timezone.utc))
        if not pending:
            return
        if time.monotonic() >= deadline:
            raise ValueError('Timed out waiting for coverage: ' + '; '.join(pending))
        print('; '.join(pending), flush=True)
        time.sleep(30)


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f'Release blocked: {error}', file=sys.stderr)
        sys.exit(1)
