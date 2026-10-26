import hashlib
import json
import pathlib
import subprocess
import xml.etree.ElementTree as ET

p = pathlib.Path(__file__).resolve().parent
rows = []
for kind in ['latin', 'ttf', 'cjk']:
    for compression in ['default', 'raw']:
        snapshots = []
        for variant in ['control', 'candidate']:
            pdf = p / f'unicode-validation-{variant}-{kind}-{compression}.pdf'
            subprocess.run([str(p / f'unicode-{variant}'), kind, '10', '1', str(pdf), 'full', compression], check=True, stdout=subprocess.DEVNULL)
            check = subprocess.run(['qpdf', '--check', str(pdf)], capture_output=True, text=True)
            assert check.returncode == 0, check.stdout + check.stderr
            doc = json.loads(subprocess.check_output(['qpdf', '--json', '--json-stream-data=inline', str(pdf)]))
            objects = doc['qpdf'][1]
            # XMP timestamps vary with generation time; compare every payload
            # stream, including embedded fonts, CMaps and page contents.
            streams = {key: value['stream'] for key, value in objects.items()
                if 'stream' in value and value['stream']['dict'].get('/Type') != '/Metadata'}
            widths = {key: {name: value['value'][name] for name in ['/W', '/DW'] if name in value['value']}
                for key, value in objects.items() if isinstance(value.get('value'), dict) and '/W' in value['value']}
            bbox = ET.fromstring(subprocess.check_output(['pdftotext', '-bbox', str(pdf), '-']))
            pages = [node for node in bbox.iter() if node.tag.endswith('}page')]
            assert len(pages) == 10
            words = [(word.text, word.attrib) for node in pages for word in node if word.tag.endswith('}word')]
            line = '中文测试 中文测试 中文测试' if kind == 'cjk' else 'Español: acción, mañana; Français: été, cœur; café 25 €.'
            expected = ''.join(line.split()) * 180
            actual = ''.join(''.join((word or '').split()) for word, _ in words)
            assert actual == expected, (kind, variant, actual[:100])
            snapshots.append((streams, widths, words, [page.attrib for page in pages]))
        assert snapshots[0] == snapshots[1], (kind, compression)
        rows.append(dict(kind=kind, compression=compression, pages=10,
            qpdf_exit=0, streams_identical=True, widths_identical=True, bbox_identical=True,
            expected_unicode_text=True, stream_count=len(snapshots[0][0]),
            bbox_sha256=hashlib.sha256(json.dumps(snapshots[0][2], sort_keys=True).encode()).hexdigest()))
(p / 'unicode-external-validation.json').write_text(json.dumps(rows, indent=2))
print(json.dumps(rows, indent=2))
