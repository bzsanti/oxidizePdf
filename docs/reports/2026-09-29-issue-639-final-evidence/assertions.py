import json, subprocess
from pathlib import Path
root=Path('/tmp/issue639-final-validation')
results=json.loads((root/'checks.json').read_text())
byname={r['file']:r for r in results}
for name in ['flat','nested','generation','compressed','rich']:
    source=byname[name+'.pdf']
    for method in range(3):
        for replace in ['false','true']:
            key=f'{name}-{method}-{replace}.pdf'
            result=byname[key]
            assert result['qpdf_exit']==0 and not result['text_stderr'],key
            assert (root/key).read_bytes().startswith((root/(name+'.pdf')).read_bytes()),key
            assert result['pages']==(3 if method==0 else 2),key
            for field in source['catalog']:
                if field == '/Pages': continue
                assert result['catalog'].get(field)==source['catalog'].get(field),(key,field)
            if replace=='false': assert result['info']==source['info'],key
            assert result['page_metadata'][:2]==source['page_metadata'],key
            assert set(result['page_parents'])=={result['catalog']['/Pages']},key
            assert 'OriginalTwo' in result['text'],key
            assert ('OriginalOne' in result['text'])==(method!=1),key
            assert ('Replacement' in result['text'])==(method!=2),key
            assert ('Overlay' in result['text'])==(method==2),key
            metadata=source['catalog']['/Metadata'].split()[0]
            def xmp(p):
                return subprocess.run(['qpdf','--show-object='+metadata,'--filtered-stream-data',str(p)],capture_output=True,check=True).stdout
            assert xmp(root/key)==xmp(root/(name+'.pdf')),key
for compressed in [False,True]:
    for xref in [False,True]:
        key=f'config-{str(compressed).lower()}-{str(xref).lower()}.pdf'
        r=byname[key]
        assert r['qpdf_exit']==0 and r['info'],key
        bits=int(r['info']['/oxidize-pdf-features'][2:],16)
        assert bool(bits&0x200)==compressed and bool(bits&0x400)==xref,key
print('PASS: 30 incremental outputs, 4 writer configurations; qpdf, Poppler, prefix, Info, catalog/page metadata, XMP bytes, parent links and content.')
