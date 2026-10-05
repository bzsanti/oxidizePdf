#!/usr/bin/env python3
"""Original TrueType symbolic/platform cmap fixtures, FontTools 4.60.1, offline.

Usage: PYTHONPATH=target/issue666-fonttools python3 tools/generate_text_symbolic_contracts.py OUTPUT
Expected PDF Unicode is explicit ToUnicode, not guessed from symbolic cmap codes.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path

import fontTools
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont
from fontTools.ttLib.tables._c_m_a_p import CmapSubtable

from text_contracts.fixture_pdf import assemble, cmap, stream


WIDTHS = {".notdef": 500, "space": 250, "A": 400, "B": 700, "C": 600}
OUTLINES = {"A": [(50, 0), (350, 0), (200, 600)],
            "B": [(50, 0), (650, 0), (650, 600), (50, 600)],
            "C": [(50, 0), (550, 0), (550, 300), (50, 300)]}
PROFILES = {"symbol": (3, 0, 4), "mac": (1, 0, 0),
            "unicode": (3, 1, 4), "ucs4": (3, 10, 12)}


def make_font(profile, subset):
    symbolic = profile in {"symbol", "mac"}
    order = [".notdef", "space", "A", "B"] + ([] if subset else ["C"])
    name = ("ABCDEF+" if subset else "") + f"ContractTT{profile.title()}"
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(order)
    # OS/2 ranges are established from Unicode before selecting the actual cmap.
    fb.setupCharacterMap({32: "space", 65: "A", 66: "B"})
    glyphs = {}
    for glyph in order:
        pen = TTGlyphPen(None)
        points = OUTLINES.get(glyph, [])
        if points:
            pen.moveTo(points[0])
            for point in points[1:]:
                pen.lineTo(point)
            pen.closePath()
        glyphs[glyph] = pen.glyph()
    fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics({glyph: (WIDTHS[glyph], 0) for glyph in order})
    fb.setupHorizontalHeader(ascent=600, descent=0)
    fb.setupNameTable({"familyName": name, "styleName": "Regular", "uniqueFontIdentifier": name,
                       "fullName": name, "psName": name, "version": "Version 1.000"})
    fb.setupOS2(sTypoAscender=600, sTypoDescender=0, usWinAscent=600, usWinDescent=0)
    fb.setupPost()
    table = CmapSubtable.newSubtable(PROFILES[profile][2])
    table.platformID, table.platEncID, table.language = *PROFILES[profile][:2], 0
    base = 0xF000 if profile == "symbol" else 0
    table.cmap = {base + 32: "space", base + 65: "B" if symbolic else "A",
                  base + 66: "A" if symbolic else "B"}
    if not subset:
        table.cmap[0x1F600 if profile == "ucs4" else base + 67] = "C"
    fb.font["cmap"].tables = [table]
    fb.font["head"].created = fb.font["head"].modified = 3400000000
    buffer = io.BytesIO()
    fb.save(buffer)
    raw = buffer.getvalue()
    restored = TTFont(io.BytesIO(raw), recalcTimestamp=False)
    assert restored.getGlyphOrder() == order
    actual = restored["cmap"].tables
    assert len(actual) == 1 and actual[0].cmap == table.cmap
    assert (actual[0].platformID, actual[0].platEncID, actual[0].format) == PROFILES[profile]
    for glyph in order:
        assert restored["hmtx"][glyph][0] == WIDTHS[glyph]
        assert restored["glyf"][glyph].numberOfContours == int(glyph in OUTLINES)
    metadata = {"name": name, "profile": profile, "subset": subset, "symbolic": symbolic,
                "platform_id": table.platformID, "encoding_id": table.platEncID, "format": table.format,
                "glyph_order": order, "mapping": {str(code): order.index(glyph) for code, glyph in table.cmap.items()},
                "widths_by_gid": [WIDTHS[glyph] for glyph in order]}
    return raw, metadata


def make_pdf(raw, info, unicode):
    symbolic = info["symbolic"]
    name = info["name"]
    encoding = "" if symbolic else "/Encoding /WinAnsiEncoding"
    widths = "700 400" if symbolic else "400 700"
    definition = (f"<< /Type /Font /Subtype /TrueType /BaseFont /{name} {encoding} "
                  f"/FirstChar 65 /LastChar 66 /Widths [{widths}] /FontDescriptor 6 0 R "
                  + ("/ToUnicode 8 0 R " if unicode else "") + ">>")
    objects = [b"<< /Type /Catalog /Pages 2 0 R >>", b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
               b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
               definition.encode(), stream(b"BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj ET"),
               (f"<< /Type /FontDescriptor /FontName /{name} /Flags {4 if symbolic else 32} "
                "/FontBBox [0 0 700 600] /ItalicAngle 0 /Ascent 600 /Descent 0 /CapHeight 600 /StemV 80 /FontFile2 7 0 R >>").encode(),
               stream(raw, f"/Length1 {len(raw)}")]
    if unicode:
        objects.append(stream(cmap(2, "SymbolicUnicode", "1 begincodespacerange <00> <FF> endcodespacerange\n"
                                  "2 beginbfchar <41> <00660069> <42> <D83DDE00> endbfchar")))
    return assemble(objects)


def generate(output):
    if fontTools.__version__ != "4.60.1":
        raise ValueError("requires FontTools 4.60.1")
    output.mkdir(parents=True, exist_ok=True)
    programs, cases, hashes = [], [], {}
    for profile in PROFILES:
        for subset in [False, True]:
            raw, info = make_font(profile, subset)
            stem = f"{profile}-{'subset' if subset else 'full'}"
            info['file'] = stem + '.ttf'
            (output / info['file']).write_bytes(raw)
            hashes[info['file']] = hashlib.sha256(raw).hexdigest()
            programs.append(info)
            # Symbolic decoding without ToUnicode has no general Unicode oracle.
            for unicode in ([True] if info['symbolic'] else [False, True]):
                filename = stem + ('-unicode.pdf' if unicode else '-winansi.pdf')
                data = make_pdf(raw, info, unicode)
                (output / filename).write_bytes(data)
                hashes[filename] = hashlib.sha256(data).hexdigest()
                cases.append({"id": filename[:-4], "path": filename, "sha256": hashes[filename],
                              "expected_text": "fi😀" if unicode else "AB",
                              "expectation_source": "Explicit ToUnicode UTF-16BE fi/surrogate pair, or WinAnsi for nonsymbolic TrueType. Original program and declared PDF Widths; not a symbolic-code-to-Unicode assumption."})
    (output / 'provenance.json').write_text(json.dumps({"tool": "FontTools 4.60.1", "license": "Original project fixtures; repository MIT license",
        "programs": programs, "sha256": hashes,
        "scope": "Real glyf programs and formats 0/4/12; symbolic Unicode without ToUnicode intentionally not inferred."}, indent=2)+'\n')
    (output / 'readers.json').write_text(json.dumps({"schema_version": 1, "issue": 666, "cases": cases}, indent=2)+'\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    generate(parser.parse_args().output)
