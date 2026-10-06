#!/usr/bin/env python3
"""#666 E08/E09: independent Differences boundaries/precedence across simple font types."""
import argparse,hashlib,json
from pathlib import Path
from text_contracts.fixture_pdf import assemble,cmap,stream
CASES={'reset':('[65 /B 67 /A]',['41','42','43'],['B','B','A']),
       'boundaries':('[0 /A 255 /B]',['00','FF'],['A','B']),
       'continuation':('[254 /A /B]',['FE','FF'],['A','B']),
       'last-entry':('[65 /A 65 /B]',['41'],['B']),
       'unicode':('[0 /A 255 /B]',['00','FF'],['fi','😀'])}

def generate(output):
    root=Path(__file__).resolve().parents[1]/'oxidize-pdf-core/tests/fixtures/text_contracts/symbolic'
    raw=(root/'unicode-full.ttf').read_bytes();reference=json.loads((root/'provenance.json').read_text())
    assert hashlib.sha256(raw).hexdigest()==reference['sha256']['unicode-full.ttf']
    output.mkdir(parents=True,exist_ok=True);cases=[]
    for kind in ['Type1','TrueType','Type3']:
        for mode,(differences,codes,texts) in CASES.items():
            name='ContractTTUnicode' if kind=='TrueType' else 'Helvetica' if kind=='Type1' else 'ContractType3'
            extras='/FontBBox [0 0 500 700] /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /A 10 0 R /B 11 0 R >> /Resources << >>' if kind=='Type3' else '/FontDescriptor 8 0 R'
            font=f'<< /Type /Font /Subtype /{kind} /BaseFont /{name} /Encoding 6 0 R /FirstChar 0 /LastChar 255 /Widths ['+'500 '*256+f'] {extras} '+('/ToUnicode 12 0 R ' if mode=='unicode' else '')+'>>'
            content='BT /F1 10 Tf 100 700 Td '+' '.join(f'{i%2} Tr <{code}> Tj' for i,code in enumerate(codes))+' ET'
            objects=[b'<< /Type /Catalog /Pages 2 0 R >>',b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
              b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>',font.encode(),stream(content.encode()),
              b'<< /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences 7 0 R >>',differences.encode(),
              (f'<< /Type /FontDescriptor /FontName /{name} /Flags 32 /FontBBox [0 0 700 600] /ItalicAngle 0 /Ascent 600 /Descent 0 /CapHeight 600 /StemV 80 '+('/FontFile2 9 0 R ' if kind=='TrueType' else '')+'>>').encode(),
              stream(raw,f'/Length1 {len(raw)}') if kind=='TrueType' else b'null',
              stream(b'500 0 0 0 500 700 d1 0 0 400 600 re f'),stream(b'500 0 0 0 500 700 d1 0 0 300 600 re f'),
              stream(cmap(2,'DifferenceUnicode','1 begincodespacerange <00> <FF> endcodespacerange 2 beginbfchar <00> <00660069> <FF> <D83DDE00> endbfchar'))]
            data=assemble(objects);filename=f'{kind}-{mode}.pdf';(output/filename).write_bytes(data)
            origins=[{'text':text,'origin':[100+i*5,92]} for i,text in enumerate(texts)]
            case={'id':filename[:-4],'path':filename,'sha256':hashlib.sha256(data).hexdigest(),'expected_text':''.join(texts),
              'expectation_source':'ISO32000-1 9.6.6 Differences integer resets/name increments and ToUnicode precedence; explicit Widths500 at10pt; independent literal source codes and destinations.',
              'expected_source_origins':origins}
            if mode!='unicode':case['expected_trace_origins']=origins
            cases.append(case)
    (output/'readers.json').write_text(json.dumps({'schema_version':1,'issue':666,'cases':cases},ensure_ascii=False,indent=2)+'\n')
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('output',type=Path);generate(p.parse_args().output)
