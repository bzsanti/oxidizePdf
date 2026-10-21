import copy
import datetime as dt
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace

import ci_evidence as evidence
from ci_release_gate import coverage_status
from ci_verify_package import verify
from ci_release_conditions import validate

NOW = dt.datetime(2026, 10, 19, 12, tzinfo=dt.timezone.utc)
STAMP = '2026-10-19T11:00:00Z'
TREE = 'a' * 40
SHA = 'b' * 40


class FixtureAPI(evidence.GitHub):
    """GitHub transport fixture; production enumeration/decisions run unchanged."""
    def __init__(self):
        super().__init__('owner/repo', 'unused')
        self.artifact = {'name': 'receipt', 'id': 12, 'expired': False,
                         'created_at': STAMP, 'workflow_run': {'id': 42}}
        self.run = {'id': 42, 'status': 'completed', 'conclusion': 'success',
                    'path': '.github/workflows/ci.yml', 'event': 'push',
                    'head_repository': {'full_name': 'owner/repo'},
                    'head_sha': SHA, 'head_branch': 'main',
                    'run_started_at': STAMP, 'html_url': 'https://github.com/owner/repo/actions/runs/42'}
        self.job = {'id': 7, 'name': 'Test (ubuntu-latest, stable)',
                    'conclusion': 'success', 'steps': [
                        {'name': 'Run tests (all others)', 'conclusion': 'success'},
                        {'name': 'Record validation evidence', 'conclusion': 'success'}]}
        self.tree = TREE
        self.calls = []
        self.no_runs = False
        self.no_artifacts = False
        self.no_jobs = False
        self.second_page = False
        self.additional_runs = []

    def get(self, path):
        self.calls.append(path)
        if path.startswith('actions/runs?'):
            return {'workflow_runs': self.additional_runs}
        if path.startswith('actions/artifacts?'):
            return {'artifacts': [] if self.no_artifacts else [self.artifact]}
        if path == 'actions/runs/42':
            return self.run
        if path.startswith('git/commits/'):
            return {'tree': {'sha': self.tree}}
        if path.startswith('actions/runs/42/jobs?'):
            if self.no_jobs:
                return {'jobs': []}
            if self.second_page and path.endswith('&page=1'):
                return {'jobs': [{'name': f'unrelated-{i}', 'conclusion': 'success'} for i in range(100)]}
            return {'jobs': [self.job]}
        if path.startswith('actions/workflows/ci.yml/runs?'):
            return {'workflow_runs': [] if self.no_runs else [self.run]}
        raise AssertionError(f'Unexpected endpoint: {path}')


