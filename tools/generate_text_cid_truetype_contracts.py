#!/usr/bin/env python3
"""#666 F10: original CID TrueType selection matrix; FontTools 4.60.1."""
import argparse
import hashlib
import json
from pathlib import Path

import fontTools
from generate_text_symbolic_contracts import make_font
from text_contracts.fixture_pdf import assemble, cmap, stream


def make_pdf(raw, name, mode, vertical, unicode, collection):
    japan = collection == 'Japan1'
    ros = '/Registry (Adobe) /Ordering (Japan1) /Supplement 0' if japan else '/Registry (Contract) /Ordering (Synthetic) /Supplement 0'
    cids = [2, 3] if mode == 'identity' else ([34, 35] if japan else [17, 29])
    remap = mode == 'remapped'
    selected = cids[::-1] if remap else cids
    codes = ['41', '42'] if remap else [f'{cid:04X}' for cid in cids]
    gidmap = bytearray((max(cids) + 1) * 2)
    for cid, gid in zip(cids, [2, 3]):
        gidmap[2*cid:2*cid+2] = gid.to_bytes(2, 'big')
    wmode = int(vertical)
    encoding = '12 0 R' if remap else ('/Identity-V' if vertical else '/Identity-H')
    unicode_entry = '/ToUnicode 9 0 R' if unicode else ''
    map_entry = '/Identity' if mode == 'identity' else '11 0 R'
    spaces = '<00> <FF>' if remap else '<0000> <FFFF>'
    objects = [b'<< /Type /Catalog /Pages 2 0 R >>',
        b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
        b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>',
        f'<< /Type /Font /Subtype /Type0 /BaseFont /{name} /Encoding {encoding} /DescendantFonts [6 0 R] {unicode_entry} >>'.encode(),
        stream(f'BT /F1 10 Tf 100 700 Td <{codes[0]}> Tj 1 Tr <{codes[1]}> Tj ET'.encode()),
        (f'<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{name} /CIDSystemInfo << {ros} >> /FontDescriptor 7 0 R '
         f'/CIDToGIDMap {map_entry} /DW 1000 /W [{cids[0]} [450] {cids[1]} [750]] '
         f'/DW2 [880 -1500] /W2 [{cids[0]} [-1200 225 880] {cids[1]} [-900 375 880]] >>').encode(),
        (f'<< /Type /FontDescriptor /FontName /{name} /Flags 4 /FontBBox [0 0 700 600] /ItalicAngle 0 '
         '/Ascent 600 /Descent 0 /CapHeight 600 /StemV 80 /FontFile2 8 0 R >>').encode(),
        stream(raw, f'/Length1 {len(raw)}'),
        stream(cmap(2, 'CIDTrueTypeUnicode', f'1 begincodespacerange {spaces} endcodespacerange 2 beginbfchar <{codes[0]}> <00660069> <{codes[1]}> <D83DDE00> endbfchar')),
        b'null', stream(bytes(gidmap)),
        stream(cmap(1, 'CIDTrueTypeRemap', f'1 begincodespacerange <00> <FF> endcodespacerange 2 begincidchar <41> {cids[1]} <42> {cids[0]} endcidchar', ros).replace(b'/WMode 0', f'/WMode {wmode}'.encode()),
               f'/Type /CMap /CMapName /CIDTrueTypeRemap /CIDSystemInfo << {ros} >> /WMode {wmode}')]
    widths = [750, 450] if remap else [450, 750]
    advances = ([-900, -1200] if remap else [-1200, -900]) if vertical else widths
    pens = [[100, 700], [100, 700 + advances[0]/100]] if vertical else [[100, 700], [100 + advances[0]/100, 700]]
    origins = [[x-width/200, 792-y+8.8] if vertical else [x,792-y] for (x,y),width in zip(pens,widths)]
    expected = ['fi', '😀'] if unicode else ((['B','A'] if remap else ['A','B']) if japan else [None,None])
    return assemble(objects), {'mode':mode,'vertical':vertical,'explicit_unicode':unicode,'collection':collection,
        'codes':codes,'cids':selected,'gids':[3,2] if remap else [2,3],'unicode':expected,'advances':advances,'pens':pens,'reader_origins':origins}


def generate(output):
    assert fontTools.__version__ == '4.60.1'
    output.mkdir(parents=True,exist_ok=True)
    hashes = {}; cases = []
    for subset in [False,True]:
        raw, info = make_font('unicode', subset)
        variant = 'subset' if subset else 'full'
        filename = f'{variant}.ttf'
        (output/filename).write_bytes(raw)
        hashes[filename] = hashlib.sha256(raw).hexdigest()
        for collection, modes in [('private',['identity','stream','remapped']),('Japan1',['stream','remapped'])]:
            for mode in modes:
                for vertical in [False,True]:
                    for unicode in [False,True]:
                        data, case = make_pdf(raw, info['name'], mode, vertical, unicode, collection)
                        path = f'{variant}-{collection}-{mode}-{"v" if vertical else "h"}-{"unicode" if unicode else "fallback"}.pdf'
                        (output/path).write_bytes(data)
                        hashes[path] = hashlib.sha256(data).hexdigest()
                        cases.append(dict(case,path=path,font=filename,subset=subset))
    (output/'manifest.json').write_text(json.dumps({'issue':666,'tool':'FontTools4.60.1','license':'Original project fixtures; repository MIT license','cases':cases,'sha256':hashes},indent=2)+'\n')


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output',type=Path)
    generate(parser.parse_args().output)
