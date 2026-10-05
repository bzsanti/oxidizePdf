from pathlib import Path
import json,subprocess,sys
import pymupdf
out=Path(sys.argv[1]);out.mkdir(exist_ok=True)
base=Path('/tmp/issue666-step5-cidchar.pdf')
results=[]
cases=['41','0041','000041','00000041','FFFF','010000','FFFFFF','01000000','E0000001']
for code in cases:
 encoding=f'/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo << /Registry (Contract) /Ordering (Test) /Supplement 0 >> def /CMapName /ContractMap1 def /CMapType 1 def /WMode 0 def 1 begincodespacerange <{code}> <{code}> endcodespacerange 1 begincidchar <{code}> 17 endcidchar endcmap CMapName currentdict /CMap defineresource pop end end'
 unicode=f'/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def /CMapName /ContractMap2 def /CMapType 2 def 1 begincodespacerange <{code}> <{code}> endcodespacerange 1 beginbfchar <{code}> <00660069> endbfchar endcmap CMapName currentdict /CMap defineresource pop end end'
 path=out/f'code-{code}.pdf'
 with pymupdf.open(base) as doc:
  doc.update_stream(9,encoding.encode());doc.update_stream(10,unicode.encode());doc.update_stream(5,f'BT /F1 12 Tf 100 700 Td <{code}> Tj ET'.encode())
  doc.save(path)
 pymupdf.TOOLS.mupdf_warnings(reset=True)
 with pymupdf.open(path) as doc:text=doc[0].get_text()
 poppler=subprocess.run(['pdftotext','-raw',str(path),'-'],capture_output=True,text=True)
 check=subprocess.run(['qpdf','--check',str(path)],capture_output=True,text=True)
 results.append({'code':code,'bytes':len(code)//2,'value':int(code,16),'expected':'fi','mupdf':text,'mupdf_warnings':pymupdf.TOOLS.mupdf_warnings(reset=True),'poppler':poppler.stdout,'poppler_stderr':poppler.stderr,'poppler_exit':poppler.returncode,'qpdf_exit':check.returncode})
(out/'results.json').write_text(json.dumps({'versions':pymupdf.version,'cases':results},indent=2,ensure_ascii=False)+'\n')
print(json.dumps(results,ensure_ascii=False))
