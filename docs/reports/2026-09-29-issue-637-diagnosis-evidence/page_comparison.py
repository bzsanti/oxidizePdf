import pathlib,json,collections,subprocess,concurrent.futures,re,hashlib
root=pathlib.Path('/tmp/issue-637-details-20260929');corpus=pathlib.Path('/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf/test-corpus/t3-stress')
entries=[]
for p in sorted((root/'candidate').rglob('pages.json')):
 rel=p.relative_to(root/'candidate');b={x['page']:x for x in json.loads((root/'base'/rel).read_text())}
 for x in json.loads(p.read_text()):entries.append((str(rel.parent)+'.pdf',x,b[x['page']],root/'base'/rel.parent/f"page-{x['page']}.txt"))
def words(t):return [w.lower() for w in re.findall(r'[^\W\d_]+',t) if len(w)>=4]
def run(e):
 name,c,b,text=e;page=c['page'];raw=text.read_bytes() if text.exists() else b'';bt=raw.decode();out={'pdf':name,'page':page,'base_text_bytes':len(raw),'base_text_sha256':hashlib.sha256(raw).hexdigest(),'base_words':len(words(bt)),'candidate_error':c['error']}
 try:
  r=subprocess.run(['pdftotext','-f',str(page+1),'-l',str(page+1),str(corpus/name),'-'],capture_output=True,timeout=30)
  pt=r.stdout.decode(errors='replace');pop=root/'poppler'/pathlib.Path(name).with_suffix('')/f'page-{page}.txt';pop.parent.mkdir(parents=True,exist_ok=True);pop.write_text(pt)
  bw,pw=collections.Counter(words(bt)),collections.Counter(words(pt));out.update(poppler_exit=r.returncode,poppler_words=sum(pw.values()),common_words=sum((bw&pw).values()),poppler_stderr=r.stderr.decode(errors='replace'),poppler_text_sha256=hashlib.sha256(r.stdout).hexdigest())
 except Exception as e:out['poppler_error']=str(e)
 return out
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:rs=list(pool.map(run,entries))
(root/'page-comparison.json').write_text(json.dumps(rs,indent=2))
print('pages',len(rs),'base bytes',sum(x['base_text_bytes'] for x in rs),'base words',sum(x['base_words'] for x in rs),'common words on poppler success',sum(x.get('common_words',0) for x in rs if x.get('poppler_exit')==0),'poppler successes',sum(x.get('poppler_exit')==0 for x in rs),'pages with shared words',sum(x.get('common_words',0)>0 and x.get('poppler_exit')==0 for x in rs))
