import json,pathlib,subprocess,zlib,hashlib,collections
root=pathlib.Path('/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf/test-corpus');p=pathlib.Path('/tmp/issue-637-raw-streams')
out=pathlib.Path('/tmp/issue-637-independent-20260929');out.mkdir(exist_ok=True)
def check(b):
 d=zlib.decompressobj();v={'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()}
 try:
  r=d.decompress(b,268435457);v.update(zlib_eof=d.eof,zlib_bytes=len(r))
 except zlib.error as e:v['zlib_error']=str(e)
 d=zlib.decompressobj(-15)
 try:
  r=d.decompress(b[2:],268435457);v.update(raw_eof=d.eof,raw_bytes=len(r),raw_sha256=hashlib.sha256(r).hexdigest())
  if d.eof:
   v.update(checksum_expected=d.unused_data[:4].hex(),checksum_actual=f'{zlib.adler32(r):08x}',checksum_match=len(d.unused_data)>=4 and int.from_bytes(d.unused_data[:4],'big')==zlib.adler32(r))
 except zlib.error as e:v['raw_error']=str(e)
 return v
records=[]
for x in json.load(open(p/'records.json')):
 obj=x['object'].removeprefix('Reference(').removesuffix(')').replace(' ','')
 r=subprocess.run(['qpdf','--show-object='+obj,'--raw-stream-data',str(root/x['pdf'])],capture_output=True,timeout=30)
 b=(p/x['raw_file']).read_bytes();(out/x['raw_file']).write_bytes(r.stdout)
 records.append({**x,'original':check(b),'qpdf':check(r.stdout),'qpdf_exit':r.returncode,'qpdf_stderr':r.stderr.decode(errors='replace'),'equal':b==r.stdout,'qpdf_is_prefix':b.startswith(r.stdout)})
(out/'verification.json').write_text(json.dumps(records,indent=2))
print('streams',len(records),'qpdf equal',sum(x['equal'] for x in records),'qpdf prefix',sum(x['qpdf_is_prefix'] for x in records),'qpdf valid complete',sum(x['qpdf'].get('zlib_eof',False) for x in records))
print(collections.Counter(x['qpdf'].get('zlib_error','incomplete') for x in records))
