import json,random,statistics
from pathlib import Path
root=Path(__file__).resolve().parent
rng=random.Random(661)
def intervals(folder,variants):
    rows=[json.loads(s) for s in (folder/'samples.jsonl').read_text().splitlines()]
    result=[]
    for mode,pages in sorted({(r['mode'],r['pages']) for r in rows}):
        values={v:{r['block']:r['elapsed_ns']/r['documents'] for r in rows if r['variant']==v and r['mode']==mode and r['pages']==pages} for v in variants}
        for i,v in enumerate(variants[1:],1):
            for base in dict.fromkeys([variants[0],variants[i-1]]):
                xs=[100*(1-values[v][b]/values[base][b]) for b in sorted(values[base])]
                res=sorted(statistics.median(rng.choices(xs,k=len(xs))) for _ in range(10000))
                result.append(dict(mode=mode,pages=pages,base=base,variant=v,median_paired_improvement_pct=statistics.median(xs),bootstrap_95pct=[res[250],res[9750]]))
    (folder/'paired-uncertainty.json').write_text(json.dumps(result,indent=2)+'\n')
intervals(root/'results',['control','formatting','compression'])
intervals(root/'extended-results',['control','compression'])
