#!/usr/bin/env python3
"""#666 Standard14 metrics/selection fixtures from unchanged Adobe AFM sources."""
import argparse, hashlib, json, re
from pathlib import Path
from text_contracts.fixture_pdf import assemble, cmap, stream
REVISION='0675784d24b28a55c607cad6b74596ce19ce333c'

def generate(output):
    afm=Path(__file__).resolve().parents[1]/'oxidize-pdf-core/tests/fixtures/text_contracts/standard14/afm'
    fonts=[]
    hashes={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(afm.iterdir()) if p.suffix=='.afm' or p.name=='LICENSE'}
    for path in sorted(afm.glob('*.afm')):
        source=path.read_text();name=re.search(r'^FontName (.+)$',source,re.M)[1]
        widths={int(c):int(w) for c,w in re.findall(r'^C (-?\d+) ; WX (\d+) ;',source,re.M)}
        codes,text=([65,105,87],'AiW') if name not in ['Symbol','ZapfDingbats'] else (([65,97,87],'ΑαΩ') if name=='Symbol' else ([33,34,35],'✁✂✃'))
        fonts.append({'name':name,'codes':codes,'text':text,'widths':[widths[c] for c in codes]})
    assert len(fonts)==14
    output.mkdir(parents=True,exist_ok=True);cases=[]
    def emit(name,used,shown,explicit=False):
        # Objects6.. are fonts; glyphs shown as separate runs with Tr boundaries.
        resources=' '.join(f'/F{i} {6+i} 0 R' for i in range(len(used)))
        objects=[b'<< /Type /Catalog /Pages 2 0 R >>',b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
          f'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << {resources} >> >> /Contents 5 0 R >>'.encode(),b'null']
        content=['BT 100 700 Td'];trace=[];x=100
        for index,(font_index,glyph_index) in enumerate(shown):
            f=used[font_index];code=f['codes'][glyph_index];char=f['text'][glyph_index]
            content.append(f'/F{font_index} 10 Tf {index%2} Tr <{code:02X}> Tj')
            trace.append({'text':char,'origin':[round(x,6),92]})
            x+=(391 if explicit else f['widths'][glyph_index])/100
        content.append('ET');objects.append(stream(' '.join(content).encode()))
        unicode_maps=[]
        for f in used:
            to_unicode=''
            if f['name']=='ZapfDingbats':
                # MuPDF's built-in aNN-to-Unicode interpretation differs from
                # Adobe AGL. This battery isolates AFM geometry; the existing
                # exhaustive encoding battery owns that decoding discrepancy.
                reference=6+len(used)+len(unicode_maps)
                body='1 begincodespacerange <00> <FF> endcodespacerange 3 beginbfchar '+' '.join(f'<{code:02X}> <{char.encode("utf-16-be").hex()}>' for code,char in zip(f['codes'],f['text']))+' endbfchar'
                unicode_maps.append(stream(cmap(2,'ZapfMetricUnicode',body)))
                to_unicode=f'/ToUnicode {reference} 0 R'
            encoding='' if f['name'] in ['Symbol','ZapfDingbats'] else '/Encoding /StandardEncoding'
            metric='/FirstChar 0 /LastChar 255 /Widths ['+'391 '*256+']' if explicit else ''
            objects.append(f"<< /Type /Font /Subtype /Type1 /BaseFont /{f['name']} {encoding} {metric} {to_unicode} >>".encode())
        objects.extend(unicode_maps)
        data=assemble(objects);filename=name+'.pdf';(output/filename).write_bytes(data)
        cases.append({'id':name,'path':filename,'sha256':hashlib.sha256(data).hexdigest(),'expected_text':''.join(t['text'] for t in trace),
          'expectation_source':f'Adobe AFM sources revision {REVISION}; font widths /100 at10pt; explicit Widths391 overrides AFM. Whitespace inference is not a normative metric claim.',
          'expected_trace_origins':trace})
    for font in fonts:
        for explicit in [False,True]:emit(font['name']+('-explicit' if explicit else '-implicit'),[font],[(0,0),(0,1),(0,2)],explicit)
    for first in fonts:
        for second in fonts:
            emit(first['name']+'-to-'+second['name'],[first,second],[(0,0),(1,0),(0,1)])
    (output/'readers.json').write_text(json.dumps({'schema_version':1,'issue':666,'cases':cases},ensure_ascii=False,indent=2)+'\n')
    (output/'provenance.json').write_text(json.dumps({'source_revision':REVISION,'source_url':f'https://github.com/tecnickcom/tc-font-core14-afms/tree/{REVISION}',
       'original_adobe_url':'https://www.adobe.com/devnet/font/pdfs/Core14_AFMs.zip (404 on 2026-10-03)',
       'license':'afm/LICENSE; unchanged Adobe sources with copyright retained','afm_sha256':hashes,'fonts':fonts},ensure_ascii=False,indent=2)+'\n')
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('output',type=Path);generate(parser.parse_args().output)
