"""Check product annotations against unchanged pinned Adobe resource headers."""
import hashlib,json,re
from pathlib import Path
pins=json.loads(Path('tools/adobe_cjk_source_pins.json').read_text())['sources']
source=Path('oxidize-pdf-core/src/text/encoding_cmap.rs').read_text()
annotations=re.findall(r'vendored_cmap!\("([^"]+)",\s*"([^"]+)"\)',source)
assert len(annotations)==47,len(annotations)
rows=[]
for name,ordering in annotations:
    raw=Path('oxidize-pdf-core/src/text/cmap_resources',name).read_bytes()
    expected=[value for path,value in pins.items() if path.endswith('/CMap/'+name)]
    assert len(expected)==1
    assert hashlib.sha256(raw).hexdigest()==expected[0],name
    text=raw.decode('ascii');registry=re.search(r'/Registry\s+\(([^)]+)\)',text)[1];actual=re.search(r'/Ordering\s+\(([^)]+)\)',text)[1]
    assert (registry,actual)==('Adobe',ordering),(name,actual,ordering)
    rows.append({'cmap':name,'registry':registry,'ordering':actual,'sha256':expected[0]})
Path(__file__).with_suffix('.json').write_text(json.dumps(rows,indent=2)+'\n')
print('47 collection annotations match pinned, unchanged Adobe headers')
