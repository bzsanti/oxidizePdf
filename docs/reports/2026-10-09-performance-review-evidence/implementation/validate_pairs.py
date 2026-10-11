import pathlib,subprocess,hashlib,json,sys,xml.etree.ElementTree as ET
p=pathlib.Path(__file__).resolve().parent;label=sys.argv[1];rows=[]
for control in sorted(p.glob(label+'-control-*.out')):
 candidate=p/control.name.replace('-control-','-candidate-'); row={'case':control.name,'files':[]}
 if not control.read_bytes().startswith(b'%PDF'):
  assert control.read_bytes()==candidate.read_bytes();row['exact_layout']=True;rows.append(row);continue
 for f in [control,candidate]:
  q=subprocess.run(['qpdf','--check',str(f)],text=True,capture_output=True);assert q.returncode==0,(f,q.stdout,q.stderr)
  t=subprocess.run(['pdftotext','-bbox',str(f),'-'],text=True,capture_output=True);assert t.returncode==0,(f,t.stderr)
  root=ET.fromstring(t.stdout); body=next(x for x in root.iter() if x.tag.endswith('}body'))
  geometry=ET.tostring(body)
  if t.stderr:
   assert all(line=='no word list' for line in t.stderr.splitlines()),(f,t.stderr)
   assert not any(x.tag.endswith('}word') for x in body.iter()),(f,t.stderr)
  directory=p/(f.stem+'-raster');directory.mkdir(exist_ok=True)
  r=subprocess.run(['pdftoppm','-gray','-r','72',str(f),str(directory/'page')],capture_output=True,text=True);assert r.returncode==0 and not r.stderr,(f,r.stderr)
  rasters=[hashlib.sha256(x.read_bytes()).hexdigest() for x in sorted(directory.glob('page-*.pgm'))]
  assert rasters
  if t.stderr: assert len(t.stderr.splitlines())==len(rasters),(f,t.stderr)
  row['files'].append({'name':f.name,'qpdf_exit':q.returncode,'geometry_sha256':hashlib.sha256(geometry).hexdigest(),'page_rasters':rasters})
 assert row['files'][0]['geometry_sha256']==row['files'][1]['geometry_sha256'],row['case']
 assert row['files'][0]['page_rasters']==row['files'][1]['page_rasters'],row['case']
 rows.append(row)
assert rows
(p/(label+'-external.json')).write_text(json.dumps(rows,indent=2));print('Validated pairs:',len(rows))
