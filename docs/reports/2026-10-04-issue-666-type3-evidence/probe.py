import sys,json,hashlib,subprocess
from pathlib import Path
sys.path[:0]=['tools','target/issue668-readers']
from text_contracts.fixture_pdf import assemble,stream,cmap
import pymupdf
out=Path('target/issue666-type3');out.mkdir(exist_ok=True)
results=[]
cases=[('identity',[.001,0,0,.001,0,0]),('rotate',[0,.001,-.001,0,0,0]),('shear',[.001,.0005,.0003,.002,0,0]),('translate',[.001,0,0,.001,.1,.2]),('reflect',[-.001,0,0,.001,0,0])]
for name,matrix in cases:
 font=f'<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] /FontMatrix [{" ".join(map(str,matrix))}] /CharProcs << /A 6 0 R /B 7 0 R >> /Encoding << /Type /Encoding /Differences [65 /A /B] >> /FirstChar 65 /LastChar 66 /Widths [500 500] /Resources << >> >>'.encode()
 objects=[b'<< /Type /Catalog /Pages 2 0 R >>',b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>',font,stream(b'BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj ET'),stream(b'500 0 0 0 500 700 d1 0 0 400 600 re f'),stream(b'500 0 0 0 500 700 d1 0 0 400 600 re f')]
 raw=assemble(objects);out.joinpath(name+'.pdf').write_bytes(raw)
 doc=pymupdf.open(stream=raw,filetype='pdf')
 chars=[(chr(c[0]),list(c[2])) for span in doc[0].get_texttrace() for c in span['chars']]
 results.append({'name':name,'matrix':matrix,'chars':chars})
for name,content,expected in [
 ('state', b'BT /F1 10 Tf 50 Tz 2 Tc 100 700 Td (A) Tj 1 Tr (B) Tj ET',(106.,700.)),
 ('tj', b'BT /F1 10 Tf 50 Tz 2 Tc 100 700 Td [(A) 100] TJ 1 Tr (B) Tj ET',(105.5,700.)),
 ('tm', b'BT /F1 10 Tf 0 1 -1 0 100 700 Tm (A) Tj 1 Tr (B) Tj ET',(100.,710.))]:
 matrix=[.002,.001,.0005,.003,.1,.2]
 objects[3]=f'<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] /FontMatrix [{" ".join(map(str,matrix))}] /CharProcs << /A 6 0 R /B 7 0 R >> /Encoding << /Type /Encoding /Differences [65 /A /B] >> /FirstChar 65 /LastChar 66 /Widths [500 500] /Resources << >> /ToUnicode 8 0 R >>'.encode()
 objects[4]=stream(content)
 unicode=stream(cmap(2,'Type3Unicode','1 begincodespacerange <00> <FF> endcodespacerange 2 beginbfchar <41> <00660069> <42> <D83DDE00> endbfchar'))
 raw=assemble(objects+[unicode]);out.joinpath(name+'.pdf').write_bytes(raw)
 doc=pymupdf.open(stream=raw,filetype='pdf')
 chars=[(chr(c[0]),list(c[2])) for span in doc[0].get_texttrace() for c in span['chars']]
 actual=next(origin for ch,origin in chars if ch=='😀')
 assert abs(actual[0]-expected[0])<.0001 and abs((792-actual[1])-expected[1])<.0001,(name,chars)
 results.append({'name':name,'matrix':matrix,'chars':chars,'expected_pdf_pen':expected})
for result in results:
 path=out/(result['name']+'.pdf')
 result['sha256']=hashlib.sha256(path.read_bytes()).hexdigest()
 check=subprocess.run(['qpdf','--check',str(path)],capture_output=True,text=True)
 assert check.returncode==0,check.stdout+check.stderr
 doc=pymupdf.open(path)
 result['raster_sha256']=hashlib.sha256(doc[0].get_pixmap().samples).hexdigest()
assert len({r['raster_sha256'] for r in results[:5]})==5
print(json.dumps({'pymupdf':pymupdf.VersionBind,'mupdf':pymupdf.VersionFitz,'qpdf_checked':len(results),'results':results},indent=2))
