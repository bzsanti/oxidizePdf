#!/usr/bin/env python3
"""Generate an original synthetic Type1C fixture, FontTools 4.60.1, offline.
Glyph names come from the independently checked MacExpert table; outlines are
original rectangles, not copies of a commercial expert font. Tests do not run
this generator. Usage: script TABLE OUTPUT_DIRECTORY
"""
from pathlib import Path
import hashlib
import json
import sys
import fontTools
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.t2CharStringPen import T2CharStringPen


def generate(table, output):
    if fontTools.__version__ != "4.60.1":
        raise ValueError("requires FontTools 4.60.1")
    names = [line.split("\t")[1] for line in table.read_text().splitlines()
             if not line.startswith("#") and line.split("\t")[1] != "-"]
    if len(names) != 165 or len(set(names)) != 165:
        raise ValueError("expected complete MacExpert glyph inventory")
    order = [".notdef"] + names
    builder = FontBuilder(1000, isTTF=False)
    builder.setupGlyphOrder(order)
    builder.setupCharacterMap({})
    chars = {}
    for name in order:
        pen = T2CharStringPen(500, None)
        if name not in [".notdef", "space"]:
            pen.moveTo((50, 0)); pen.lineTo((450, 0))
            pen.lineTo((450, 600)); pen.lineTo((50, 600)); pen.closePath()
        chars[name] = pen.getCharString()
    builder.setupCFF("ContractExpert", {"FullName": "ContractExpert",
        "FamilyName": "ContractExpert", "Weight": "Regular"}, chars, {})
    builder.setupHorizontalMetrics({name: (500, 0) for name in order})
    raw = builder.font["CFF "].compile(builder.font)
    output.mkdir(parents=True, exist_ok=True)
    (output / "ContractExpert.cff").write_bytes(raw)
    (output / "expert-provenance.json").write_text(json.dumps({
        "license": "Original project test fixture; same license as repository",
        "tool": "FontTools 4.60.1", "program": "name-keyed CFF/Type2 charstrings (PDF Type1C)",
        "glyphs": len(order), "width": 500,
        "outline_note": "original rectangles, not expert typography; space and .notdef empty",
        "oracle_sha256": hashlib.sha256(table.read_bytes()).hexdigest(),
        "sha256": hashlib.sha256(raw).hexdigest(),
    }, indent=2) + "\n")


if __name__ == "__main__":
    generate(Path(sys.argv[1]), Path(sys.argv[2]))
