import pathlib,json,subprocess,zlib
root=pathlib.Path('/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf/test-corpus/t3-stress');out=[]
for p in sorted(pathlib.Path('/tmp/issue-637-details-20260929/candidate').rglob('pages.json')):
 pages=json.loads(p.read_text())
 if any(not any(s['error'] for s in x.get('streams',[])) for x in pages):
  name=str(p.parent.relative_to('/tmp/issue-637-details-20260929/candidate'))+'.pdf';r=subprocess.run(['qpdf','--json','--json-key=qpdf',str(root/name)],capture_output=True,timeout=30);j=json.loads(r.stdout)['qpdf'][1];forms=[]
  for k,v in j.items():
   d=v.get('stream',{}).get('dict',{})
   if d.get('/Subtype')=='/Form' and d.get('/Filter')=='/FlateDecode':
    obj=k.removeprefix('obj:').removesuffix(' R').replace(' ',',');rr=subprocess.run(['qpdf','--show-object='+obj,'--raw-stream-data',str(root/name)],capture_output=True,timeout=30)
    try:dd=zlib.decompressobj();dd.decompress(rr.stdout);error=None if dd.eof else 'incomplete'
    except zlib.error as e:error=str(e)
    forms.append({'object':k,'length':len(rr.stdout),'dict':d,'error':error})
  out.append({'pdf':name,'forms':forms,'page_objects':{k:v for k,v in j.items() if isinstance(v.get('value'),dict) and v['value'].get('/Type')=='/Page'}})
pathlib.Path('/tmp/issue-637-forms-20260929.json').write_text(json.dumps(out,indent=2));print('documents',len(out),'with empty flate forms',sum(any(f['length']==0 for f in x['forms']) for x in out))
