"""F09 independent FontTools and MuPDF glyph/FD/geometry/raster controls."""
import hashlib,io,json,subprocess,sys
from pathlib import Path
sys.path[:0]=['target/issue666-fonttools','target/issue668-readers']
import fontTools,pymupdf
from fontTools.cffLib import CFFFontSet
from fontTools.pens.recordingPen import RecordingPen
assert (fontTools.__version__,pymupdf.VersionBind,pymupdf.VersionFitz)==('4.60.1','1.26.5','1.26.10')
root=Path('oxidize-pdf-core/tests/fixtures/text_contracts/cff_selection')
manifest=json.loads((root/'manifest.json').read_text());results=[];rasters={}
for case in manifest['cases']:
    raw=(root/case['font']).read_bytes();parsed=CFFFontSet();parsed.decompile(io.BytesIO(raw),None);top=parsed.topDictIndex[0]
    assert top.charset[1:3]==[f'cid{cid:05}' for cid in case['cids']]
    assert top.FDSelect.gidArray[:3]==[0,0,1]
    assert len(top.FDArray)==2
    assert top.ROS==(('Adobe','Japan1',0) if case['collection']=='Japan1' else ('Contract','Synthetic',0))
    for cid,width in zip(case['cids'],[400,700]):
        char=top.CharStrings[f'cid{cid:05}'];pen=RecordingPen();char.draw(pen)
        assert char.width==width
        assert pen.value==[('moveTo',((50,0),)),('lineTo',((width-50,0),)),('lineTo',((width/2,600),)),('closePath',())],pen.value
    path=root/case['path'];qpdf=subprocess.run(['qpdf','--check',str(path)],capture_output=True,text=True);assert qpdf.returncode==0,qpdf.stderr
    with pymupdf.open(path) as doc:
        page=doc[0];chars=[c for span in page.get_texttrace() for c in span['chars']]
        # Raw CID-keyed CFF in FreeType uses CID-valued glyph selectors.
        # Actual CharStrings indexes are independently checked by FontTools/Rust.
        assert [c[1] for c in chars]==case['cids'],(path,chars)
        for char,origin in zip(chars,case['reader_origins']):
            assert all(abs(a-b)<.0001 for a,b in zip(char[2],origin)),(path,chars,case['reader_origins'])
        text=''.join(chr(c[0]) for c in chars)
        if case['explicit_unicode'] or case['collection']=='Japan1':
            assert text==''.join(case['expected_unicode']),(path,text)
        pix=page.get_pixmap(colorspace=pymupdf.csGRAY,alpha=False)
        digest=hashlib.sha256(pix.samples).hexdigest()
        assert min(pix.samples)<32,'blank raster'
        key=(case['collection'],case['vertical'])
        rasters.setdefault(key,set()).add(digest)
    results.append({'case':case['path'],'mupdf_cid_selectors':[c[1] for c in chars],'charstring_gids':[1,2],'origins':[list(c[2]) for c in chars],'text_observed':text,'text_oracle':'explicit/Adobe collection' if case['explicit_unicode'] or case['collection']=='Japan1' else 'unknown; observation only','raster_sha256':digest,'qpdf':'structure only'})
assert all(len(values)==1 for values in rasters.values()),rasters
assert next(iter(rasters[('private',False)]))!=next(iter(rasters[('private',True)]))
Path('docs/reports/2026-10-04-f09-evidence/readers.json').write_text(json.dumps({'versions':[fontTools.__version__,pymupdf.VersionBind,pymupdf.VersionFitz],'results':results},indent=2)+'\n')
print('32 independent CFF glyph/FD/position/raster cases passed.')
