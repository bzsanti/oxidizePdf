import json
import pathlib
import random
import statistics
import subprocess

p = pathlib.Path(__file__).resolve().parent
rng = random.Random(82203)
rows = []
for block in range(16):
    cases = [(kind, phase) for kind in ['latin', 'ttf', 'cjk'] for phase in ['full', 'serialize']]
    rng.shuffle(cases)
    for kind, phase in cases:
        variants = ['control', 'candidate']
        rng.shuffle(variants)
        for variant in variants:
            result = json.loads(subprocess.check_output([
                str(p / f'unicode-{variant}'), kind, '10',
                str({'latin': 200, 'ttf': 60, 'cjk': 5}[kind]),
                str(p / f'unicode-{variant}-{kind}-{phase}.pdf'), phase, 'default',
            ], text=True))
            result.update(block=block, variant=variant)
            rows.append(result)
(p / 'unicode-paired-samples.json').write_text(json.dumps(rows, indent=2))
summary = []
for kind in ['latin', 'ttf', 'cjk']:
    for phase in ['full', 'serialize']:
        values = {}
        for variant in ['control', 'candidate']:
            values[variant] = [v['elapsed_ns'] / v['documents'] / 1000
                for v in sorted(rows, key=lambda v: v['block'])
                if v['kind'] == kind and v['phase'] == phase and v['variant'] == variant]
        reductions = [100 * (1 - c / b) for b, c in zip(values['control'], values['candidate'])]
        bootstrap = sorted(statistics.median(rng.choices(reductions, k=len(reductions))) for _ in range(10000))
        summary.append(dict(kind=kind, phase=phase,
            control_us=statistics.median(values['control']),
            candidate_us=statistics.median(values['candidate']),
            median_paired_reduction_percent=statistics.median(reductions),
            bootstrap_95_percent=[bootstrap[249], bootstrap[9749]]))
(p / 'unicode-paired-summary.json').write_text(json.dumps(summary, indent=2))
print(json.dumps(summary, indent=2))
