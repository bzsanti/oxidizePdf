"""Independent qpdf comparison of generated objects and decoded stream bytes."""
import base64, json, re, subprocess
from pathlib import Path
root = Path(__file__).parent
results = []
for mode, kind in [('compressed', 'mixed'), ('raw', 'mixed'), ('compressed', 'text')]:
    documents = []
    ignored = []
    for label in ('baseline', 'candidate'):
        output = root / f'{label}-{mode}-{kind}.pdf'
        subprocess.run([str(root/label), 'generate', '100', '50', '1', mode, kind, str(output)], check=True, capture_output=True)
        check = subprocess.run(['qpdf', '--check', str(output)], text=True, capture_output=True)
        if check.returncode != 0:
            raise RuntimeError(check.stdout + check.stderr)
        document = json.loads(subprocess.check_output(['qpdf', '--json', '--json-stream-data=inline', str(output)]))
        # Only clock metadata varies between sequential executions. All object
        # identities, dictionaries, resources, pages and decoded streams remain.
        info_ref = document['qpdf'][1]['trailer']['value'].get('/Info')
        info = document['qpdf'][1]['obj:' + info_ref]['value'] if info_ref else {}
        ignored.append({key: info.pop(key) for key in ('/CreationDate', '/ModDate') if key in info})
        for obj in document['qpdf'][1].values():
            stream = obj.get('stream', {})
            if stream.get('dict', {}).get('/Type') == '/Metadata':
                raw = base64.b64decode(stream['data']).decode('utf-8')
                raw = re.sub(r'(<xmp:(?:CreateDate|ModifyDate)>)[^<]*(</xmp:(?:CreateDate|ModifyDate)>)', r'\1CLOCK\2', raw)
                stream['data'] = base64.b64encode(raw.encode('utf-8')).decode('ascii')
        documents.append(document)
    equal = documents[0] == documents[1]
    results.append({'mode': mode, 'kind': kind, 'all_objects_and_decoded_streams_equal': equal,
                    'qpdf_check': 'PASS', 'excluded_clock_metadata': ignored})
    if not equal:
        raise AssertionError(f'{mode}/{kind}: independently decoded PDF objects differ')
print(json.dumps(results, indent=2))
