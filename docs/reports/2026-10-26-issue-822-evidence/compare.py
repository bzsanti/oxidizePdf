import json
import pathlib
import random
import statistics
import subprocess

p = pathlib.Path(__file__).resolve().parent
rng = random.Random(822)
rows = []
for block in range(16):
    cases = [(mode, pages) for mode in ['default', 'raw'] for pages in [1, 10, 100]]
    rng.shuffle(cases)
    for mode, pages in cases:
        variants = ['control', 'candidate']
        rng.shuffle(variants)
        for variant in variants:
            result = json.loads(subprocess.check_output([
                str(p / variant), mode, str(pages),
                str({1: 1000, 10: 250, 100: 30}[pages]),
                str(p / f'{variant}-{mode}-{pages}.pdf'),
            ], text=True))
            result.update(block=block, variant=variant)
            rows.append(result)
(p / 'paired-samples.json').write_text(json.dumps(rows, indent=2))
summary = []
for mode in ['default', 'raw']:
    for pages in [1, 10, 100]:
        values = {}
        for variant in ['control', 'candidate']:
            values[variant] = [v['elapsed_ns'] / v['documents'] / 1000
                for v in sorted(rows, key=lambda v: v['block'])
                if v['mode'] == mode and v['pages'] == pages and v['variant'] == variant]
        reductions = [100 * (1 - c / b) for b, c in zip(values['control'], values['candidate'])]
        bootstrap = sorted(statistics.median(rng.choices(reductions, k=len(reductions))) for _ in range(10000))
        summary.append(dict(mode=mode, pages=pages,
            control_us=statistics.median(values['control']),
            candidate_us=statistics.median(values['candidate']),
            median_paired_reduction_percent=statistics.median(reductions),
            bootstrap_95_percent=[bootstrap[249], bootstrap[9749]]))
(p / 'paired-summary.json').write_text(json.dumps(summary, indent=2))
print(json.dumps(summary, indent=2))
