from pathlib import Path
import json, subprocess, pymupdf
root=Path('/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf')
rows=[l.split('\t') for l in (root/'oxidize-pdf-core/tests/fixtures/text_contracts/cjk/samples.tsv').read_text().splitlines()[1:]]
results=[]
for i,r in enumerate(rows):
 p=Path(f'/tmp/issue666-step8-pdfs/case-{i:03}.pdf')
 expected=''.join(chr(int(c,16)) for c in r[7].split())+'\n'
 pymupdf.TOOLS.mupdf_warnings(reset=True)
 with pymupdf.open(p) as d: actual=d[0].get_text()
 warnings=pymupdf.TOOLS.mupdf_warnings(reset=True)
 q=subprocess.run(['qpdf','--check',str(p)],capture_output=True,text=True)
 pop=subprocess.run(['pdftotext','-enc','UTF-8','-raw',str(p),'-'],capture_output=True,text=True)
 results.append(dict(index=i,cmap=r[2],category=r[4],code=r[5],cid=int(r[6]),expected=expected,actual=actual,warnings=warnings,qpdf=q.returncode,poppler_text=pop.stdout,poppler_stderr=pop.stderr,poppler_exit=pop.returncode))
Path('/tmp/issue666-step8-readers.json').write_text(json.dumps({'pymupdf':pymupdf.VersionBind,'mupdf':pymupdf.VersionFitz,'cases':results},ensure_ascii=False,indent=2)+'\n')
print('cases',len(results),'mupdf_match',sum(r['expected']==r['actual'] for r in results),'qpdf_ok',sum(r['qpdf']==0 for r in results),'mupdf_warnings',sum(bool(r['warnings']) for r in results),'poppler_match',sum(r['poppler_text']==r['expected']+'\f' for r in results))
