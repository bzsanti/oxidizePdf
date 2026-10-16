from pathlib import Path
import subprocess, json, hashlib
root=Path('/tmp/oxidize-issue757/oxidize-pdf-core/tests/fixtures')
evidence=Path('/tmp/oxidize-issue757-evidence')
results=[]
for name in ['issue_498_actual_text_interop.pdf','issue_272_higgs_arxiv_1207_7214.pdf','issue_272_boe_sumario_2025_01_15.pdf','Cold_Email_Hacks.pdf','synthetic_slide_deck.pdf','incremental_metadata/compressed.pdf']:
 row={'file':name,'runs':{}}
 for variant in ['baseline','candidate']:
  out=evidence/(variant+'-'+Path(name).stem+'.txt')
  run=subprocess.run(['prlimit','--as=2147483648','--core=0','/usr/bin/time','-f','maxrss_kb=%M elapsed=%e',str(evidence/(variant+'-probe')),str(root/name),str(out)],capture_output=True,timeout=90)
  row['runs'][variant]={'code':run.returncode,'stderr':run.stderr.decode(),'sha256':hashlib.sha256(out.read_bytes()).hexdigest() if out.exists() else None,'bytes':out.stat().st_size if out.exists() else None}
 row['identical']=row['runs']['baseline']['code']==row['runs']['candidate']['code']==0 and row['runs']['baseline']['sha256']==row['runs']['candidate']['sha256']
 results.append(row)
(evidence/'controls.json').write_text(json.dumps(results,indent=2))
print([(row['file'],row['identical'],row['runs']['candidate']['bytes']) for row in results])
assert all(row['identical'] for row in results)
