import subprocess,json,hashlib
from pathlib import Path
root=Path(__file__).resolve().parent/'results';records=[]
for mode in ['default','raw']:
 for pages in [1,10,100]:
  for page in sorted({1,pages}):
   reference=None
   for variant in ['control','tracking','copies','formatting','compression']:
    result=subprocess.run(['pdftoppm','-f',str(page),'-l',str(page),'-singlefile','-scale-to','1000','-gray',str(root/f'{variant}-{mode}-{pages}.pdf')],check=True,capture_output=True)
    assert not result.stderr,result.stderr
    digest=hashlib.sha256(result.stdout).hexdigest()
    if reference is None:reference=digest
    assert digest==reference,(mode,pages,page,variant)
    records.append(dict(mode=mode,pages=pages,page=page,variant=variant,sha256=digest))
(root/'rasters.json').write_text(json.dumps(records,indent=2)+'\n')
print('PASS',len(records),'rasters')
