"""Independent glyph, position and raster controls; Unicode guesses are observations."""
import hashlib
import json
import subprocess
import sys
from pathlib import Path
sys.path[:0] = ['tools', 'target/issue666-fonttools', 'target/issue668-readers']
import pymupdf
import fontTools
from generate_text_symbolic_contracts import make_font, make_pdf
assert (fontTools.__version__, pymupdf.VersionBind, pymupdf.VersionFitz) == ('4.60.1', '1.26.5', '1.26.10')
out = Path('target/f07-readers')
out.mkdir(exist_ok=True)
results, raster = [], {}
for profile in ['symbol', 'mac', 'unicode', 'ucs4']:
    symbolic = profile in ['symbol', 'mac']
    for subset in [False, True]:
        raw, info = make_font(profile, subset)
        # Existing frozen programs must be reproduced byte for byte.
        original = Path('oxidize-pdf-core/tests/fixtures/text_contracts/symbolic') / f"{profile}-{'subset' if subset else 'full'}.ttf"
        assert raw == original.read_bytes()
        for unicode in [False, True]:
            name = f'{profile}-{subset}-{unicode}'
            pdf = make_pdf(raw, info, unicode)
            path = out / (name + '.pdf')
            path.write_bytes(pdf)
            check = subprocess.run(['qpdf', '--check', str(path)], capture_output=True, text=True)
            assert check.returncode == 0, check.stderr
            with pymupdf.open(path) as doc:
                page = doc[0]
                chars = [c for span in page.get_texttrace() for c in span['chars']]
                # Multi-scalar mappings can add sentinel glyphs: compare actual glyph IDs.
                visible = [c for c in chars if c[1] != -1 and c[1] != 65535]
                gids = [c[1] for c in visible]
                assert gids == ([3, 2] if symbolic else [2, 3]), (name, chars)
                assert abs(visible[1][2][0] - (107 if symbolic else 104)) < .0001, (name, chars)
                assert all(abs(c[2][1] - 92) < .0001 for c in visible)
                pix = page.get_pixmap(colorspace=pymupdf.csGRAY, alpha=False)
                assert pix.pixel(105, 89)[0] == (0 if symbolic else 255), (name, pix.pixel(105,89))
                assert pix.pixel(101, 91)[0] < 64
                assert pix.pixel(100, 85)[0] == 255
                digest = hashlib.sha256(pix.samples).hexdigest()
                raster[(profile, subset, unicode)] = digest
                text = page.get_text()
            results.append({'case': name, 'pdf_sha256': hashlib.sha256(pdf).hexdigest(), 'gids': gids, 'origins': [list(c[2]) for c in visible], 'raster_sha256': digest, 'mupdf_text_observation': text, 'unicode_policy': 'explicit ToUnicode' if unicode else ('reader heuristic; not a normative Unicode oracle' if symbolic else 'WinAnsi'), 'qpdf': 'structure only'})
    assert len({raster[(profile,s,u)] for s in [False,True] for u in [False,True]}) == 1
assert raster[('symbol',False,False)] != raster[('unicode',False,False)]
Path('docs/reports/2026-10-04-f07-evidence/readers.json').write_text(json.dumps({'versions': [fontTools.__version__,pymupdf.VersionBind,pymupdf.VersionFitz], 'results': results}, indent=2)+'\n')
print('16 glyph/position/raster/structure controls passed; four full/subset pairs identical.')
