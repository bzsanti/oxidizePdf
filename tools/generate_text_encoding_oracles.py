#!/usr/bin/env python3
"""Rebuild #666 test data from external sources; never import oxidize-pdf tables.

Usage: python3 tools/generate_text_encoding_oracles.py SOURCE_DIR OUTPUT_DIR
SOURCE_DIR holds text-contracts-{Encoding}.java, text-contracts-glyphlist.txt,
text-contracts-zapfdingbats.txt and text-contracts-iso32000.{pdf,txt}.
The text file must be produced by pdftotext -layout from the recorded PDF.
No network access, product imports, or automatic updates at test runtime.
"""

import argparse
import hashlib
import json
from pathlib import Path
import re

ENCODINGS = {
    "StandardEncoding": 149,
    "WinAnsiEncoding": 224,
    "MacRomanEncoding": 208,
    "MacExpertEncoding": 165,
    "SymbolEncoding": 189,
    "ZapfDingbatsEncoding": 188,
}
PDFBOX_REVISION = "804cc824f1a19bcce85a3d0d60a8f10e98188e52"
ADOBE_REVISION = "4036a9ca80a62f64f9de4f7321a9a045ad0ecfd6"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def generate(source, output):
    glyphs = {}
    for name in ("glyphlist", "zapfdingbats"):
        for line in (source / f"text-contracts-{name}.txt").read_text().splitlines():
            if line and not line.startswith("#"):
                name, scalars = line.split(";")
                if name in glyphs:
                    raise ValueError(f"duplicate glyph: {name}")
                glyphs[name] = scalars

    tables = {}
    for encoding, count in ENCODINGS.items():
        java = (source / f"text-contracts-{encoding}.java").read_text()
        entries = re.findall(r'\{(0[0-7]+),\s*"([^"]+)"\}', java)
        table = {}
        for octal, name in entries:
            code = int(octal, 8)
            if code in table or not 0 <= code <= 255:
                raise ValueError(f"duplicate/out-of-range {encoding} {octal}")
            table[code] = name
        if encoding == "WinAnsiEncoding":
            # Annex D.2 note 3: unused positions above octal 040 are bullet.
            for code in range(0o41, 256):
                table.setdefault(code, "bullet")
        if len(table) != count:
            raise ValueError(f"unexpected assigned population: {encoding}: {len(table)}")
        for name in table.values():
            if name not in glyphs:
                raise ValueError(f"no independent Unicode mapping for {name}")
        tables[encoding] = table

    # Independently reconcile named assignments against the normative document.
    spec = (source / "text-contracts-iso32000.txt").read_text()
    start = spec.index("      A   A             101")
    section = spec[start : spec.index("D.3 PDFDocEncoding Character Set", start)]
    pattern = (
        r"\b([A-Za-z][A-Za-z0-9]*)(?:\s+[1-6])?\s+"
        r"([0-7]{3}|—)\s+([0-7]{3}|—)\s+([0-7]{3}|—)\s+([0-7]{3}|—)"
    )
    rows = re.findall(pattern, section)
    if len(rows) != 229:
        raise ValueError(f"normative table extraction incomplete: {len(rows)}")
    for column, encoding in enumerate(("StandardEncoding", "MacRomanEncoding", "WinAnsiEncoding")):
        for name, *codes in rows:
            code = codes[column]
            if code != "—" and tables[encoding].get(int(code, 8)) != name:
                raise ValueError(f"reference disagrees with Annex D: {encoding} {code} {name}")

    # Compare complete Symbol/Zapf name-to-byte maps, including absence of extras.
    for encoding, heading, next_heading in (
        ("MacExpertEncoding", "D.4 Expert Set and MacExpertEncoding", "D.5 Symbol Set and Encoding"),
        ("SymbolEncoding", "D.5 Symbol Set and Encoding", "D.6 ZapfDingbats Set and Encoding"),
        ("ZapfDingbatsEncoding", "D.6 ZapfDingbats Set and Encoding", "Annex E"),
    ):
        section = spec[spec.index(heading):]
        section = section[:section.index(next_heading)]
        pairs = re.findall(r"\b([A-Za-z][A-Za-z0-9]*)\s+([0-7]{3})\b", section)
        normative = {}
        for name, octal in pairs:
            code = int(octal, 8)
            if code > 255:  # printed page number following the copyright footer
                continue
            if code in normative:
                raise ValueError(f"duplicate normative position: {encoding} {octal}")
            normative[code] = name
        if normative != tables[encoding]:
            raise ValueError(f"reference disagrees with Annex D: {encoding}")

    # Explicit semantic aliases: retain NBSP and soft-hyphen distinctions rather
    # than treating typographically identical glyphs as identical Unicode text.
    if tables["MacRomanEncoding"].get(0xDB) != "currency":
        raise ValueError("Mac OS euro substituted for PDF currency")
    if tables["MacRomanEncoding"].get(0xCA) != "nbspace":
        raise ValueError("missing MacRoman nonbreaking duplicate")
    if tables["WinAnsiEncoding"].get(0xA0) != "nbspace" or tables["WinAnsiEncoding"].get(0xAD) != "sfthyphen":
        raise ValueError("missing WinAnsi semantic duplicates")

    output.mkdir(parents=True, exist_ok=True)
    for encoding, table in tables.items():
        lines = ["# byte_hex\tglyph_name\tunicode_scalars_hex; '-' means undefined"]
        for code in range(256):
            name = table.get(code)
            lines.append(f"{code:02X}\t{name or '-'}\t{glyphs[name] if name else '-'}")
        (output / f"{encoding}.tsv").write_text("\n".join(lines) + "\n")

    # PDF document strings have their own repertoire, not a font encoding.
    section = spec[spec.index("D.3 PDFDocEncoding Character Set"):
                   spec.index("D.4 Expert Set and MacExpertEncoding")]
    document_rows = re.findall(
        r"(?m)^.*?\b(\d+)\s+0x([0-9a-f]{2})\s+([0-7]{4})\s+"
        r"(?:U\+([0-9A-F]{4}))?([^\n]*)", section)
    if len(document_rows) != 256:
        raise ValueError("incomplete PDFDocEncoding normative inventory")
    lines = ["# byte_hex\tstatus\tunicode_scalar_hex; '-' means undefined"]
    defined = 0
    for index, (decimal, hexadecimal, octal, scalar, notes) in enumerate(document_rows):
        if not index == int(decimal) == int(hexadecimal, 16) == int(octal, 8):
            raise ValueError("PDFDocEncoding positions disagree")
        undefined = re.search(r"\sU\s*$", notes) is not None
        if not undefined and not scalar:
            raise ValueError("missing defined PDFDocEncoding scalar")
        defined += not undefined
        lines.append(f"{index:02X}\t{'undefined' if undefined else 'defined'}\t{'-' if undefined else scalar}")
    if defined != 232:
        raise ValueError("unexpected PDFDocEncoding defined population")
    (output / "PDFDocEncoding.tsv").write_text("\n".join(lines) + "\n")

    sources = [source / f"text-contracts-{name}.java" for name in ENCODINGS]
    sources += [source / f"text-contracts-{name}.txt" for name in ("glyphlist", "zapfdingbats", "iso32000")]
    sources.append(source / "text-contracts-iso32000.pdf")
    provenance = {
        "schema_version": 1,
        "pdfbox_revision": PDFBOX_REVISION,
        "adobe_agl_revision": ADOBE_REVISION,
        "normative_reference": "PDF 32000-1:2008 Annex D.2-D.6; printed pages 653-672",
        "normative_url": "https://raw.githubusercontent.com/adobe/dc-acrobat-sdk-docs/master/docs/pdfstandards/PDF32000_2008.pdf",
        "pdfbox_url_template": f"https://raw.githubusercontent.com/apache/pdfbox/{PDFBOX_REVISION}/pdfbox/src/main/java/org/apache/pdfbox/pdmodel/font/encoding/{{encoding}}.java",
        "adobe_url_template": f"https://raw.githubusercontent.com/adobe-type-tools/agl-aglfn/{ADOBE_REVISION}/{{name}}.txt",
        "unicode_alias_policy": {"nbspace": "00A0", "sfthyphen": "00AD"},
        "undefined_policy": "inventory only; extraction behavior requires a separate contract",
        "normative_crosscheck": "229 named rows; Standard, MacRoman, WinAnsi assignments checked; Symbol 189 and Zapf 188 complete assignments checked; PDFDocEncoding 256 positions checked (232 defined, 24 undefined); MacExpert 165 complete assignments checked",
        "assigned_counts": {**ENCODINGS, "PDFDocEncoding": 232},
        "source_sha256": {path.name: digest(path) for path in sources},
        "table_sha256": {f"{name}.tsv": digest(output / f"{name}.tsv") for name in (*ENCODINGS, "PDFDocEncoding")},
    }
    (output / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    generate(args.source, args.output)
