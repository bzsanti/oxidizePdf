#!/usr/bin/env python3
"""#666 F09: CID CFF charset/FD/HV selection, pinned FontTools, original outlines."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import fontTools
from fontTools.cffLib import CFFFontSet
from fontTools.ttLib import TTFont
from generate_text_composite_font_contracts import cff_font
from text_contracts.fixture_pdf import assemble, cmap, stream

def generate(out):
    if fontTools.__version__ != '4.60.1': raise ValueError('FontTools 4.60.1 required')
    out.mkdir(parents=True, exist_ok=True)
    cases=[]; hashes={}
    for collection in ['private','Japan1']:
        registry,ordering=('Contract','Synthetic') if collection=='private' else ('Adobe','Japan1')
        # Adobe-Japan1 CIDs34/35 are A/B (Adobe collection oracle, not product).
        cids=[17,29] if collection=='private' else [34,35]
        ros=f'/Registry ({registry}) /Ordering ({ordering}) /Supplement 0'
        for subset in [False,True]:
            for selection in [0,3]:
                raw,info=cff_font(subset,selection)
                if collection=='Japan1':
                    parsed=CFFFontSet();parsed.decompile(io.BytesIO(raw),None);top=parsed.topDictIndex[0]
                    top.ROS=('Adobe','Japan1',0)
                    old=list(top.charset);new=['.notdef','cid00034','cid00035']+([] if subset else ['cid00042'])
                    top.CharStrings.charStrings={b:top.CharStrings.charStrings[a] for a,b in zip(old,new)}
                    top.charset=new
                    saved=io.BytesIO();parsed.compile(saved,TTFont(recalcBBoxes=False));raw=saved.getvalue()
                stem=f'{collection}-fd{selection}-'+('subset' if subset else 'full')
                fontfile=stem+'.cff';(out/fontfile).write_bytes(raw);hashes[fontfile]=hashlib.sha256(raw).hexdigest()
                for vertical in [False,True]:
                    for explicit in [False,True]:
                        name=stem+('-v' if vertical else '-h')+('-unicode' if explicit else '-fallback')+'.pdf'
                        objects=[b'<< /Type /Catalog /Pages 2 0 R >>',b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>',
                          f'<< /Type /Font /Subtype /Type0 /BaseFont /F09 /Encoding /Identity-{"V" if vertical else "H"} /DescendantFonts [6 0 R] {"/ToUnicode 9 0 R" if explicit else ""} >>'.encode(),
                          stream(f'BT /F1 10 Tf 100 700 Td <{cids[0]:04X}> Tj 1 Tr <{cids[1]:04X}> Tj ET'.encode()),
                          f'<< /Type /Font /Subtype /CIDFontType0 /BaseFont /F09 /CIDSystemInfo << {ros} >> /FontDescriptor 7 0 R /DW 1000 /W [{cids[0]} [400] {cids[1]} [700]] /DW2 [880 -1000] /W2 [{cids[0]} [-1200 200 880] {cids[1]} [-900 350 880]] >>'.encode(),
                          b'<< /Type /FontDescriptor /FontName /F09 /Flags 4 /FontBBox [0 0 700 600] /ItalicAngle 0 /Ascent 600 /Descent 0 /CapHeight 600 /StemV 80 /FontFile3 8 0 R >>',stream(raw,'/Subtype /CIDFontType0C'),
                          stream(cmap(2,'F09Unicode',f'1 begincodespacerange <0000> <FFFF> endcodespacerange 2 beginbfchar <{cids[0]:04X}> <0058> <{cids[1]:04X}> <0059> endbfchar'))]
                        pdf=assemble(objects);(out/name).write_bytes(pdf);hashes[name]=hashlib.sha256(pdf).hexdigest()
                        cases.append({'path':name,'font':fontfile,'collection':collection,'subset':subset,'fdselect_format':selection,'vertical':vertical,'explicit_unicode':explicit,'cids':cids,'expected_unicode':['X','Y'] if explicit else (['A','B'] if collection=='Japan1' else [None,None]),'gids':[1,2],'fds':[0,1],'advances':[-1200,-900] if vertical else [400,700],'text_origins':[[100,700],[100,688]] if vertical else [[100,700],[104,700]],'reader_origins':[[98,100.8],[96.5,112.8]] if vertical else [[100,92],[104,92]]})
    (out/'manifest.json').write_text(json.dumps({'tool':'FontTools4.60.1','license':'Original fixtures, repository MIT license','unicode_reference':'Adobe-Japan1 CID34 U+0041 / CID35 U+0042; explicit ToUnicode X/Y; private collection unknown is a recovery policy','cases':cases,'sha256':hashes},indent=2)+'\n')
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('output',type=Path);generate(parser.parse_args().output)
