import json, random, statistics, sys
from pathlib import Path
rows = json.loads(Path(sys.argv[1]).read_text())['samples']
rng = random.Random(854)
summary = []
for case in dict.fromkeys(row['case'] for row in rows):
    by_label = {label: sorted((r for r in rows if r['case'] == case and r['label'] == label), key=lambda r:r['sample']) for label in ('baseline','candidate')}
    b, c = by_label.values()
    if not case.startswith('generate'):
        assert len({(r['units'],r['digest']) for r in b+c}) == 1, case + ': content mismatch'
    result = {'case':case, 'samples_per_binary':len(b)}
    for field in ('elapsed_ns','max_rss_kb'):
        ratios = [100*(1-y[field]/x[field]) for x,y in zip(b,c)]
        bootstrap = sorted(statistics.median(rng.choices(ratios,k=len(ratios))) for _ in range(10000))
        result[field] = {'baseline_median':statistics.median(r[field] for r in b), 'candidate_median':statistics.median(r[field] for r in c), 'paired_median_reduction_percent':statistics.median(ratios), 'bootstrap_95_interval_percent':[bootstrap[250],bootstrap[9750]], 'baseline_range':[min(r[field] for r in b),max(r[field] for r in b)], 'candidate_range':[min(r[field] for r in c),max(r[field] for r in c)]}
    summary.append(result)
print(json.dumps(summary,indent=2))