class ReuseContracts(unittest.TestCase):
    def setUp(self):
        self.api = FixtureAPI()
        self.inputs = {'tree': TREE, 'job': self.api.job['name']}

    def find(self):
        return evidence.find_evidence(self.api, 'receipt', self.inputs, 'ci.yml', NOW)

    def test_identical_tree_accepts_different_commit_identity(self):
        self.assertEqual(self.find()['run_id'], 42)
        self.api.run['head_sha'] = 'c' * 40
        self.assertEqual(self.find()['artifact_id'], 12)

    def test_changed_tree_or_workflow_is_not_reusable(self):
        self.api.tree = 'd' * 40
        self.assertIsNone(self.find())
        self.api.tree = TREE
        self.api.run['path'] = '.github/workflows/unrelated.yml'
        self.assertIsNone(self.find())

    def test_failed_cancelled_pending_and_fork_runs_are_rejected(self):
        for conclusion in ('failure', 'cancelled', 'timed_out', 'skipped', None):
            with self.subTest(conclusion=conclusion):
                self.api.run['conclusion'] = conclusion
                self.assertIsNone(self.find())
        self.api.run['conclusion'] = 'success'
        self.api.run['status'] = 'in_progress'
        self.assertIsNone(self.find())
        self.api.run['status'] = 'completed'
        self.api.run['head_repository']['full_name'] = 'fork/repo'
        self.assertIsNone(self.find())

    def test_incomplete_job_or_reused_receipt_cannot_renew_evidence(self):
        for conclusion in ('failure', 'cancelled', 'skipped', None):
            with self.subTest(conclusion=conclusion):
                self.api.job['steps'][-1]['conclusion'] = conclusion
                self.assertIsNone(self.find())
        self.api.job['steps'] = []
        self.assertIsNone(self.find())
        self.api.no_jobs = True
        self.assertIsNone(self.find())

    def test_receipt_missing_expired_stale_or_future_is_rejected(self):
        original = copy.deepcopy(self.api.artifact)
        for change in ({'expired': True}, {'name': 'another-fingerprint'},
                       {'created_at': '2026-10-18T10:00:00Z'},
                       {'created_at': '2026-10-19T13:00:00Z'}):
            with self.subTest(change=change):
                self.api.artifact = original | change
                self.assertIsNone(self.find())
        self.api.artifact = original
        self.api.run['run_started_at'] = '2026-10-17T11:00:00Z'
        self.assertIsNone(self.find())
        self.api.no_artifacts = True
        self.assertIsNone(self.find())

    def test_jobs_are_paginated(self):
        self.api.second_page = True
        self.assertEqual(self.find()['job_id'], 7)
        self.assertTrue(any('page=2' in path for path in self.api.calls))

    def test_current_run_cannot_attest_itself(self):
        with patch.dict(os.environ, {'GITHUB_RUN_ID': '42'}):
            self.assertIsNone(self.find())

    def test_fingerprint_tracks_tree_environment_toolchain_and_role(self):
        values = {'git': TREE, 'rustc': 'rustc 1.90.0', 'cargo': 'cargo 1.90.0'}
        with patch.dict(os.environ, {'ImageOS': 'ubuntu24', 'ImageVersion': '20261018', 'RUNNER_ARCH': 'X64'}), patch.object(evidence, 'command', side_effect=lambda *args: values[args[0]]):
            baseline = evidence.fingerprint('tests', 'job', '')[1]
            for tool in ('git', 'rustc', 'cargo'):
                old = values[tool]
                values[tool] += '-changed'
                self.assertNotEqual(evidence.fingerprint('tests', 'job', '')[1], baseline)
                values[tool] = old
            for key in ('ImageVersion', 'ImageOS', 'RUNNER_ARCH', 'RUSTFLAGS'):
                with patch.dict(os.environ, {key: 'changed'}):
                    self.assertNotEqual(evidence.fingerprint('tests', 'job', '')[1], baseline)
            self.assertNotEqual(evidence.fingerprint('optimized', 'job', '')[1], baseline)
            self.assertNotEqual(evidence.fingerprint('tests', 'other-platform', '')[1], baseline)
            self.assertNotEqual(evidence.fingerprint('tests', 'job', 'changed-external-tool')[1], baseline)

    def test_api_outage_runs_full_validation(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)/'output'
            summary = Path(directory)/'summary'
            previous = Path.cwd()
            try:
                os.chdir(directory)
                environment = {'GITHUB_OUTPUT': str(output), 'GITHUB_STEP_SUMMARY': str(summary),
                               'GITHUB_REPOSITORY': 'owner/repo', 'GH_TOKEN': 'fixture-token'}
                with patch.dict(os.environ, environment), patch.object(evidence, 'fingerprint', return_value=(self.inputs, 'receipt')), patch.object(evidence.GitHub, 'get', side_effect=OSError('API unavailable')):
                    evidence.probe(SimpleNamespace(role='tests', job='Test', workflow='ci.yml', extra=''))
            finally:
                os.chdir(previous)
            self.assertIn('hit=false\n', output.read_text())
            self.assertNotIn('hit=true', output.read_text())


