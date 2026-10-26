import pathlib,json,subprocess,resource,random,statistics,math,os
P=pathlib.Path('/tmp/oxidize-perf822');R=pathlib.Path('/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf');E=R/'docs/reports/2026-10-26-issue-822-evidence/corpus'
flags=[r for m in json.loads((E/'summary.json').read_text())['sample'].values() for r in m['regression_flags']]
os.sched_setaffinity(0,{0});rng=random.Random(82209);rows=[]
def limits():
 resource.setrlimit(resource.RLIMIT_AS,(2147483648,2147483648));resource.setrlimit(resource.RLIMIT_CORE,(0,0))
for block in range(15):
 order=flags[:];rng.shuffle(order)
 for r in order:
  variants=['control','same_control','candidate'];rng.shuffle(variants)
  for v in variants:
   binary='control' if v=='same_control' else v
   q=subprocess.run([str(P/f'corpus-{binary}'),str(R/r['path']),r['mode'],'10','-'],capture_output=True,text=True,timeout=15,preexec_fn=limits)
   if q.returncode:raise RuntimeError(q.stderr)
   val=json.loads(q.stdout);val.update(sha256=r['sha256'],path=r['path'],mode=r['mode'],variant=v,block=block);rows.append(val)
 (E/'confirmation-samples.json').write_text(json.dumps(rows,indent=2));print('confirmation block',block+1,flush=True)
out=[]
for r in flags:
 diffs=[];aa=[]
 for b in range(15):
  g={v['variant']:statistics.mean(v['serialize_ns'][1:]) for v in rows if v['sha256']==r['sha256'] and v['mode']==r['mode'] and v['block']==b}
  diffs.append(100*(1-g['candidate']/math.sqrt(g['control']*g['same_control'])));aa.append(100*(1-g['same_control']/g['control']))
 def stats(ds):
  boot=sorted(statistics.median(rng.choices(ds,k=15)) for _ in range(10000));return {'median_reduction_percent':statistics.median(ds),'bootstrap95':[boot[249],boot[9749]],'bootstrap98_75':[boot[62],boot[9937]]}
 out.append({'path':r['path'],'mode':r['mode'],'candidate_vs_two_controls':stats(diffs),'identical_control_pair':stats(aa)})
(E/'confirmation-summary.json').write_text(json.dumps(out,indent=2));print(json.dumps(out,indent=2))
