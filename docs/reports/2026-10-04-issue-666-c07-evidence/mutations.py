from pathlib import Path
import os,json,subprocess,hashlib
root=Path.cwd();p=root/'docs/reports/2026-10-04-issue-666-c07-evidence'
scratch=root/'target/issue666-c07-controls'
source=(root/'oxidize-pdf-core/tests/text_cmap_boundary_contract_test.rs').read_text()
source=source.replace('#[path = "common/text_contracts.rs"]',f'#[path = "{root}/oxidize-pdf-core/tests/common/text_contracts.rs"]')
source=source.replace('"fixtures/text_contracts/fonts/ContractCID.cff"',f'"{root}/oxidize-pdf-core/tests/fixtures/text_contracts/fonts/ContractCID.cff"')
rlib=max((root/'target/debug/deps').glob('liboxidize_pdf-*.rlib'),key=lambda s:s.stat().st_mtime)
env=dict(os.environ,CARGO_MANIFEST_DIR=str(root/'oxidize-pdf-core'))
variants={
 'control':source,
 'notdef-cid-input':source.replace('17 endnotdefrange','29 endnotdefrange'),
 'missing-parent-input':source.replace('"/UseCMap 9 0 R"','""'),
 # End the test at a complete code instead of a partial tail: it must reject
 # the assumption of truncation via unwrap_err on the actual successful result.
 'complete-code-negative-control':source.replace('for length in 1..code.len()', 'for length in code.len()..=code.len()'),
}
results=[]
for name,body in variants.items():
 path=scratch/f'{name}.rs';path.write_text(body);binary=scratch/name
 cmd=['rustc','--edition=2021','--test',str(path),'--extern',f'oxidize_pdf={rlib}','-L',f'dependency={root}/target/debug/deps','-o',str(binary)]
 with (p/f'mutation-{name}.log').open('w') as log:
  build=subprocess.run(cmd,env=env,stdout=log,stderr=subprocess.STDOUT)
  result=subprocess.run([str(binary)],env=env,stdout=log,stderr=subprocess.STDOUT) if build.returncode==0 else None
 results.append({'name':name,'compile_exit':build.returncode,'test_exit':None if result is None else result.returncode,'source_sha256':hashlib.sha256(body.encode()).hexdigest(),'command':cmd})
(p/'mutations.json').write_text(json.dumps(results,indent=2)+'\n')
print([(x['name'],x['compile_exit'],x['test_exit']) for x in results])
assert [x['compile_exit'] for x in results]==[0]*4
assert [x['test_exit'] for x in results]==[0,101,101,101]
