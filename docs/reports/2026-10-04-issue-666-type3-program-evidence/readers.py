import sys,json,hashlib,subprocess
from pathlib import Path
sys.path[:0]=['tools','target/issue668-readers']
from text_contracts.fixture_pdf import assemble,stream,cmap
import pymupdf
assert (pymupdf.VersionBind, pymupdf.VersionFitz) == ('1.26.5', '1.26.10'), 'unvalidated external reader version'
out=Path('target/f08-fixed-readers');out.mkdir(exist_ok=True)
results=[]
for name,width,program,x in [('d0',500,'500 0 d0 0 0 400 600 re f',105),('d1',500,'500 0 0 0 500 700 d1 0 0 400 600 re f',105),('mismatch',500,'900 0 d0 0 0 400 600 re f',105),('zero',0,'0 0 d0',100),('negative',-250,'-250 0 d0',97.5),('fractional',125.5,'125.5 0 d0',101.255),('malformed',500,'500 d1',105)]:
 font=f'<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /A 6 0 R /B 7 0 R >> /Encoding << /Differences [65 /A /B] >> /FirstChar 65 /LastChar 66 /Widths [{width} 500] /Resources << >> /ToUnicode 8 0 R >>'.encode()
 raw=assemble([b'<< /Type /Catalog /Pages 2 0 R >>',b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>',font,stream(b'BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj ET'),stream(program.encode()),stream(b'500 0 d0 0 0 400 600 re f'),stream(cmap(2,'ProgramUnicode','1 begincodespacerange <00> <FF> endcodespacerange 2 beginbfchar <41> <00660069> <42> <0042> endbfchar'))])
 path=out/(name+'.pdf');path.write_bytes(raw)
 doc=pymupdf.open(path); chars=[(chr(c[0]),list(c[2])) for span in doc[0].get_texttrace() for c in span['chars']]
 origin=next(o for c,o in chars if c=='B'); matches=abs(origin[0]-x)<.0001 and abs(origin[1]-92)<.0001
 # MuPDF 1.26.10 truncates fractional Type3 hmtx widths; keep the PDF oracle.
 external_x = 101.25 if name == 'fractional' else x
 assert abs(origin[0]-external_x)<.0001 and abs(origin[1]-92)<.0001,(name,chars,external_x)
 check=subprocess.run(['qpdf','--check',str(path)],capture_output=True,text=True);assert check.returncode==0
 results.append({'name':name,'expected_pdf_x':x,'mupdf_matches':matches,'expected_mupdf_x':external_x,'classification':'known MuPDF integer hmtx truncation' if name == 'fractional' else 'matches PDF oracle','width':width,'program':program,'chars':chars,'sha256':hashlib.sha256(raw).hexdigest(),'qpdf':'structural check passed; does not validate CharProc semantics'})
Path(sys.argv[1] if len(sys.argv)>1 else 'docs/reports/2026-10-04-f08-fixes-evidence/readers.json').write_text(json.dumps({'mupdf':pymupdf.VersionFitz,'pymupdf':pymupdf.VersionBind,'results':results},indent=2)+'\n')
