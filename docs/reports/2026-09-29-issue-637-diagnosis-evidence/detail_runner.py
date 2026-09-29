import pathlib,subprocess,json,concurrent.futures,sys
root=pathlib.Path('/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf/test-corpus/t3-stress');out=pathlib.Path('/tmp/issue-637-details-20260929');out.mkdir(exist_ok=True)
mode=sys.argv[1]
lost=json.load(open('/tmp/issue-637-lost-20260929.json'))
def run(p):
 dest=out/mode/p.removesuffix('.pdf');sel='failed'
 if mode=='base':sel=','.join(str(x['page']) for x in json.load(open(out/'candidate'/p.removesuffix('.pdf')/'pages.json')))
 r=subprocess.run(['/tmp/issue-637-probe-'+mode+'-detail-20260929',str(root/p),str(dest),sel],capture_output=True,timeout=120)
 return {'path':p,'exit':r.returncode,'stdout':r.stdout.decode(errors='replace'),'stderr':r.stderr.decode(errors='replace')}
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
 result=list(pool.map(run,lost))
(out/(mode+'-runs.json')).write_text(json.dumps(result,indent=2));print('done',mode,len(result),'failures',sum(x['exit']!=0 for x in result))
