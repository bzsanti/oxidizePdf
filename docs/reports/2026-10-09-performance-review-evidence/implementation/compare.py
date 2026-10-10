import subprocess,json,random,statistics,pathlib,sys
p=pathlib.Path(__file__).resolve().parent
label=sys.argv[1];control=p/sys.argv[2];candidate=p/sys.argv[3];kind=sys.argv[4]
if kind=='layout': cases=[('layout-standard',300),('layout-custom',20),('flow',20),('flow-raw',20)]
elif kind=='encoding': cases=[('accent',150),('accent-raw',150),('cjk',60),('cjk-raw',60),('flow',50)]
elif kind=='graphics': cases=[('graphics',60),('graphics-raw',100),('graphics-fractional',60),('graphics-fractional-raw',100)]
elif kind=='extended': cases=[('accent',50),('accent-raw',50),('graphics',30),('graphics-raw',30),('flow',20)]
else: cases=[('default',1),('default',10),('default',100),('raw',1),('raw',10),('raw',100)]
r=random.Random(700);rows=[]
for block in range(12):
 order=[(v,c) for v in ['control','candidate'] for c in cases];r.shuffle(order)
 for variant,(mode,n) in order:
  bin=control if variant=='control' else candidate
  out=p/f'{label}-{variant}-{mode}-{n}.out'
  args=[str(bin),mode,str(n),str(out)] if kind!='invoice' else [str(bin),mode,str(n),str({1:300,10:60,100:10}[n]),str(out)]
  x=json.loads(subprocess.check_output(args,text=True));x.update(block=block,variant=variant,case=f'{mode}-{n}');rows.append(x)
summary=[]
for case in sorted({x['case'] for x in rows}):
 med={v:statistics.median(x['elapsed_ns']/x.get('iterations',x.get('documents',1))/1000 for x in rows if x['case']==case and x['variant']==v) for v in ['control','candidate']}
 paired=[]
 for b in range(12):
  pair={x['variant']:x['elapsed_ns'] for x in rows if x['case']==case and x['block']==b};paired.append(1-pair['candidate']/pair['control'])
 boots=sorted(statistics.median(r.choices(paired,k=12)) for _ in range(10000))
 summary.append(dict(case=case,median_us=med,ratio_improvement=1-med['candidate']/med['control'],paired_median=statistics.median(paired),paired_bootstrap95=[boots[250],boots[9750]]))
(p/f'{label}-samples.json').write_text(json.dumps(rows,indent=2));(p/f'{label}-summary.json').write_text(json.dumps(summary,indent=2));print(json.dumps(summary,indent=2))
