#!/usr/bin/env python3
"""Offline Adobe CMap sampling, independent of oxidize tables. Python stdlib only.
Inputs are the two pinned upstream archives extracted unchanged. Never reads src/.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re

CMAP_REV = 'f5cf3bca7fdfeaceb77aa82847e974f2306c20b4'
UNICODE_REV = '2dd5e53fb74a01718b9dfd448a0d1cce6fff2aa5'
FAMILIES = {
    'GB1': (6, ['GB-EUC', 'GBK-EUC', 'GBKp-EUC', 'UniGB-UCS2', 'UniGB-UTF16']),
    'CNS1': (7, ['B5pc', 'ETen-B5', 'UniCNS-UCS2', 'UniCNS-UTF16']),
    'Japan1': (7, ['90ms-RKSJ', '90pv-RKSJ', 'UniJIS-UCS2', 'UniJIS-UTF16']),
    'Korea1': (2, ['KSC-EUC', 'KSCms-UHC', 'UniKS-UCS2', 'UniKS-UTF16']),
    'KR': (9, ['UniAKR-UTF16']),
}

def entries(path, kind, stack=()):
    """Read only Adobe's line-oriented char/range syntax; reject unknown rows."""
    if path.name in stack:
        raise ValueError('cyclic usecmap')
    text = path.read_text('ascii')
    result = {}
    parents = re.findall(r'^/([^\s]+) usecmap$', text, re.M)
    for parent in parents:
        result.update(entries(path.with_name(parent), kind, stack + (path.name,)))
    for count, mode, body in re.findall(r'(\d+) begin(' + kind + r'(?:char|range))\s+(.*?)end\2', text, re.S):
        rows = [line.strip() for line in body.splitlines() if line.strip()]
        if len(rows) != int(count):
            raise ValueError(f'{path}: row count mismatch')
        for row in rows:
            pattern = r'<([0-9a-fA-F]+)>\s+' + (r'<([0-9a-fA-F]+)>\s+' if mode.endswith('range') else '')
            pattern += r'(\d+)' if kind == 'cid' else r'<([0-9a-fA-F]+)>'
            match = re.fullmatch(pattern, row)
            if not match:
                raise ValueError(f'{path}: unsupported row {row}')
            parts = match.groups()
            lo = bytes.fromhex(parts[0])
            hi = bytes.fromhex(parts[1]) if mode.endswith('range') else lo
            if len(lo) != len(hi) or lo > hi:
                raise ValueError('invalid source range')
            dst = int(parts[-1], 10 if kind == 'cid' else 16)
            for delta in range(int.from_bytes(hi, 'big') - int.from_bytes(lo, 'big') + 1):
                code = (int.from_bytes(lo, 'big') + delta).to_bytes(len(lo), 'big')
                result[code] = dst + delta if kind == 'cid' else (dst + delta).to_bytes(len(parts[-1]) // 2, 'big').decode('utf-16-be')
    return result


def generate(cmaps, unicode, output):
    sources = {}
    def record(path, root, repo, rev):
        relative = str(path.relative_to(root))
        sources[f'{repo}/{relative}'] = {'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
            'url': f'https://github.com/adobe-type-tools/{repo}/blob/{rev}/{relative}'}
    rows = []
    for ordering, (supplement, bases) in FAMILIES.items():
        unicode_path = unicode / 'pdf2unicode' / f'Adobe-{ordering}-UCS2'
        decoded = entries(unicode_path, 'bf')
        record(unicode_path, unicode, 'mapping-resources-pdf', UNICODE_REV)
        directory = cmaps / f'Adobe-{ordering}-{supplement}' / 'CMap'
        for base in bases:
            for direction in ('H',) if ordering == 'KR' else ('H', 'V'):
                name = f'{base}-{direction}'
                path = directory / name
                mapping = entries(path, 'cid')
                record(path, cmaps, 'cmap-resources', CMAP_REV)
                for parent in re.findall(r'^/([^\s]+) usecmap$', path.read_text(), re.M):
                    record(path.with_name(parent), cmaps, 'cmap-resources', CMAP_REV)
                candidates = [(code, cid, decoded[cid.to_bytes(2, 'big')]) for code, cid in sorted(mapping.items()) if cid > 0 and cid.to_bytes(2, 'big') in decoded and all(ord(c) >= 32 for c in decoded[cid.to_bytes(2, 'big')])]
                cjk = [r for r in candidates if any(0x3400 <= ord(c) <= 0x9fff or 0xac00 <= ord(c) <= 0xd7af for c in r[2])]
                supplementary = [r for r in candidates if any(ord(c) > 0xffff for c in r[2])]
                selected = [('first-cjk', cjk[0]), ('last-cjk', cjk[-1])]
                if supplementary:
                    selected.append(('non-bmp', supplementary[0]))
                wide = [r for r in candidates if len(r[0]) == 4]
                if wide:
                    selected.append(('four-byte-source', wide[0]))
                if direction == 'V':
                    horizontal = entries(directory / f'{base}-H', 'cid')
                    overrides = [r for r in candidates if horizontal.get(r[0]) != r[1]]
                    if overrides:
                        selected.append(('vertical-override', overrides[0]))
                for category, (code, cid, text) in selected:
                    rows.append([ordering, str(supplement), name, direction, category, code.hex().upper(), str(cid), ' '.join(f'{ord(c):04X}' for c in text)])
    output.mkdir(parents=True, exist_ok=True)
    table = 'ordering\tsupplement\tcmap\tdirection\tcategory\tcode\tcid\tunicode\n' + ''.join('\t'.join(row) + '\n' for row in rows)
    (output / 'samples.tsv').write_text(table)
    manifest = {'cmap_revision': CMAP_REV, 'unicode_revision': UNICODE_REV, 'sources': sources,
        'samples_sha256': hashlib.sha256(table.encode()).hexdigest(), 'maps': len(set(r[2] for r in rows)), 'samples': len(rows),
        'scope': 'Deterministic representative samples, not exhaustive collection coverage. KR has no vertical UniAKR CMap in this revision. Unicode is CID fallback, not original Unicode input.'}
    (output / 'provenance.json').write_text(json.dumps(manifest, indent=2) + '\n')
    (output / 'ADOBE-LICENSE.txt').write_bytes((unicode / 'LICENSE.txt').read_bytes())

if __name__ == '__main__':
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('cmaps', type=Path)
    parser.add_argument('unicode', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    generate(args.cmaps, args.unicode, args.output)
