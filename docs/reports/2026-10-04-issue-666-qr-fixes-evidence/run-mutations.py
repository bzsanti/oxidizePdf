from pathlib import Path
import json,os,subprocess
root=Path.cwd()
evidence=root/'docs/reports/2026-10-04-issue-666-qr-fixes-evidence'
scratch=root/'target/issue666-qr-fix-mutations'
scratch.mkdir(exist_ok=True)
rlib=max((root/'target/debug/deps').glob('liboxidize_pdf-*.rlib'),key=lambda p:p.stat().st_mtime)
env=dict(os.environ,CARGO_MANIFEST_DIR=str(root/'oxidize-pdf-core'))
results=[]

def check(name,source,filters=()):
    path=evidence/f'{name}.rs'
    path.write_text(source)
    binary=scratch/name
    cmd=['rustc','--edition=2021','--test','-A','dead_code','-A','unused_variables',str(path),'--extern',f'oxidize_pdf={rlib}','-L',f'dependency={root}/target/debug/deps','-o',str(binary)]
    for dependency in ['sha2', 'serde_json']:
        library=max((root/'target/debug/deps').glob(f'lib{dependency}-*.rlib'), key=lambda p:p.stat().st_mtime)
        cmd.extend(['--extern', f'{dependency}={library}'])
    with (evidence/f'{name}.log').open('w') as log:
        compiled=subprocess.run(cmd,env=env,stdout=log,stderr=subprocess.STDOUT)
        run=subprocess.run([str(binary),*filters],env=env,stdout=log,stderr=subprocess.STDOUT) if compiled.returncode==0 else None
    results.append({'name':name,'compile_exit':compiled.returncode,'run_exit':None if run is None else run.returncode,'command':cmd})

text=(root/'oxidize-pdf-core/tests/text_transform_contract_test.rs').read_text()
text=text.replace('#[path = "common/text_contracts.rs"]',f'#[path = "{root}/oxidize-pdf-core/tests/common/text_contracts.rs"]')
check('nan-control',text)
text=text.replace('let result = TextExtractor','let mut result = TextExtractor').replace('    assert_eq!(\n        result.fragments.len(),','    for fragment in &mut result.fragments { fragment.x=f64::NAN; fragment.y=f64::NAN; }\n    assert_eq!(\n        result.fragments.len(),',1)
check('nan-mutation',text)
text=(root/'oxidize-pdf-core/tests/text_composite_font_contract_test.rs').read_text().replace('"fixtures/text_contracts/',f'"{root}/oxidize-pdf-core/tests/fixtures/text_contracts/')
check('gid-control',text)
text=text.replace('let glyphs = font.decode_glyphs(codes).unwrap();','let mut glyphs = font.decode_glyphs(codes).unwrap();\n                for glyph in &mut glyphs { glyph.gid = glyph.cid.and_then(|cid| u16::try_from(cid).ok()); }')
check('gid-identity-mutation',text)
original=root/'oxidize-pdf-core/src/text/fonts/cff'
text=(original/'dict.rs').read_text()
mutated=scratch/'dict-mutated.rs'
mutated.write_text(text.replace('Ok(cff[offset + 1..offset + 1 + num_glyphs].to_vec())','Ok(vec![0; num_glyphs])').replace('result[gid] = fd_idx;','result[gid] = 0;'))
wrapper='''pub mod parser { pub use oxidize_pdf::parser::*; }
mod text { pub mod fonts { pub mod cff {
#[path = "TYPES"] pub mod types;
#[path = "INDEX"] pub mod index;
#[path = "DICT"] mod dict;
}}}
'''.replace('TYPES',str(original/'types.rs')).replace('INDEX',str(original/'index.rs'))
check('fd-control',wrapper.replace('DICT',str(original/'dict.rs')),['contract_fd_selection'])
check('fd-zero-mutation',wrapper.replace('DICT',str(mutated)),['contract_fd_selection'])
(evidence/'mutations.json').write_text(json.dumps(results,indent=2)+'\n')
print([(r['name'],r['compile_exit'],r['run_exit']) for r in results])
assert all(r['compile_exit']==0 for r in results)
assert [r['run_exit'] for r in results]==[0,101,0,101,0,101]