class PublicationContracts(unittest.TestCase):
    def setUp(self):
        self.api = FixtureAPI()
        self.required = {'ci.yml': [self.api.job['name']]}

    def status(self):
        return coverage_status(self.api, SHA, self.required, NOW)

    def test_complete_expected_current_main_coverage_passes(self):
        self.assertEqual(self.status(), [])
        self.api.second_page = True
        self.assertEqual(self.status(), [])

    def test_scheduled_failure_on_candidate_cannot_be_hidden_by_green_push(self):
        self.api.additional_runs = [self.api.run | {'id': 43, 'event': 'schedule', 'conclusion': 'failure'}]
        with self.assertRaisesRegex(ValueError, '43'):
            self.status()

    def test_additional_running_validation_waits_and_successful_rerun_passes(self):
        scheduled = self.api.run | {'id': 43, 'event': 'schedule', 'status': 'in_progress', 'conclusion': None}
        self.api.additional_runs = [scheduled]
        self.assertTrue(self.status())
        scheduled.update(status='completed', conclusion='success')
        self.assertEqual(self.status(), [])

    def test_other_commit_scheduled_failure_does_not_block_candidate(self):
        self.api.additional_runs = [self.api.run | {'id': 43, 'event': 'schedule', 'head_sha': 'f'*40, 'conclusion': 'failure'}]
        self.assertEqual(self.status(), [])

    def test_empty_policy_or_missing_matrix_member_blocks(self):
        with self.assertRaises(ValueError):
            coverage_status(self.api, SHA, {}, NOW)
        self.required['ci.yml'].append('Test (windows-latest, stable)')
        with self.assertRaisesRegex(ValueError, 'windows-latest'):
            self.status()

    def test_no_runs_pending_wrong_sha_and_wrong_branch_wait(self):
        for field, value in (('head_sha', 'f'*40), ('head_branch', 'develop'),
                             ('status', 'in_progress'), ('path', '.github/workflows/other.yml')):
            original = self.api.run[field]
            self.api.run[field] = value
            self.assertTrue(self.status())
            self.api.run[field] = original
        self.api.no_runs = True
        self.assertTrue(self.status())

    def test_failed_cancelled_missing_and_skipped_jobs_block(self):
        for conclusion in ('failure', 'cancelled', 'timed_out', 'skipped', None):
            self.api.run['conclusion'] = conclusion
            with self.assertRaises(ValueError):
                self.status()
        self.api.run['conclusion'] = 'success'
        self.api.job['conclusion'] = 'skipped'
        with self.assertRaises(ValueError):
            self.status()
        self.api.no_jobs = True
        with self.assertRaises(ValueError):
            self.status()

    def test_stale_evidence_and_api_outage_do_not_authorize_release(self):
        self.api.run['run_started_at'] = '2026-10-17T11:00:00Z'
        with self.assertRaisesRegex(ValueError, 'older than'):
            self.status()
        with patch.object(self.api, 'get', side_effect=OSError('connection refused')):
            with self.assertRaises(OSError):
                self.status()

    def test_tag_version_and_main_merge_identity(self):
        pulls = [{'merged_at': STAMP, 'base': {'ref': 'main'}, 'merge_commit_sha': SHA}]
        validate('v5.5.0', '5.5.0', SHA, pulls)
        for tag, version, sha, candidates in (
            ('v5.4.2', '5.5.0', SHA, pulls),
            ('v5.5.0;echo bad', '5.5.0', SHA, pulls),
            ('v5.5.0', '5.5.0', 'other', pulls),
            ('v5.5.0', '5.5.0', SHA, []),
            ('v5.5.0', '5.5.0', SHA, [pulls[0] | {'base': {'ref': 'develop'}}]),
        ):
            with self.assertRaises(ValueError):
                validate(tag, version, sha, candidates)

    def test_registry_identity_and_yank_are_checked(self):
        data = {'version': {'num': '5.5.0', 'checksum': 'abc', 'yanked': False}}
        verify(data, '5.5.0', 'abc')
        for changed in ({'num': '5.4.2'}, {'checksum': 'def'}, {'yanked': True}):
            bad = {'version': data['version'] | changed}
            with self.assertRaises(ValueError):
                verify(bad, '5.5.0', 'abc')


if __name__ == '__main__':
    unittest.main()
