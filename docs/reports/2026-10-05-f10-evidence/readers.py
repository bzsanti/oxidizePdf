"""Independent glyph/outline/geometry/raster controls for F10."""
import hashlib, io, json, subprocess, sys
from pathlib import Path
sys.path[:0]=['target/issue666-fonttools','target/issue668-readers']
import fontTools,pymupdf
from fontTools.ttLib import TTFont
from fontTools.pens.recordingPen import RecordingPen
assert (fontTools.__version__,pymupdf.VersionBind,pymupdf.VersionFitz)==('4.60.1','1.26.5','1.26.10')
root=Path('oxidize-pdf-core/tests/fixtures/text_contracts/cid_truetype')
manifest=json.loads((root/'manifest.json').read_text()); results=[]; rasters={}
for variant in ['full','subset']:
    font=TTFont(io.BytesIO((root/(variant+'.ttf')).read_bytes()))
    assert font.getGlyphOrder()==['.notdef','space','A','B']+(['C'] if variant=='full' else [])
    for name,points,width in [('A',[(50,0),(350,0),(200,600)],400),('B',[(50,0),(650,0),(650,600),(50,600)],700)]:
        pen=RecordingPen();font.getGlyphSet()[name].draw(pen)
        # hmtx lsb=0 causes the glyph set to translate x by -50.
        assert pen.value==[('moveTo',((points[0][0]-50,points[0][1]),))]+[('lineTo',((x-50,y),)) for x,y in points[1:]]+[('closePath',())],pen.value
        assert font['hmtx'][name][0]==width
for case in manifest['cases']:
    path=root/case['path']; checked=subprocess.run(['qpdf','--check',str(path)],capture_output=True,text=True);assert checked.returncode==0,checked.stderr
    with pymupdf.open(path) as doc:
        page=doc[0]; chars=[c for span in page.get_texttrace() for c in span['chars']]; selected=[c for c in chars if c[1]>=0]
        assert [c[1] for c in selected]==case['gids'],(path,chars)
        for char,origin in zip(selected,case['reader_origins']):
            assert all(abs(a-b)<.0001 for a,b in zip(char[2],origin)),(path,chars,case['reader_origins'])
        text=''.join(chr(c[0]) for c in chars)
        if case['explicit_unicode'] or case['collection']=='Japan1':assert text==''.join(case['unicode']),(path,text)
        pix=page.get_pixmap(colorspace=pymupdf.csGRAY,alpha=False);assert min(pix.samples)<32
        digest=hashlib.sha256(pix.samples).hexdigest();key=(case['vertical'],case['mode']=='remapped');rasters.setdefault(key,set()).add(digest)
        results.append({'path':case['path'],'gids':[c[1] for c in selected],'origins':[list(c[2]) for c in selected],'observed_text':text,'raster_sha256':digest,'qpdf':'pass'})
assert all(len(v)==1 for v in rasters.values()),rasters
assert len({next(iter(v)) for v in rasters.values()})==4,rasters
Path('docs/reports/2026-10-05-f10-evidence/readers.json').write_text(json.dumps({'versions':[fontTools.__version__,pymupdf.VersionBind,pymupdf.VersionFitz],'cases':results},indent=2)+'\n')
print('40 PDFs: glyphs, geometry, nonblank discriminating raster and structure pass; two original programs/outline controls pass.')
