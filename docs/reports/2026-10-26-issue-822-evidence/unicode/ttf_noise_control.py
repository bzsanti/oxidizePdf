import json, pathlib, random, statistics, subprocess, resource, os
p=pathlib.Path(__file__).resolve().parent
os.sched_setaffinity(0,{0})
rng=random.Random(82205)
rows=[]
for block in range(24):
    order=['control','same_control','candidate'];rng.shuffle(order)
    for variant in order:
        binary='control' if variant=='same_control' else variant
        before=resource.getrusage(resource.RUSAGE_CHILDREN)
        value=json.loads(subprocess.check_output([str(p/f'unicode-{binary}'),'ttf','10','1000',str(p/'noise-output.pdf'),'serialize','default'],text=True))
        after=resource.getrusage(resource.RUSAGE_CHILDREN)
        value.update(block=block,variant=variant,cpu_seconds=after.ru_utime+after.ru_stime-before.ru_utime-before.ru_stime,involuntary_switches=after.ru_nivcsw-before.ru_nivcsw)
        rows.append(value)
    (p/'ttf-noise-samples.json').write_text(json.dumps(rows,indent=2))
    print('completed block',block+1,flush=True)
summary={}
for metric in ['elapsed_ns','cpu_seconds']:
    for variant in ['same_control','candidate']:
        diffs=[]
        for block in range(24):
            pair={v['variant']:v[metric] for v in rows if v['block']==block}
            diffs.append(100*(1-pair[variant]/pair['control']))
        boot=sorted(statistics.median(rng.choices(diffs,k=24)) for _ in range(10000))
        summary[metric+'_'+variant]={'median_reduction_percent':statistics.median(diffs),'bootstrap_95_percent':[boot[249],boot[9749]]}
(p/'ttf-noise-summary.json').write_text(json.dumps(summary,indent=2))
print(json.dumps(summary,indent=2))
