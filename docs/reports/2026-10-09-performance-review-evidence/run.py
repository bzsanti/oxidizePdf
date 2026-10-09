import subprocess,json,random,statistics,pathlib,platform
p=pathlib.Path(__file__).resolve().parent
binary=p/'target/release/writer-perf-probe'
r=random.Random(1009); rows=[]
for block in range(12):
 cases=[(m,n) for m in ['default','raw'] for n in [1,10,100]]; r.shuffle(cases)
 for mode,pages in cases:
  out=p/f'{mode}-{pages}.pdf'
  v=json.loads(subprocess.check_output([str(binary),mode,str(pages),str({1:300,10:60,100:10}[pages]),str(out)],text=True)); v['block']=block; rows.append(v)
p.joinpath('timings.json').write_text(json.dumps(rows,indent=2))
summary=[]
for mode in ['default','raw']:
 for pages in [1,10,100]:
  cohort=[v for v in rows if v['mode']==mode and v['pages']==pages]
  values=[v['elapsed_ns']/v['documents']/1000 for v in cohort]
  summary.append(dict(mode=mode,pages=pages,median_us=statistics.median(values),min_us=min(values),max_us=max(values),output_bytes=cohort[-1]['bytes']))
p.joinpath('summary.json').write_text(json.dumps(summary,indent=2));print(json.dumps(summary,indent=2))
