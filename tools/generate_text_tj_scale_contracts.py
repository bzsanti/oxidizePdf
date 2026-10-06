#!/usr/bin/env python3
"""#666/#672: original TJ/Tz source-origin fixtures with literal independent geometry."""
import argparse, hashlib, json
from pathlib import Path
from text_contracts.fixture_pdf import assemble, stream

CASES = [(50,-300,104),(50,0,102.5),(50,300,101),
         (100,-300,108),(100,0,105),(100,300,102),
         (200,-300,116),(200,0,110),(200,300,104)]

def generate(output):
    output.mkdir(parents=True,exist_ok=True)
    cases=[]
    for scale,kern,x in CASES:
        name=f'scale-{scale}-kern-{kern}.pdf'
        objects=[b'<< /Type /Catalog /Pages 2 0 R >>',b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
          b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>',
          b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /FirstChar 65 /LastChar 66 /Widths [500 500] >>',
          stream(f'BT /F1 10 Tf {scale} Tz 100 700 Td [(A){kern}] TJ 1 Tr (B) Tj ET'.encode())]
        data=assemble(objects);(output/name).write_bytes(data)
        cases.append({'id':name[:-4],'path':name,'sha256':hashlib.sha256(data).hexdigest(),
          'expected_text':'AB','expectation_source':'ISO 32000-1 9.4.4: (width/1000*font_size - TJ/1000*font_size)*Tz/100; width500/font_size10. Literal origins table, not product calculation.',
          'expected_trace_origins':[{'text':'A','origin':[100,92]},{'text':'B','origin':[x,92]}]})
    (output/'readers.json').write_text(json.dumps({'schema_version':1,'issue':666,'cases':cases},indent=2)+'\n')

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('output',type=Path)
    generate(parser.parse_args().output)
