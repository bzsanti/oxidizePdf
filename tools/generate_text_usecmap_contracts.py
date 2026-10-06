#!/usr/bin/env python3
"""Original PDF UseCMap inheritance/notdef fixtures for #666; standard library only."""
import argparse
import hashlib
import json
from pathlib import Path

from text_contracts.fixture_pdf import assemble, cmap, stream


ROS = "/Registry (Contract) /Ordering (Synthetic) /Supplement 0"
SPACE = "1 begincodespacerange <00> <FF> endcodespacerange"
ENC = "2 begincidchar <01> 17 <02> 29 endcidchar"
UNICODE = "2 beginbfchar <01> <0041> <02> <0042> endbfchar"
CASES = {
    "flat": ("AB", 104, "01", "02"),
    "encoding-parent": ("AB", 104, "01", "02"),
    "encoding-shadow": ("AB", 107, "01", "02"),
    "unicode-parent": ("AB", 104, "01", "02"),
    "unicode-shadow": ("XB", 104, "01", "02"),
    "both-chain": ("AB", 104, "01", "02"),
    "named-operator": ("AB", 104, "0011", "001D"),
    "named-dictionary": ("AB", 104, "0011", "001D"),
    "notdef-char": ("�B", 105, "03", "02"),
    "notdef-range": ("�B", 105, "04", "02"),
    "notdef-explicit-wins": ("AB", 104, "01", "02"),
}


def map_object(kind, name, body, parent=""):
    dictionary = f"/Type /CMap /CMapName /{name} /CIDSystemInfo << {ROS} >> /WMode 0" if kind == 1 else ""
    if parent:
        dictionary += f" /UseCMap {parent}"
    return stream(cmap(kind, name, body, ROS), dictionary)


def make_pdf(font, mode):
    _, _, first, second = CASES[mode]
    enc, uni = SPACE + "\n" + ENC, SPACE + "\n" + UNICODE
    enc_parent = uni_parent = ""
    parent_enc_body, parent_uni_body = enc, uni
    parent_enc_parent = parent_uni_parent = ""
    if mode in {"encoding-parent", "encoding-shadow", "both-chain"}:
        enc, enc_parent = "", "11 0 R"
        if mode == "encoding-shadow":
            # Swap both CIDs: avoid conflating inheritance with readers that
            # collapse distinct source-code Unicode mappings onto a shared CID.
            enc = SPACE + "\n2 begincidchar <01> 29 <02> 17 endcidchar"
    if mode in {"unicode-parent", "unicode-shadow", "both-chain"}:
        uni, uni_parent = "", "12 0 R"
        if mode == "unicode-shadow":
            uni = SPACE + "\n1 beginbfchar <01> <0058> endbfchar"
    if mode == "both-chain":
        parent_enc_body = parent_uni_body = ""
        parent_enc_parent, parent_uni_parent = "13 0 R", "14 0 R"
    if mode.startswith("named-"):
        enc = "/Identity-H usecmap" if mode == "named-operator" else ""
        enc_parent = "/Identity-H" if mode == "named-dictionary" else ""
        uni = "1 begincodespacerange <0000> <FFFF> endcodespacerange 2 beginbfchar <0011> <0041> <001D> <0042> endbfchar"
    if mode.startswith("notdef-"):
        enc += ("\n1 beginnotdefchar <03> 0 endnotdefchar" if mode == "notdef-char" else
                "\n1 beginnotdefrange <00> <FF> 0 endnotdefrange")
        uni = SPACE + "\n4 beginbfchar <01> <0041> <02> <0042> <03> <FFFD> <04> <FFFD> endbfchar"
    objects = [b"<< /Type /Catalog /Pages 2 0 R >>", b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
               b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
               b"<< /Type /Font /Subtype /Type0 /BaseFont /ContractCID /Encoding 9 0 R /DescendantFonts [6 0 R] /ToUnicode 10 0 R >>",
               stream(f"BT /F1 10 Tf 100 700 Td <{first}> Tj 1 Tr <{second}> Tj ET".encode()),
               (f"<< /Type /Font /Subtype /CIDFontType0 /BaseFont /ContractCID /CIDSystemInfo << {ROS} >> "
                "/FontDescriptor 7 0 R /DW 1000 /W [0 [500] 17 [400] 29 [700]] >>").encode(),
               b"<< /Type /FontDescriptor /FontName /ContractCID /Flags 4 /FontBBox [0 0 700 600] /ItalicAngle 0 /Ascent 600 /Descent 0 /CapHeight 600 /StemV 80 /FontFile3 8 0 R >>",
               stream(font, "/Subtype /CIDFontType0C"),
               map_object(1, "ChildEncoding", enc, enc_parent),
               map_object(2, "ChildUnicode", uni, uni_parent),
               map_object(1, "ParentEncoding", parent_enc_body, parent_enc_parent),
               map_object(2, "ParentUnicode", parent_uni_body, parent_uni_parent),
               map_object(1, "GrandparentEncoding", SPACE + "\n" + ENC),
               map_object(2, "GrandparentUnicode", SPACE + "\n" + UNICODE)]
    return assemble(objects)


def generate(output):
    root = Path(__file__).resolve().parents[1]
    source = root / "oxidize-pdf-core/tests/fixtures/text_contracts/fonts/ContractCID.cff"
    provenance = json.loads(source.with_name('cid-cff-provenance.json').read_text())
    font = source.read_bytes()
    if hashlib.sha256(font).hexdigest() != provenance['sha256']:
        raise ValueError("CID-keyed CFF differs from pinned independent program")
    output.mkdir(parents=True, exist_ok=True)
    cases = []
    for mode, (text, x, _, _) in CASES.items():
        data = make_pdf(font, mode)
        filename = mode + '.pdf'
        (output / filename).write_bytes(data)
        cases.append({"id": 'usecmap-' + mode, "path": filename, "sha256": hashlib.sha256(data).hexdigest(),
                      "expected_text": text,
                      "expectation_source": "ISO 32000-1 9.7.5/Table 120 UseCMap inheritance; child overrides parent. Explicit source-code ToUnicode and W: CID0=500, CID17=400, CID29=700. notdef ranges use one CID, not incrementing CIDs.",
                      "expected_trace_origins": [{"text": text[0], "origin": [100,92]}, {"text":text[1],"origin":[x,92]}]})
    (output / 'readers.json').write_text(json.dumps({'schema_version':1,'issue':666,'cases':cases},ensure_ascii=False,indent=2)+'\n')
    (output / 'provenance.json').write_text(json.dumps({'license':'Original project fixtures; repository MIT license',
        'font_source':'../fonts/ContractCID.cff','font_sha256':provenance['sha256'],
        'sha256':{case['path']:case['sha256'] for case in cases},
        'scope':'Valid horizontal Encoding/ToUnicode inheritance and notdef; cycles and malformed-code policy not asserted.'},indent=2)+'\n')


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output',type=Path)
    generate(parser.parse_args().output)
