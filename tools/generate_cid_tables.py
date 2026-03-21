#!/usr/bin/env python3
"""
Generate CID-to-Unicode mapping tables for oxidize-pdf.

Downloads Adobe CMap Resources (cid2code.txt) and generates a Rust source file
with static lookup tables for Adobe-CNS1, Adobe-GB1, Adobe-Japan1, and Adobe-Korea1.

Source: https://github.com/adobe-type-tools/cmap-resources
License: BSD-3-Clause (Adobe)

Usage:
    python3 tools/generate_cid_tables.py

Output:
    oxidize-pdf-core/src/text/cid_to_unicode.rs
"""

import os
import sys
import urllib.request

BASE_URL = "https://raw.githubusercontent.com/adobe-type-tools/cmap-resources/master"

# (name, directory, utf16_column_index)
COLLECTIONS = [
    ("CNS1", "Adobe-CNS1-7", 10),   # UniCNS-UTF16
    ("GB1", "Adobe-GB1-6", 12),      # UniGB-UTF16
    ("JAPAN1", "Adobe-Japan1-7", 20), # UniJIS-UTF16
    ("KOREA1", "Adobe-Korea1-2", 9),  # UniKS-UTF16
]


def download(url: str) -> str:
    with urllib.request.urlopen(url) as resp:
        return resp.read().decode("utf-8")


def parse_cid2code(text: str, utf16_col: int) -> list[tuple[int, int]]:
    """Parse cid2code.txt and return sorted list of (cid, unicode_codepoint)."""
    mappings = []
    for line in text.splitlines():
        if line.startswith("#") or line.startswith("CID"):
            continue
        parts = line.strip().split("\t")
        if len(parts) <= utf16_col:
            continue
        cid = int(parts[0])
        utf16_val = parts[utf16_col]
        if utf16_val == "*":
            continue
        code = utf16_val.split(",")[0].strip().rstrip("v")
        if not code:
            continue
        try:
            cp = int(code, 16)
            if 0 < cp <= 0x10FFFF:
                mappings.append((cid, cp))
        except ValueError:
            continue
    mappings.sort()
    return mappings


def generate_rust(collections_data: dict[str, list[tuple[int, int]]]) -> str:
    lines = [
        "// Auto-generated from Adobe CMap Resources (cid2code.txt)",
        "// Source: https://github.com/adobe-type-tools/cmap-resources",
        "// License: BSD-3-Clause (Adobe)",
        "//",
        "// DO NOT EDIT MANUALLY - regenerate with tools/generate_cid_tables.py",
        "",
        "/// Adobe CID character collection identifiers",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum CidCollection {",
        "    /// Adobe-CNS1 (Traditional Chinese)",
        "    Cns1,",
        "    /// Adobe-GB1 (Simplified Chinese)",
        "    Gb1,",
        "    /// Adobe-Japan1 (Japanese)",
        "    Japan1,",
        "    /// Adobe-Korea1 (Korean)",
        "    Korea1,",
        "}",
        "",
        "impl CidCollection {",
        "    /// Detect collection from CIDSystemInfo Registry/Ordering",
        "    pub fn from_ordering(ordering: &str) -> Option<Self> {",
        "        match ordering {",
        '            "CNS1" => Some(Self::Cns1),',
        '            "GB1" => Some(Self::Gb1),',
        '            "Japan1" => Some(Self::Japan1),',
        '            "Korea1" | "KR" => Some(Self::Korea1),',
        "            _ => None,",
        "        }",
        "    }",
        "",
        "    /// Look up the Unicode code point for a CID in this collection.",
        "    /// Returns None if the CID is not mapped.",
        "    pub fn cid_to_unicode(&self, cid: u16) -> Option<char> {",
        "        let table: &[(u16, u32)] = match self {",
        "            Self::Cns1 => &CNS1_CID_TO_UNICODE,",
        "            Self::Gb1 => &GB1_CID_TO_UNICODE,",
        "            Self::Japan1 => &JAPAN1_CID_TO_UNICODE,",
        "            Self::Korea1 => &KOREA1_CID_TO_UNICODE,",
        "        };",
        "        // Binary search on sorted CID array",
        "        table",
        "            .binary_search_by_key(&cid, |&(c, _)| c)",
        "            .ok()",
        "            .and_then(|idx| char::from_u32(table[idx].1))",
        "    }",
        "}",
        "",
    ]

    for name in ["CNS1", "GB1", "JAPAN1", "KOREA1"]:
        mappings = collections_data[name]
        lines.append(
            f"/// Adobe-{name} CID → Unicode mapping ({len(mappings)} entries)"
        )
        lines.append(
            f"static {name}_CID_TO_UNICODE: [(u16, u32); {len(mappings)}] = ["
        )
        for i in range(0, len(mappings), 8):
            chunk = mappings[i : i + 8]
            entries = ", ".join(f"({cid}, 0x{cp:04X})" for cid, cp in chunk)
            lines.append(f"    {entries},")
        lines.append("];")
        lines.append("")

    return "\n".join(lines)


def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(script_dir)
    outpath = os.path.join(
        project_root, "oxidize-pdf-core", "src", "text", "cid_to_unicode.rs"
    )

    collections_data = {}
    for name, directory, col in COLLECTIONS:
        url = f"{BASE_URL}/{directory}/cid2code.txt"
        print(f"Downloading {name} from {url}...")
        text = download(url)
        mappings = parse_cid2code(text, col)
        collections_data[name] = mappings
        print(f"  {len(mappings)} entries (max CID={max(c for c, _ in mappings)})")

    rust_code = generate_rust(collections_data)
    with open(outpath, "w") as f:
        f.write(rust_code)

    size = os.path.getsize(outpath)
    total = sum(len(m) for m in collections_data.values())
    print(f"\nGenerated {outpath}")
    print(f"  {total} total entries, {size / 1024:.1f} KB")


if __name__ == "__main__":
    main()
