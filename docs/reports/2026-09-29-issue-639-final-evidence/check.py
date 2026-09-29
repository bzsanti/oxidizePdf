from pathlib import Path
import subprocess,json
root=Path('/tmp/issue639-final-validation'); results=[]
for p in sorted(root.glob('*.pdf')):
 if p.stem=='maxsize':continue
 checked=subprocess.run(['qpdf','--check',str(p)],capture_output=True,text=True)
 j=subprocess.run(['qpdf','--json',str(p)],capture_output=True,text=True)
 (root/(p.stem+'.json')).write_text(j.stdout)
 text=subprocess.run(['pdftotext',str(p),'-'],capture_output=True,text=True)
 r={'file':p.name,'qpdf_exit':checked.returncode,'qpdf_message':checked.stderr,'text':text.stdout.strip(),'text_stderr':text.stderr}
 if j.stdout:
  d=json.loads(j.stdout); r['pages']=len(d['pages']); objs=d['qpdf'][1]; trailer=objs['trailer']['value']; rootobj=objs['obj:'+trailer['/Root']]['value']; r['catalog']=rootobj; r['info']=objs.get('obj:'+trailer.get('/Info',''),{}).get('value'); r['page_parents']=[]; r['page_metadata']=[]
  for page in d['pages']:
   obj=objs['obj:'+page['object']]['value']; r['page_parents'].append(obj.get('/Parent'));r['page_metadata'].append(obj.get('/Metadata'))
 results.append(r)
(root/'checks.json').write_text(json.dumps(results,indent=2))
for r in results:
 print(json.dumps(r))
