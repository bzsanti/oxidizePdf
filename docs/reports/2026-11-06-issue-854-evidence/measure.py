import argparse,hashlib,json,os,random,statistics,subprocess,tempfile,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',action='append',required=True,help='label=absolute path');p.add_argument('--samples',type=int,default=7);p.add_argument('--output',required=True);a=p.parse_args()
root=Path(__file__).parent
bins=dict(item.split('=',1) for item in a.binary)
binary_hashes={label:hashlib.sha256(Path(path).read_bytes()).hexdigest() for label,path in bins.items()}
if len(set(binary_hashes.values())) != len(binary_hashes):
 raise ValueError('Comparison requires distinct binary hashes; verify baseline/candidate provenance')
cpu=min(os.sched_getaffinity(0))
cases={
 'generate_text':['generate','100','50','5','compressed','text'],
 'generate_mixed':['generate','100','50','5','compressed','mixed'],
 'generate_mixed_raw':['generate','100','50','5','raw','mixed'],
 'generate_small':['generate','1','10','100','compressed','text'],
 'extract_shared_font':['extract',str(root/'shared-font-256p.pdf'),'5'],
 'extract_generated':['extract',str(root/'invoice-100p.pdf'),'3'],
 'extract_real':['extract','/tmp/oxidize-perf854/oxidize-pdf-core/tests/fixtures/Cold_Email_Hacks.pdf','5'],
 'memory_vectors_32':['read-streams',str(root/'vectors-32p.pdf')],
 'memory_vectors_256':['read-streams',str(root/'vectors-256p.pdf')],
}
rows=[];rng=random.Random(854)
def run(label,case):
 with tempfile.NamedTemporaryFile(prefix='perf854-time-',mode='r+') as f:
  cmd=['/usr/bin/time','-f','{"max_rss_kb":%M,"user_s":%U,"sys_s":%S}','-o',f.name,'taskset','-c',str(cpu),bins[label],*cases[case]]
  r=subprocess.run(cmd,text=True,capture_output=True,check=True);f.seek(0)
  return {**json.loads(r.stdout),**json.loads(f.read()),'label':label,'case':case,'cpu':cpu}
for case in cases:
 for label in bins:run(label,case)
 for sample in range(a.samples):
  labels=list(bins);rng.shuffle(labels)
  for label in labels:
   row=run(label,case);row['sample']=sample;rows.append(row)
 Path(a.output).write_text(json.dumps({'binaries':bins,'binary_sha256':binary_hashes,'samples':rows},indent=2)+'\n')
 print(case,{label:{'median_ms':round(statistics.median(r['elapsed_ns'] for r in rows if r['case']==case and r['label']==label)/1e6,3),'median_rss_kb':statistics.median(r['max_rss_kb'] for r in rows if r['case']==case and r['label']==label)} for label in bins},flush=True)
