import pathlib,json,subprocess,resource,random,os,time,sys,hashlib
ROOT=pathlib.Path('/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf')
E=pathlib.Path(os.environ.get('OXIDIZE_CORPUS_EVIDENCE',str(ROOT/'docs/reports/2026-10-26-issue-822-evidence/corpus')))
P=pathlib.Path('/tmp/oxidize-perf822-fixed-corpus')
os.sched_setaffinity(0,{0})
def limits():
 resource.setrlimit(resource.RLIMIT_AS,(2147483648,2147483648))
 resource.setrlimit(resource.RLIMIT_CORE,(0,0))
def run(row,variant,mode,iters,out):
 start=time.monotonic()
 try:
  result=subprocess.run([str(P/f'corpus-{variant}'),str(ROOT/row['path']),mode,str(iters),out],capture_output=True,text=True,timeout=15,preexec_fn=limits)
  value=json.loads(result.stdout.strip().splitlines()[-1]) if result.returncode==0 else {}
  value.update(status='ok' if result.returncode==0 else 'error',returncode=result.returncode,stderr=result.stderr[-2000:])
 except subprocess.TimeoutExpired: value={'status':'timeout'}
 except Exception as e: value={'status':'harness_error','error':str(e)}
 value.update(sha256=row['sha256'],path=row['path'],tiers=row['tiers'],variant=variant,mode=mode,process_wall_seconds=time.monotonic()-start)
 return value
phase=sys.argv[1];rng=random.Random(82207)
rows=json.loads((E/('unique-population.json' if phase=='full' else 'stratified-sample.json')).read_text());rng.shuffle(rows)
path=E/f'{phase}-results.jsonl'
identity={'runner_sha256':hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest(),'protocol_sha256':hashlib.sha256((E/'protocol.json').read_bytes()).hexdigest(),'cpu':0,'timeout_seconds':15,'address_space_bytes':2147483648,'blocks':1 if phase=='full' else 5,'iterations':1 if phase=='full' else 3,'binaries':{v:hashlib.sha256((P/f'corpus-{v}').read_bytes()).hexdigest() for v in ['control','candidate']},'population_hash':hashlib.sha256((E/('unique-population.json' if phase=='full' else 'stratified-sample.json')).read_bytes()).hexdigest()}
identity_path=E/f'{phase}-run-identity.json'
if identity_path.exists():
 assert json.loads(identity_path.read_text())==identity,'resume identity mismatch'
else:
 assert not path.exists(),'existing results have no run identity'
 identity_path.write_text(json.dumps(identity,indent=2))
completed=set()
if path.exists():
 for line in path.read_text().splitlines():
  v=json.loads(line);completed.add((v['sha256'],v['variant'],v['mode'],v['block']))
with path.open('a',buffering=1) as f:
 for block in range(1 if phase=='full' else 5):
  for i,row in enumerate(rows):
   modes=['rewrite'] if phase=='full' else ['rewrite','ttf','cjk'];rng.shuffle(modes)
   for mode in modes:
    order=['control','candidate'];rng.shuffle(order)
    for variant in order:
     key=(row['sha256'],variant,mode,block)
     if key in completed:continue
     out='-'
     if phase=='sample' and block==0:
      folder=P/'corpus-output';folder.mkdir(exist_ok=True)
      out=str(folder/f"{row['sha256']}-{mode}-{variant}.pdf")
     value=run(row,variant,mode,1 if phase=='full' else 3,out);value['block']=block
     f.write(json.dumps(value)+'\n')
   if (i+1)%100==0 or phase=='sample':print(phase,'block',block,'documents',i+1,'/',len(rows),flush=True)
print('complete',phase,flush=True)
