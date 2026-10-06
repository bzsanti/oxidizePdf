#!/usr/bin/env python3
"""Generate original Type1/PFB and Type1C intrinsic-encoding fixtures for #666.

Offline, FontTools 4.60.1 only. No product tables or host fonts are used.
Usage: PYTHONPATH=target/issue666-fonttools python3 tools/generate_text_type1_contracts.py OUTPUT
"""
import argparse
import hashlib
import io
import json
from pathlib import Path

import fontTools
from fontTools import t1Lib
from fontTools.cffLib import CFFFontSet
from fontTools.fontBuilder import FontBuilder
from fontTools.misc.psCharStrings import T1CharString
from fontTools.pens.recordingPen import RecordingPen
from fontTools.pens.t2CharStringPen import T2CharStringPen


WIDTHS = {".notdef": 500, "space": 250, "A": 400, "B": 700, "C": 600}
OUTLINES = {
    "A": [(50, 0), (350, 0), (200, 600)],
    "B": [(50, 0), (650, 0), (650, 600), (50, 600)],
    "C": [(50, 0), (550, 0), (550, 300), (50, 300)],
}
CASES = {
    "intrinsic": ("", None, "BA", [100, 107]),
    "differences": ("/Encoding << /Differences [65 /A] >>", None, "AA", [100, 104]),
    "winansi": ("/Encoding /WinAnsiEncoding", None, "AB", [100, 104]),
    "tounicode": ("/Encoding << /Differences [65 /A] >>", "XY", "XY", [100, 104]),
}


def encoding(subset):
    table = [".notdef"] * 256
    table[32] = "space"
    table[65:67] = ["B", "A"]
    if not subset:
        table[67] = "C"
    return table


def verify_glyphs(chars, order):
    assert list(chars.keys()) == order
    for name in order:
        pen = RecordingPen()
        chars[name].draw(pen)
        assert chars[name].width == WIDTHS[name], name
        if name not in {".notdef", "space"}:
            assert pen.value[0] == ("moveTo", (OUTLINES[name][0],)), name
            assert pen.value[-1][0] == "closePath", name


def type1(output, name, subset):
    order = [".notdef", "space", "A", "B"] + ([] if subset else ["C"])
    chars = {}
    for glyph in order:
        program = [0, WIDTHS[glyph], "hsbw"]
        points = OUTLINES.get(glyph, [])
        if points:
            x, y = points[0]
            program += [x, y, "rmoveto"]
            for nx, ny in points[1:]:
                program += [nx - x, ny - y, "rlineto"]
                x, y = nx, ny
            program += ["closepath"]
        chars[glyph] = T1CharString(program=program + ["endchar"], subrs=[])
    font = t1Lib.T1Font.__new__(t1Lib.T1Font)
    font.encoding = "ascii"
    font.font = {
        "FontName": name, "FontType": 1, "PaintType": 0,
        "FontMatrix": [0.001, 0, 0, 0.001, 0, 0], "FontBBox": [0, 0, 700, 600],
        "Encoding": encoding(subset),
        "Private": {"RD": t1Lib.RD_value, "ND": t1Lib.ND_values[0],
                    "NP": t1Lib.PD_values[0], "BlueValues": [], "lenIV": 4, "Subrs": []},
        "CharStrings": chars,
    }
    pfb = output / f"{name}.pfb"
    font.saveAs(pfb, "PFB")
    restored = t1Lib.T1Font(pfb)
    assert restored["Encoding"] == encoding(subset)
    verify_glyphs(restored["CharStrings"], order)
    # PDF FontFile embeds the three payloads, without PFB record markers/lengths.
    chunks = t1Lib.findEncryptedChunks(t1Lib.readPFB(pfb))
    assert [kind for kind, _ in chunks] == [0, 1, 0]
    lengths = [len(chunk) for _, chunk in chunks]
    raw = b"".join(chunk for _, chunk in chunks)
    (output / f"{name}.bin").write_bytes(raw)
    return raw, f"/Length1 {lengths[0]} /Length2 {lengths[1]} /Length3 {lengths[2]}", lengths


def cff(output, name, subset):
    order = [".notdef", "space", "A", "B"] + ([] if subset else ["C"])
    builder = FontBuilder(1000, isTTF=False)
    builder.setupGlyphOrder(order)
    builder.setupCharacterMap({})
    chars = {}
    for glyph in order:
        pen = T2CharStringPen(WIDTHS[glyph], None)
        points = OUTLINES.get(glyph, [])
        if points:
            pen.moveTo(points[0])
            for point in points[1:]:
                pen.lineTo(point)
            pen.closePath()
        chars[glyph] = pen.getCharString()
    builder.setupCFF(name, {"FullName": name, "FamilyName": "ContractIntrinsic",
                           "Weight": "Regular", "FontBBox": [0, 0, 700, 600]}, chars, {})
    builder.setupHorizontalMetrics({glyph: (WIDTHS[glyph], 0) for glyph in order})
    builder.font["CFF "].cff.topDictIndex[0].Encoding = encoding(subset)
    raw = builder.font["CFF "].compile(builder.font)
    restored = CFFFontSet()
    restored.decompile(io.BytesIO(raw), None)
    top = restored.topDictIndex[0]
    assert top.Encoding == encoding(subset)
    verify_glyphs(top.CharStrings, order)
    (output / f"{name}.cff").write_bytes(raw)
    return raw, "/Subtype /Type1C", None


