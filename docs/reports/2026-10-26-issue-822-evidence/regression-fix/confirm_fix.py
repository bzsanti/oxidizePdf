import pathlib,json,subprocess,os,random,statistics
P=pathlib.Path('/tmp/oxidize-perf822');R=pathlib.Path('/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf');E=R/'docs/reports/2026-10-26-issue-822-evidence/regression-fix';os.sched_setaffinity(0,{0});rng=random.Random(82211);rows=[]
f=R/'test-corpus/t1-spec/verapdf/veraPDF test suite 6-2-11-7-3-t01-pass-e.pdf'
for b in range(15):
 variants=['control','step2','candidate','fixed'];rng.shuffle(variants)
 for v in variants:
  q=subprocess.run([str(P/f'corpus-{v}'),str(f),'cjk','10','-'],capture_output=True,text=True,check=True,timeout=15)
  x=json.loads(q.stdout);x.update(variant=v,block=b);rows.append(x)
 print('block',b+1,flush=True)
(E/'focus-samples.json').write_text(json.dumps(rows,indent=2));out={}
for c,t in [('control','candidate'),('control','fixed'),('step2','fixed'),('candidate','fixed')]:
 ds=[]
 for b in range(15):
  g={x['variant']:statistics.mean(x['serialize_ns'][1:]) for x in rows if x['block']==b};ds.append(100*(1-g[t]/g[c]))
 boot=sorted(statistics.median(rng.choices(ds,k=15)) for _ in range(10000));out[t+'_vs_'+c]={'median_reduction_percent':statistics.median(ds),'bootstrap95':[boot[249],boot[9749]]}
(E/'focus-summary.json').write_text(json.dumps(out,indent=2));print(json.dumps(out,indent=2))
