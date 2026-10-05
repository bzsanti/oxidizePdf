"""Mutate disposable PDF copies; assert live public API checks reject changes."""
import json,subprocess
from pathlib import Path
root=Path.cwd();work=root/'target/f10-mutations';work.mkdir(exist_ok=True)
out=root/'docs/reports/2026-10-05-f10-evidence';fixtures=root/'oxidize-pdf-core/tests/fixtures/text_contracts/cid_truetype'
source=(root/'oxidize-pdf-core/tests/text_cid_truetype_program_contract_test.rs').read_text()
for old,new in [('common/text_contracts.rs',str(root/'oxidize-pdf-core/tests/common/text_contracts.rs')),('fixtures/text_contracts/cid_truetype/manifest.json',str(fixtures/'manifest.json')),('fixtures/text_contracts/cid_truetype/full.ttf',str(fixtures/'full.ttf'))]:source=source.replace('"'+old+'"','"'+new+'"')
source=source.replace('PathBuf::from(env!("CARGO_MANIFEST_DIR"))','PathBuf::from("'+str(root/'oxidize-pdf-core')+'")')
libs={n:max((root/'target/debug/deps').glob('lib'+n+'-*.rlib'),key=lambda p:p.stat().st_mtime) for n in ['oxidize_pdf','serde_json','sha2','flate2']}
results=[]
for variant in ['control','gid','width','unicode']:
    folder=work/variant;folder.mkdir(exist_ok=True)
    for p in fixtures.glob('*.pdf'):(folder/p.name).write_bytes(p.read_bytes())
    path=folder/'full-private-stream-h-unicode.pdf';data=path.read_bytes()
    if variant=='gid':
        anchor=data.index(b'11 0 obj');start=data.index(b'stream\n',anchor)+7
        assert data[start+34:start+36]==b'\0\2'
        data=data[:start+35]+b'\3'+data[start+36:]
    elif variant=='width':
        assert b'17 [450]' in data;data=data.replace(b'17 [450]',b'17 [750]')
    elif variant=='unicode':
        assert b'<00660069>' in data;data=data.replace(b'<00660069>',b'<0066006A>')
    path.write_bytes(data)
    text=source.replace('.join("tests/fixtures/text_contracts/cid_truetype")','.join("'+str(folder)+'")')
    rs=work/(variant+'.rs');rs.write_text(text);binary=work/(variant+'-test')
    cmd=['rustc','--edition','2021','--test',str(rs),'-L','dependency=target/debug/deps','-o',str(binary)]
    for name,lib in libs.items():cmd+=['--extern',name+'='+str(lib)]
    result=subprocess.run(cmd,capture_output=True,text=True);(out/(variant+'-build.log')).write_text(result.stdout+result.stderr);assert result.returncode==0,result.stderr
    result=subprocess.run([str(binary),'full_subset_identity_stream_and_remapped_hv_select_real_glyphs','--exact'],capture_output=True,text=True)
    (out/(variant+'-mutation.log')).write_text(result.stdout+result.stderr)
    assert (result.returncode==0)==(variant=='control'),(variant,result.stdout,result.stderr)
    if variant!='control':assert {'gid':': GID','width':': PDF advance','unicode':': Unicode'}[variant] in result.stdout,result.stdout
    results.append({'mutation':variant,'exit':result.returncode})
(out/'mutations.json').write_text(json.dumps(results,indent=2)+'\n')
print('Control passes; GID, PDF width and Unicode mutations fail the intended live assertion.')
