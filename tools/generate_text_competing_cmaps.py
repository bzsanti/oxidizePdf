#!/usr/bin/env python3
"""Generate original F07 competing cmap programs, using FontTools 4.60.1 offline.

PYTHONPATH=target/issue666-fonttools python3 tools/generate_text_competing_cmaps.py OUTPUT
The selector priority is an oxidize-pdf API policy, not a PDF symbolic-font rule.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path

import fontTools
from fontTools.ttLib import TTFont
from fontTools.ttLib.tables._c_m_a_p import CmapSubtable
from generate_text_symbolic_contracts import make_font


def generate(output):
    if fontTools.__version__ != "4.60.1":
        raise RuntimeError("Requires frozen FontTools 4.60.1")
    output.mkdir(parents=True, exist_ok=True)
    raw, _ = make_font("unicode", False)
    font = TTFont(io.BytesIO(raw), recalcTimestamp=False)
    specs = [(0, 3, 4, {65: "C"}), (1, 0, 0, {65: "space"}),
             (3, 0, 4, {0xF041: "A"}), (3, 1, 4, {65: "A"}),
             (3, 10, 12, {65: "B", 0x1F600: "C"})]
    tables = []
    for platform, encoding, fmt, mapping in specs:
        table = CmapSubtable.newSubtable(fmt)
        table.platformID, table.platEncID, table.language = platform, encoding, 0
        table.cmap = mapping
        tables.append(table)
    font["cmap"].tables = tables
    buffer = io.BytesIO()
    font.save(buffer)
    raw = buffer.getvalue()
    restored = TTFont(io.BytesIO(raw), recalcTimestamp=False)
    assert len(restored["cmap"].tables) == 5
    for table, spec in zip(restored["cmap"].tables, specs):
        assert (table.platformID, table.platEncID, table.format, table.cmap) == spec
    assert restored.getBestCmap() == {65: "B", 0x1F600: "C"}
    assert restored.getGlyphOrder() == [".notdef", "space", "A", "B", "C"]
    output.joinpath("competing.ttf").write_bytes(raw)
    output.joinpath("provenance.json").write_text(json.dumps({
        "generator": "tools/generate_text_competing_cmaps.py",
        "fonttools": fontTools.__version__, "origin": "Original synthetic glyf outlines; no external font input",
        "sha256": hashlib.sha256(raw).hexdigest(),
        "glyph_order": restored.getGlyphOrder(),
        "widths_by_gid": [restored["hmtx"][g][0] for g in restored.getGlyphOrder()],
        "subtables": [{"platform": p, "encoding": e, "format": f, "mapping": m} for p, e, f, m in specs],
        "independent_best_cmap": restored.getBestCmap(),
        "policy": "API priority (3,10) > (3,1) > (0,any); first fallback only when preferred platforms absent. Not a universal PDF symbolic-font oracle."
    }, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    generate(parser.parse_args().output)
