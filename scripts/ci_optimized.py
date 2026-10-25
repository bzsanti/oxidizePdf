#!/usr/bin/env python3
"""Partition Cargo's default release test targets and require every shard report."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

from ci_evidence import fingerprint

SHARDS = 8


def command(*args):
    return subprocess.check_output(args, text=True).strip()


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


def plan(metadata, tree, rust):
    # Separate package invocations would change workspace feature unification.
    # Fail explicitly if the workspace grows instead of silently losing coverage.
    members = metadata['workspace_members']
    if len(members) != 1:
        raise ValueError('Optimized partitioning requires one workspace package')
    package = next(p for p in metadata['packages'] if p['id'] == members[0])
    features = next(set(n['features']) for n in metadata['resolve']['nodes']
                    if n['id'] == members[0])
    shards = {str(i): [] for i in range(SHARDS)}
    shards['unit-doc'] = []
    excluded = []
    index = 0
    for target in sorted(package['targets'], key=lambda t: (t['kind'], t['name'])):
        kind, name = target['kind'], target['name']
        if not set(target.get('required-features', [])) <= features:
            excluded.append({'name': name, 'kind': kind, 'reason': 'required-features'})
            continue
        tasks = []
        if kind == ['lib']:
            if target['test']:
                tasks.append(('test', '--lib'))
            if target['doctest']:
                tasks.append(('test', '--doc'))
        elif kind == ['test']:
            if target['test']:
                tasks.append(('test', '--test'))
        elif kind == ['example']:
            tasks.append(('test' if target['test'] else 'build', '--example'))
        elif kind == ['bin']:
            # Integration tests also need ordinary binaries for CARGO_BIN_EXE_*.
            tasks.append(('build', '--bin'))
            if target['test']:
                tasks.append(('test', '--bin'))
        elif kind != ['bench'] and kind != ['custom-build']:
            raise ValueError(f'Unsupported Cargo target kind: {kind}')
        if not tasks:
            excluded.append({'name': name, 'kind': kind, 'reason': 'not-default-test'})
        for operation, selector in tasks:
            task = {'name': name, 'operation': operation, 'selector': selector}
            if kind == ['lib']:
                shards['unit-doc'].append(task)
            else:
                shards[str(index % SHARDS)].append(task)
                index += 1
    if not any(shards.values()):
        raise ValueError('No optimized targets found')
    return {'schema': 1, 'tree': tree, 'rust': rust, 'package': package['name'],
            'shards': {k: v for k, v in shards.items() if v}, 'excluded': excluded}


def commands(plan_data, shard):
    grouped = {}
    for target in plan_data['shards'][shard]:
        key = (target['operation'], target['selector'])
        grouped.setdefault(key, []).append(target['name'])
    result = []
    for (operation, selector), names in grouped.items():
        args = ['cargo', operation, '--locked', '--release', '-p', plan_data['package']]
        if selector in ('--lib', '--doc'):
            args.append(selector)
        else:
            for name in names:
                args.extend((selector, name))
        result.append(args)
    return result


def run(plan_data, shard, execute=subprocess.run, reuse=None):
    started = time.monotonic()
    results = []
    for args in commands(plan_data, shard):
        if reuse is None:
            execute(args, check=True)
        results.append({'command': args, 'returncode': 0})
    return {'shard': shard, 'plan': digest(plan_data), 'tree': plan_data['tree'],
            'results': results, 'reuse': reuse, 'elapsed_seconds': time.monotonic() - started}


def verify(plan_data, reports):
    expected = set(plan_data['shards'])
    found = [r['shard'] for r in reports]
    if len(found) != len(expected) or set(found) != expected:
        raise ValueError('Missing, duplicate or unexpected optimized shard')
    for report in reports:
        if report['plan'] != digest(plan_data) or report['tree'] != plan_data['tree']:
            raise ValueError('Report belongs to a different source or target plan')
        expected_results = [{'command': c, 'returncode': 0}
                            for c in commands(plan_data, report['shard'])]
        if report['results'] != expected_results:
            raise ValueError('Incomplete or failed optimized commands')


def matching_environments(current, reports):
    # Role/job/plan identify each shard, not compiler or runner compatibility.
    def environment(inputs):
        return {k: v for k, v in inputs.items() if k not in ('role', 'job', 'extra')}
    return bool(reports) and all(environment(r.get('environment', {})) == environment(current)
                                 for r in reports)


def load_plan(path, expected_digest):
    data = json.loads(Path(path).read_text())
    if digest(data) != expected_digest:
        raise ValueError('Optimized plan digest mismatch')
    if command('git', 'rev-parse', 'HEAD^{tree}') != data['tree']:
        raise ValueError('Optimized plan source mismatch')
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=('plan', 'run', 'verify', 'environment'))
    parser.add_argument('--plan', default='optimized-plan.json')
    parser.add_argument('--digest')
    parser.add_argument('--shard')
    parser.add_argument('--reuse-file')
    parser.add_argument('--inputs-file')
    parser.add_argument('--reports', default='optimized-reports')
    args = parser.parse_args()
    if args.operation == 'plan':
        metadata = json.loads(command('cargo', 'metadata', '--locked', '--format-version', '1'))
        data = plan(metadata, command('git', 'rev-parse', 'HEAD^{tree}'), command('rustc', '-Vv'))
        Path(args.plan).write_text(json.dumps(data, indent=2) + '\n')
        version = command('rustc', '--version').split()[1]
        with open(os.environ['GITHUB_OUTPUT'], 'a') as output:
            output.write(f'digest={digest(data)}\n')
            output.write(f'matrix={json.dumps(list(data["shards"]))}\n')
            output.write(f'toolchain={version}\n')
    else:
        data = load_plan(args.plan, args.digest)
        if args.operation == 'run':
            if command('rustc', '-Vv') != data['rust']:
                raise ValueError('Optimized compiler differs from planner')
            reuse = json.loads(Path(args.reuse_file).read_text()) if args.reuse_file else None
            report = run(data, args.shard, reuse=reuse)
            report['environment'] = json.loads(Path(args.inputs_file).read_text())
            Path(args.reports).mkdir(exist_ok=True)
            Path(args.reports, f'{args.shard}.json').write_text(json.dumps(report, indent=2))
        else:
            reports = [json.loads(p.read_text()) for p in Path(args.reports).glob('*.json')]
            verify(data, reports)
            if args.operation == 'environment':
                current, _ = fingerprint('publisher', 'Release', '')
                match = matching_environments(current, reports)
                with open(os.environ['GITHUB_OUTPUT'], 'a') as output:
                    output.write(f'match={str(match).lower()}\n')
            print(f'Validated {len(reports)} shards, '
                  f'{sum(map(len, data["shards"].values()))} target operations')


if __name__ == '__main__':
    main()
