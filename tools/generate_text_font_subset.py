#!/usr/bin/env python3
"""Offline #666 fixture generation: FontTools 4.60.1, official SourceSans 3.052 TTF.
Usage: PYTHONPATH=FONTTOOLS_PATH python3 tools/generate_text_font_subset.py SOURCE OUTPUT
No production library code participates in the reference or subsetting.
"""
import hashlib
import json
from pathlib import Path
import sys
import fontTools
from fontTools import subset
from fontTools.ttLib import TTFont


def generate(source, output):
    if fontTools.__version__ != "4.60.1":
        raise ValueError("requires FontTools 4.60.1")
    expected = "4644c81b86ec9caaa76b634889968ed3c4f4f52f054855933acc7c2b21e53b0f"
    if hashlib.sha256(source.read_bytes()).hexdigest() != expected:
        raise ValueError("input is not the pinned official SourceSans 3.052 TTF")
    font = TTFont(source, recalcTimestamp=False)
    if "glyf" not in font or "CFF " in font:
        raise ValueError("expected TrueType outlines")
    original_count = len(font.getGlyphOrder())
    repertoire = [0x20, 0x41, 0x42, 0xE9]
    cmap = font.getBestCmap()
    widths = {f"{code:04X}": font["hmtx"][cmap[code]][0] for code in repertoire}
    pdf_widths = []
    for code in range(32, 234):
        try:
            scalar = ord(bytes([code]).decode("cp1252"))
        except UnicodeDecodeError:
            scalar = 0x2022
        if code == 0x7F:
            scalar = 0x2022
        name = cmap.get(scalar, ".notdef")
        pdf_widths.append(font["hmtx"][name][0])
    units = font["head"].unitsPerEm
    options = subset.Options()
    options.recalc_timestamp = False
    options.name_IDs = [0, 1, 2, 3, 4, 5, 6, 13, 14, 16, 17]
    options.name_legacy = True
    options.name_languages = [0x409]
    processor = subset.Subsetter(options=options)
    processor.populate(unicodes=repertoire)
    processor.subset(font)
    # Modified font must not use the OFL Reserved Font Name Source as its name.
    names = {1: "ContractSansSubset", 2: "Regular", 3: "ContractSansSubset 1.0",
             4: "ContractSansSubset Regular", 6: "ContractSansSubset-Regular",
             16: "ContractSansSubset", 17: "Regular"}
    for record in font["name"].names:
        if record.nameID in names:
            record.string = names[record.nameID].encode(record.getEncoding())
    output.mkdir(parents=True, exist_ok=True)
    target = output / "ContractSansSubset-Regular.ttf"
    font.save(target)
    restored = TTFont(target, recalcTimestamp=False)
    new_cmap = restored.getBestCmap()
    assert set(new_cmap) == set(repertoire)
    assert len(restored.getGlyphOrder()) < original_count
    for code in repertoire:
        assert restored["hmtx"][new_cmap[code]][0] == widths[f"{code:04X}"]
    assert restored["glyf"][new_cmap[0xE9]].isComposite()
    data = {
        "source_url": "https://raw.githubusercontent.com/adobe-fonts/source-sans/3.052R/TTF/SourceSans3-Regular.ttf",
        "license": "SIL OFL 1.1; see SourceSans3-LICENSE.md; subset renamed",
        "tool": "FontTools 4.60.1", "units_per_em": units,
        "full_glyph_count": original_count, "subset_glyph_count": len(restored.getGlyphOrder()),
        "widths": widths, "winansi_widths_32_233": pdf_widths, "subset_cmap": {f"{k:04X}": v for k, v in new_cmap.items()},
        "sha256": {"SourceSans3-Regular.ttf": hashlib.sha256(source.read_bytes()).hexdigest(),
                   target.name: hashlib.sha256(target.read_bytes()).hexdigest()},
    }
    (output / "truetype-provenance.json").write_text(json.dumps(data, indent=2) + "\n")


if __name__ == "__main__":
    generate(Path(sys.argv[1]), Path(sys.argv[2]))
