"""Coverage and failure contracts for partitioned release validation."""
import copy
import subprocess
import unittest
from unittest.mock import Mock

import ci_optimized as ci


def target(name, kind, test=True, doctest=False, required=()):
    return {'name': name, 'kind': [kind], 'test': test, 'doctest': doctest,
            'required-features': list(required)}


def metadata():
    return {'workspace_members': ['pkg'], 'resolve': {'nodes': [
        {'id': 'pkg', 'features': ['default', 'enabled']}]}, 'packages': [
            {'id': 'pkg', 'name': 'fixture', 'targets': [
                target('core', 'lib', doctest=True),
                target('cli', 'bin'),
                target('integration', 'test'),
                target('disabled', 'test', test=False),
                target('available', 'test', required=['enabled']),
                target('optional', 'test', required=['missing']),
                target('demo', 'example', test=False),
                target('tested_demo', 'example'),
                target('bench', 'bench')]}]}


class OptimizedTests(unittest.TestCase):
    def setUp(self):
        self.plan = ci.plan(metadata(), 'tree', 'rust')

    def reports(self):
        return [ci.run(self.plan, shard, execute=Mock()) for shard in self.plan['shards']]

    def test_cargo_default_target_coverage(self):
        tasks = [(t['name'], t['operation'], t['selector'])
                 for tasks in self.plan['shards'].values() for t in tasks]
        self.assertCountEqual(tasks, [
            ('core', 'test', '--lib'), ('core', 'test', '--doc'),
            ('cli', 'build', '--bin'), ('cli', 'test', '--bin'),
            ('integration', 'test', '--test'), ('available', 'test', '--test'),
            ('demo', 'build', '--example'), ('tested_demo', 'test', '--example')])
        self.assertEqual(len(tasks), len(set(tasks)))

    def test_deterministic_under_metadata_reordering(self):
        source = metadata()
        source['packages'][0]['targets'].reverse()
        self.assertEqual(ci.plan(source, 'tree', 'rust'), self.plan)

    def test_new_targets_are_included_and_bounded(self):
        source = metadata()
        source['packages'][0]['targets'] += [target(f'new_{i}', 'test') for i in range(100)]
        plan = ci.plan(source, 'tree', 'rust')
        names = [t['name'] for tasks in plan['shards'].values() for t in tasks]
        self.assertTrue(all(names.count(f'new_{i}') == 1 for i in range(100)))
        self.assertLessEqual(len(plan['shards']), 9)

    def test_workspace_expansion_and_unknown_kinds_fail_closed(self):
        source = metadata()
        source['workspace_members'].append('other')
        with self.assertRaisesRegex(ValueError, 'one workspace'):
            ci.plan(source, 'tree', 'rust')
        source = metadata()
        source['packages'][0]['targets'].append(target('new', 'unknown'))
        with self.assertRaisesRegex(ValueError, 'Unsupported'):
            ci.plan(source, 'tree', 'rust')

    def test_execute_preserves_profile_lock_package_and_doc_separation(self):
        calls = []
        ci.run(self.plan, 'unit-doc', execute=lambda args, check: calls.append((args, check)))
        self.assertEqual(calls, [
            (['cargo', 'test', '--locked', '--release', '-p', 'fixture', '--lib'], True),
            (['cargo', 'test', '--locked', '--release', '-p', 'fixture', '--doc'], True)])

    def test_command_failure_does_not_produce_success_report(self):
        execute = Mock(side_effect=subprocess.CalledProcessError(1, ['cargo']))
        with self.assertRaises(subprocess.CalledProcessError):
            ci.run(self.plan, 'unit-doc', execute=execute)
        self.assertEqual(execute.call_count, 1)

    def test_publisher_requires_every_shard_environment(self):
        current = {'tree': 'same', 'image': {'ImageVersion': 'A'}, 'rust': 'exact',
                   'role': 'publisher', 'job': 'Release', 'extra': ''}
        shard = dict(current, role='optimized-0', job='shard', extra='digest')
        self.assertTrue(ci.matching_environments(current, [{'environment': shard}]))
        for changed in ({}, dict(shard, image={'ImageVersion': 'B'}),
                        dict(shard, rust='different'), dict(shard, tree='different')):
            self.assertFalse(ci.matching_environments(current, [
                {'environment': shard}, {'environment': changed}]))
        self.assertFalse(ci.matching_environments(current, []))

    def test_complete_reports_pass(self):
        ci.verify(self.plan, self.reports())

    def test_missing_duplicate_extra_and_failed_reports_are_rejected(self):
        reports = self.reports()
        variants = [reports[:-1], reports + [reports[0]], reports[1:] + [reports[1]]]
        wrong_shard = copy.deepcopy(reports)
        wrong_shard[0]['shard'] = 'unknown'
        variants.append(wrong_shard)
        for field, value in [('tree', 'other'), ('plan', 'other'), ('results', [])]:
            invalid = copy.deepcopy(reports)
            invalid[0][field] = value
            variants.append(invalid)
        failed = copy.deepcopy(reports)
        failed[0]['results'][0]['returncode'] = 1
        variants.append(failed)
        for invalid in variants:
            with self.subTest(invalid=invalid):
                with self.assertRaises(ValueError):
                    ci.verify(self.plan, invalid)

    def test_reused_validation_retains_origin_and_executes_no_commands(self):
        execute = Mock(side_effect=AssertionError('reused validation executed'))
        origin = {'run_id': 10, 'job_id': 20, 'artifact_id': 30, 'url': 'https://example.test/10'}
        report = ci.run(self.plan, 'unit-doc', execute=execute, reuse=origin)
        self.assertEqual(report['reuse'], origin)
        self.assertEqual(len(report['results']), 2)
        execute.assert_not_called()


if __name__ == '__main__':
    unittest.main()
