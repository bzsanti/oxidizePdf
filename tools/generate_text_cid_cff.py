#!/usr/bin/env python3
"""Original CID-keyed CFF fixture; FontTools 4.60.1; no external outlines.
Usage: script OUTPUT_DIRECTORY. The CID charset deliberately differs from GIDs.
"""
from pathlib import Path
import hashlib
import io
import json
import sys
import fontTools
from fontTools.cffLib import CFFFontSet, FDArrayIndex, FDSelect, FontDict
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.t2CharStringPen import T2CharStringPen


def generate(output):
    if fontTools.__version__ != "4.60.1":
        raise ValueError("requires FontTools 4.60.1")
    order = [".notdef", "cid00017", "cid00029"]
    widths = [500, 400, 700]
    builder = FontBuilder(1000, isTTF=False)
    builder.setupGlyphOrder(order)
    builder.setupCharacterMap({})
    chars = {}
    for name, width in zip(order, widths):
        pen = T2CharStringPen(width, None)
        if name == "cid00017":
            pen.moveTo((50, 0)); pen.lineTo((350, 0)); pen.lineTo((200, 600)); pen.closePath()
        elif name == "cid00029":
            pen.moveTo((50, 0)); pen.lineTo((650, 0))
            pen.lineTo((650, 600)); pen.lineTo((50, 600)); pen.closePath()
        chars[name] = pen.getCharString()
    builder.setupCFF("ContractCID", {"FullName": "ContractCID", "FamilyName": "ContractCID", "Weight": "Regular"}, chars, {})
    builder.setupHorizontalMetrics(dict(zip(order, [(width, 0) for width in widths])))
    top = builder.font["CFF "].cff.topDictIndex[0]
    top.ROS = ("Contract", "Synthetic", 0)
    top.CIDCount = 30
    fd = FontDict()
    fd.FontName = "ContractCID-FD0"
    fd.Private = top.Private
    array = FDArrayIndex()
    array.append(fd)
    top.FDArray = array
    top.FDSelect = FDSelect(format=0)
    top.FDSelect.gidArray = [0, 0, 0]
    top.CharStrings.fdArray = array
    top.CharStrings.fdSelect = top.FDSelect
    for charstring in top.CharStrings.values():
        charstring.fdSelectIndex = 0
    del top.Private
    raw = builder.font["CFF "].compile(builder.font)
    restored = CFFFontSet()
    restored.decompile(io.BytesIO(raw), None)
    decoded = restored.topDictIndex[0]
    assert decoded.ROS == ("Contract", "Synthetic", 0)
    assert decoded.charset == order
    assert decoded.FDSelect.gidArray == [0, 0, 0]
    from fontTools.pens.recordingPen import RecordingPen
    for name, width in zip(order, widths):
        charstring = decoded.CharStrings[name]
        charstring.draw(RecordingPen())
        assert charstring.width == width
    output.mkdir(parents=True, exist_ok=True)
    (output / "ContractCID.cff").write_bytes(raw)
    data = {"license": "Original project fixture; repository license", "tool": "FontTools 4.60.1",
            "format": "CID-keyed CFF1, Type2 charstrings, one FDArray entry",
            "ROS": ["Contract", "Synthetic", 0], "charset_by_gid": order,
            "widths_by_gid": widths, "CIDCount": 30, "sha256": hashlib.sha256(raw).hexdigest(),
            "scope": "Original triangle/rectangle outlines; not Chinese typography or Adobe collection coverage"}
    (output / "cid-cff-provenance.json").write_text(json.dumps(data, indent=2) + "\n")


if __name__ == "__main__":
    generate(Path(sys.argv[1]))
