import json
import pathlib
import random
import statistics
import subprocess

p = pathlib.Path(__file__).resolve().parent
rng = random.Random(82202)
rows = []
for block in range(20):
    cases = [('ascii', 80), ('ascii', 16384), ('latin', 16384), ('unique', 4096)]
    rng.shuffle(cases)
    for kind, count in cases:
        variants = ['control', 'candidate']
        rng.shuffle(variants)
        for variant in variants:
            result = json.loads(subprocess.check_output([
                str(p / f'usage-{variant}'), kind, str(count),
                str(1000 if count == 80 else 200),
                str(p / f'usage-{variant}-{kind}-{count}.pdf'),
            ], text=True))
            result.update(block=block, variant=variant)
            rows.append(result)
(p / 'usage-paired-samples.json').write_text(json.dumps(rows, indent=2))
summary = []
for kind, count in [('ascii', 80), ('ascii', 16384), ('latin', 16384), ('unique', 4096)]:
    values = {}
    for variant in ['control', 'candidate']:
        values[variant] = [v['elapsed_ns'] / v['documents'] / 1000
            for v in sorted(rows, key=lambda v: v['block'])
            if v['kind'] == kind and v['characters'] == count and v['variant'] == variant]
    reductions = [100 * (1 - c / b) for b, c in zip(values['control'], values['candidate'])]
    bootstrap = sorted(statistics.median(rng.choices(reductions, k=len(reductions))) for _ in range(10000))
    summary.append(dict(kind=kind, characters=count,
        control_us=statistics.median(values['control']),
        candidate_us=statistics.median(values['candidate']),
        median_paired_reduction_percent=statistics.median(reductions),
        bootstrap_95_percent=[bootstrap[249], bootstrap[9749]]))
(p / 'usage-paired-summary.json').write_text(json.dumps(summary, indent=2))
print(json.dumps(summary, indent=2))
