"""Validate independently generated contracts and compare timing-output rasters.
Usage: python3 validate_outputs.py CONTRACT_PDFS TIMING_RESULTS
"""
import hashlib
import json
import subprocess
import sys
from pathlib import Path

contracts, timing = map(Path, sys.argv[1:])
rows = []
for pdf in sorted(contracts.glob('*.pdf')):
    subprocess.run(['qpdf', '--check', str(pdf)], check=True, capture_output=True)
    assert int(subprocess.check_output(['qpdf', '--show-npages', str(pdf)])) == 2
    result = subprocess.run(['pdftotext', '-layout', str(pdf), '-'], check=True, capture_output=True, text=True)
    assert not result.stderr, result.stderr
    lines = [line.strip() for line in result.stdout.replace('\f', '\n').splitlines() if line.strip()]
    expected = (['中文', '测试'] if pdf.name.startswith('cjk') else
                [f'Sample{i}' for _ in range(2) for i in range(12)] if pdf.name.startswith('raw') else
                ['Alpha', 'Omega'])
    assert lines == expected, (pdf, lines)
    rows.append({'file': pdf.name, 'exact_text': lines, 'sha256': hashlib.sha256(pdf.read_bytes()).hexdigest()})
assert len(rows) == 8
for mode in ['default', 'raw']:
    for pages in [1, 10, 100]:
        images = []
        for variant in ['control', 'shared']:
            result = subprocess.run(['pdftoppm', '-f', '1', '-singlefile', '-scale-to', '1000', '-gray', str(timing / f'{variant}-{mode}-{pages}.pdf')], check=True, capture_output=True)
            assert not result.stderr, result.stderr
            images.append(result.stdout)
        assert images[0] == images[1], (mode, pages)
print(json.dumps({'contracts': rows, 'identical_raster_pairs': 6}, indent=2))
