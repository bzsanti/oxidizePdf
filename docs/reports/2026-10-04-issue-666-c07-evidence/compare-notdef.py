"""Paired CMap experiment: replace only notdef with explicit CID mapping."""
from pathlib import Path
import sys,json,hashlib,subprocess
root=Path.cwd()
sys.path[:0]=[str(root/'target/issue668-readers'),str(root/'tools')]
import pymupdf
import generate_text_usecmap_contracts as gen
p=root/'docs/reports/2026-10-04-issue-666-c07-evidence'
scratch=root/'target/issue666-c07-controls';scratch.mkdir(exist_ok=True)
font=(root/'oxidize-pdf-core/tests/fixtures/text_contracts/fonts/ContractCID.cff').read_bytes()
results=[]
original=gen.map_object
for name in ['notdef-char','notdef-range']:
 for explicit in [False,True]:
  def mapped(kind,title,body,parent=''):
   if explicit and title=='ChildEncoding':
    # Replace the .notdef operator only; Unicode, widths and PDF program stay fixed.
    if name=='notdef-char':body=body.replace('beginnotdefchar','begincidchar').replace('endnotdefchar','endcidchar')
    else:body=body.replace('1 beginnotdefrange <00> <FF> 0 endnotdefrange','1 begincidchar <04> 0 endcidchar')
   return original(kind,title,body,parent)
  gen.map_object=mapped
  data=gen.make_pdf(font,name);file=scratch/f'{name}-{explicit}.pdf';file.write_bytes(data)
  pymupdf.TOOLS.reset_mupdf_warnings()
  doc=pymupdf.open(stream=data,filetype='pdf');page=doc[0]
  text=page.get_text()
  trace=[{'unicode':c[0],'gid':c[1],'origin':c[2]} for span in page.get_texttrace() for c in span['chars']]
  pix=page.get_pixmap()
  warnings=pymupdf.TOOLS.mupdf_warnings()
  pop=subprocess.run(['pdftotext','-raw','-enc','UTF-8',str(file),'-'],capture_output=True,text=True)
  qpdf=subprocess.run(['qpdf','--check',str(file)],capture_output=True,text=True)
  results.append({'case':name,'explicit_cid_control':explicit,'sha256':hashlib.sha256(data).hexdigest(),'mupdf':{'text':text,'trace':trace,'warnings':warnings,'raster_sha256':hashlib.sha256(pix.samples).hexdigest()},'poppler':{'text':pop.stdout,'stderr':pop.stderr,'exit':pop.returncode},'qpdf':{'exit':qpdf.returncode,'stderr':qpdf.stderr}})
(p/'notdef-reader-pairs.json').write_text(json.dumps({'versions':pymupdf.version,'cases':results},ensure_ascii=False,indent=2)+'\n')
print([(x['case'],x['explicit_cid_control'],x['mupdf']['text'],x['mupdf']['trace'],x['poppler']['text']) for x in results])
