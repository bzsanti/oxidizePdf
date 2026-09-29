import concurrent.futures,subprocess,pathlib,json,time,sys
root=pathlib.Path('/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf/test-corpus/t3-stress')
binary,out=sys.argv[1:3]
files=sorted(root.rglob('*.pdf'))
def run(p):
 t=time.monotonic()
 try:
  r=subprocess.run([binary,str(p)],capture_output=True,timeout=60)
  v=json.loads(r.stdout.decode().splitlines()[-1]);v['exit']=r.returncode
 except Exception as e:v={'probe_error':str(e)}
 return {'path':str(p.relative_to(root)),'seconds':round(time.monotonic()-t,3),**v}
with open(out,'w') as f, concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
 for i,v in enumerate(pool.map(run,files),1):
  f.write(json.dumps(v)+'\n');f.flush()
  if i%100==0: print(i,'/',len(files),flush=True)
print('done',len(files),flush=True)