def stream(data, dictionary=""):
    return f"<< {dictionary} /Length {len(data)} >>\nstream\n".encode() + data + b"\nendstream"


def pdf(name, raw, dictionary, kind, mode):
    enc, unicode, _, _ = CASES[mode]
    # Widths follow the PDF encoding in each case, not the byte's ASCII identity.
    widths = "700 400" if mode == "intrinsic" else ("400 700" if mode == "winansi" else "400 400")
    definition = (f"<< /Type /Font /Subtype /Type1 /BaseFont /{name} {enc} "
                  f"/FirstChar 65 /LastChar 66 /Widths [{widths}] /FontDescriptor 6 0 R "
                  + ("/ToUnicode 8 0 R " if unicode else "") + ">>")
    objects = [b"<< /Type /Catalog /Pages 2 0 R >>",
               b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
               b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
               definition.encode(), stream(b"BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj ET"),
               (f"<< /Type /FontDescriptor /FontName /{name} /Flags 32 /FontBBox [0 0 700 600] "
                f"/ItalicAngle 0 /Ascent 600 /Descent 0 /CapHeight 600 /StemV 80 "
                f"/{'FontFile' if kind == 'pfb' else 'FontFile3'} 7 0 R >>").encode(),
               stream(raw, dictionary)]
    if unicode:
        objects.append(stream(b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n"
                              b"/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n"
                              b"/CMapName /Type1Contract def /CMapType 2 def\n"
                              b"1 begincodespacerange <00> <FF> endcodespacerange\n"
                              b"2 beginbfchar <41> <0058> <42> <0059> endbfchar\n"
                              b"endcmap CMapName currentdict /CMap defineresource pop end end"))
    data = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n"
    offsets = [0]
    for index, obj in enumerate(objects, 1):
        offsets.append(len(data))
        data += f"{index} 0 obj\n".encode() + obj + b"\nendobj\n"
    start = len(data)
    data += f"xref\n0 {len(offsets)}\n0000000000 65535 f \n".encode()
    data += b"".join(f"{offset:010} 00000 n \n".encode() for offset in offsets[1:])
    data += f"trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n".encode()
    return data


def generate(output):
    if fontTools.__version__ != "4.60.1":
        raise ValueError("requires FontTools 4.60.1")
    output.mkdir(parents=True, exist_ok=True)
    programs, cases = [], []
    for kind, builder in [("pfb", type1), ("cff", cff)]:
        for subset in [False, True]:
            variant = "subset" if subset else "full"
            name = ("ABCDEF+" if subset else "") + f"ContractIntrinsic{kind.upper()}"
            raw, dictionary, lengths = builder(output, name, subset)
            programs.append({"name": name, "kind": kind, "subset": subset,
                             "glyphs": [".notdef", "space", "A", "B"] + ([] if subset else ["C"]),
                             "encoding": {"32": "space", "65": "B", "66": "A", **({} if subset else {"67": "C"})},
                             "pdf_segment_lengths": lengths, "payload_sha256": hashlib.sha256(raw).hexdigest()})
            for mode, (_, _, text, xs) in CASES.items():
                filename = f"{kind}-{variant}-{mode}.pdf"
                data = pdf(name, raw, dictionary, kind, mode)
                (output / filename).write_bytes(data)
                cases.append({"id": f"type1-{kind}-{variant}-{mode}", "path": filename,
                              "sha256": hashlib.sha256(data).hexdigest(), "expected_text": text,
                              "expectation_source": "Original embedded encoding 65=B/66=A; named PDF Differences/WinAnsi or explicit ToUnicode. ISO 32000-1 9.6.6.1/9.10.2; widths 400/700 and Tfs=10.",
                              "expected_trace_origins": [{"text": ch, "origin": [x, 92]} for ch, x in zip(text, xs)]})
    # A previous unrelated file in OUTPUT must not become part of provenance.
    generated = [case["path"] for case in cases]
    for program in programs:
        suffixes = [".pfb", ".bin"] if program["kind"] == "pfb" else [".cff"]
        generated.extend(program["name"] + suffix for suffix in suffixes)
    hashes = {name: hashlib.sha256((output / name).read_bytes()).hexdigest()
              for name in sorted(generated)}
    provenance = {"tool": "FontTools 4.60.1", "license": "Original project fixtures; repository MIT license",
                  "scope": "Original triangle/rectangle outlines, full and real glyph subset, custom intrinsic encodings. No general Type1 interpreter or typographic conformance claim.",
                  "programs": programs, "sha256": hashes}
    (output / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
    (output / "readers.json").write_text(json.dumps({"schema_version": 1, "issue": 666, "cases": cases}, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    generate(parser.parse_args().output)
