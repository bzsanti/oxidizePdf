import pathlib,json,subprocess,hashlib,collections,sys,xml.etree.ElementTree as ET
P=pathlib.Path('/tmp/oxidize-perf822');E=pathlib.Path('/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf/docs/reports/2026-10-26-issue-822-evidence/corpus')
def snapshot(path):
 check=subprocess.run(['qpdf','--check',str(path)],capture_output=True,text=True,errors='replace',timeout=30)
 proc=subprocess.run(['qpdf','--json','--json-stream-data=inline',str(path)],capture_output=True,text=True,timeout=30)
 if proc.returncode not in (0,3):return {'qpdf_check':check.returncode,'error':proc.stderr[-1000:]}
 doc=json.loads(proc.stdout);objs=doc['qpdf'][1]
 streams={k:hashlib.sha256(v['stream'].get('data','').encode()).hexdigest() for k,v in objs.items() if 'stream' in v and v['stream']['dict'].get('/Type')!='/Metadata'}
 pages=[[streams.get('obj:'+x,'missing:'+x) for x in page['contents']] for page in doc['pages']]
 widths=sorted(json.dumps({k:v['value'][k] for k in ['/W','DW','/DW'] if k in v['value']},sort_keys=True) for v in objs.values() if isinstance(v.get('value'),dict) and '/W' in v['value'])
 result={'qpdf_check':check.returncode,'qpdf_warnings':check.stderr[-1000:],'streams':sorted(streams.values()),'page_contents':pages,'widths':widths}
 try:
  text=subprocess.run(['pdftotext','-bbox',str(path),'-'],capture_output=True,timeout=30)
  if text.returncode:result['text_error']=text.stderr.decode(errors='replace')[-1000:]
  else:
   xml=ET.fromstring(text.stdout);ps=[n for n in xml.iter() if n.tag.endswith('}page')]
   canonical=[(p.attrib,[(w.text,w.attrib) for w in p.iter() if w.tag.endswith('}word')]) for p in ps]
   result['bbox_hash']=hashlib.sha256(json.dumps(canonical,sort_keys=True).encode()).hexdigest();result['page_count']=len(ps)
 except Exception as ex:result['text_error']=str(ex)
 return result
existing=json.loads((E/'external-validation.json').read_text()) if '--retry-errors' in sys.argv else []
rows=[x for x in existing if not x.get('validation_error')]
identities=sorted({f.name.removesuffix('-control.pdf').removesuffix('-candidate.pdf') for f in (P/'corpus-output').glob('*.pdf')})
if existing:identities=[x['file'].removesuffix('-control.pdf') for x in existing if x.get('validation_error')]
files=identities
for i,key in enumerate(identities):
 control=P/'corpus-output'/f'{key}-control.pdf'
 candidate=P/'corpus-output'/f'{key}-candidate.pdf'
 if not control.exists() or not candidate.exists():
  rows.append({'file':key,'status':'missing_control' if not control.exists() else 'missing_candidate'});continue
 try:
  a=snapshot(control);b=snapshot(candidate)
  row={'file':control.name,'control_qpdf':a['qpdf_check'],'candidate_qpdf':b['qpdf_check'],'streams_equal':a.get('streams')==b.get('streams') if 'streams' in a and 'streams' in b else None,'page_contents_equal':a.get('page_contents')==b.get('page_contents') if 'page_contents' in a and 'page_contents' in b else None,'widths_equal':a.get('widths')==b.get('widths') if 'widths' in a and 'widths' in b else None,'bbox_equal':a.get('bbox_hash')==b.get('bbox_hash') if 'bbox_hash' in a and 'bbox_hash' in b else None,'control_pages':a.get('page_count'),'candidate_pages':b.get('page_count'),'control_error':a.get('error',a.get('text_error')),'candidate_error':b.get('error',b.get('text_error')),'control_warnings':a.get('qpdf_warnings'),'candidate_warnings':b.get('qpdf_warnings')}
 except Exception as ex:row={'file':control.name,'validation_error':str(ex)}
 rows.append(row)
 (E/'external-validation.json').write_text(json.dumps(rows,indent=2))
 if (i+1)%25==0:print('validated pairs',i+1,'/',len(files),flush=True)
(E/'external-validation.json').write_text(json.dumps(rows,indent=2))
print('complete',len(rows),flush=True)
