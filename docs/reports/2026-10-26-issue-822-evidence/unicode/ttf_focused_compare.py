import json
import pathlib
import random
import statistics
import subprocess
import os
import sys

p = pathlib.Path(__file__).resolve().parent
cpu = int(sys.argv[1]) if len(sys.argv) > 1 else None
if cpu is not None:
    os.sched_setaffinity(0, {cpu})
suffix = '-pinned' if cpu is not None else ''
rng = random.Random(82304)
rows = []
for block in range(24):
    order = ['control', 'candidate']
    rng.shuffle(order)
    for variant in order:
        value = json.loads(subprocess.check_output([
            str(p / f'unicode-{variant}'), 'ttf', '10', '250',
            str(p / f'ttf-focused-{variant}.pdf'), 'serialize', 'default'
        ], text=True))
        value.update(block=block, variant=variant)
        rows.append(value)
reductions = []
for block in range(24):
    pair = {v['variant']: v['elapsed_ns'] / v['documents'] for v in rows if v['block'] == block}
    reductions.append(100 * (1 - pair['candidate'] / pair['control']))
boot = sorted(statistics.median(rng.choices(reductions, k=24)) for _ in range(10000))
summary = dict(blocks=24, iterations=250, cpu=cpu, median_paired_reduction_percent=statistics.median(reductions),
               bootstrap_95_percent=[boot[249], boot[9749]])
(p / f'ttf-focused{suffix}-samples.json').write_text(json.dumps(rows, indent=2))
(p / f'ttf-focused{suffix}-summary.json').write_text(json.dumps(summary, indent=2))
print(json.dumps(summary, indent=2))
