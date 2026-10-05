#!/usr/bin/env python3
"""Additional KR oracles from pinned upstream files; never reads product tables."""
import argparse
import hashlib
import json
from pathlib import Path

from generate_text_cjk_oracles import CMAP_REV, UNICODE_REV, entries, verify_sources


def generate(cmaps, unicode, output):
    verify_sources(cmaps, unicode)
    decoded = entries(unicode / 'pdf2unicode/Adobe-KR-UCS2', 'bf')
    names = [f'Adobe-KR-{n}' for n in range(10)] + ['UniAKR-UTF8-H', 'UniAKR-UTF32-H']
    rows = []
    sources = {}
    for name in names:
        path = cmaps / 'Adobe-KR-9/CMap' / name
        sources[name] = hashlib.sha256(path.read_bytes()).hexdigest()
        mapping = entries(path, 'cid')
        candidates = [(code, cid, decoded[cid.to_bytes(2, 'big')])
                      for code, cid in sorted(mapping.items())
                      if cid > 0 and cid.to_bytes(2, 'big') in decoded
                      and all(ord(c) >= 32 for c in decoded[cid.to_bytes(2, 'big')])]
        selected = [('first', candidates[0]), ('last', candidates[-1])]
        for width in range(1, 5):
            group = [row for row in candidates if len(row[0]) == width]
            if group:
                selected.append((f'{width}-byte', group[0]))
        non_bmp = [row for row in candidates if any(ord(c) > 0xffff for c in row[2])]
        if non_bmp:
            selected.append(('non-bmp', non_bmp[0]))
        supplement = name.rsplit('-', 1)[1] if name.startswith('Adobe-KR-') else '9'
        for category, (code, cid, text) in selected:
            rows.append(['KR', supplement, name, 'H', category, code.hex().upper(),
                         str(cid), ' '.join(f'{ord(c):04X}' for c in text)])
    table = 'ordering\tsupplement\tcmap\tdirection\tcategory\tcode\tcid\tunicode\n'
    table += ''.join('\t'.join(row) + '\n' for row in rows)
    output.mkdir(parents=True, exist_ok=True)
    (output / 'samples.tsv').write_text(table)
    (output / 'provenance.json').write_text(json.dumps({
        'cmap_revision': CMAP_REV, 'unicode_revision': UNICODE_REV,
        'sources_sha256': sources, 'samples': len(rows), 'maps': len(names),
        'samples_sha256': hashlib.sha256(table.encode()).hexdigest(),
        'scope': 'Additional KR maps in the fixed revision; representative decoding, not shaping.'
    }, indent=2) + '\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('cmaps', type=Path)
    parser.add_argument('unicode', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    generate(args.cmaps, args.unicode, args.output)
