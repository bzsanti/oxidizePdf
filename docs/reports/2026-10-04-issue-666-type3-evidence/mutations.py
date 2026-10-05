from pathlib import Path
import subprocess,json
root=Path.cwd();ev=root/'docs/reports/2026-10-04-issue-666-type3-evidence'; work=root/'target/issue666-type3/mutations';work.mkdir(exist_ok=True)
source=(root/'oxidize-pdf-core/tests/text_font_program_contract_test.rs').read_text().replace('common/text_contracts.rs',str(root/'oxidize-pdf-core/tests/common/text_contracts.rs'))
variants={'control':source,'fixed_scale':source.replace('fn type3_document(matrix: [f64; 6], unicode: bool, content: &[u8]) -> Vec<u8> {','fn type3_document(mut matrix: [f64; 6], unicode: bool, content: &[u8]) -> Vec<u8> {\n matrix[0]=0.001;'),'drop_translation':source.replace('fn type3_document(matrix: [f64; 6], unicode: bool, content: &[u8]) -> Vec<u8> {','fn type3_document(mut matrix: [f64; 6], unicode: bool, content: &[u8]) -> Vec<u8> {\n matrix[4]=0.0; matrix[5]=0.0;'),'double_tj':source.replace('[(A) 100] TJ','[(A) 200] TJ')}
results=[]
for name,code in variants.items():
 p=work/(name+'.rs');p.write_text(code); binary=work/name
 subprocess.run(['rustc','--edition=2021','--test','--crate-name','type3_contract_probe',str(p),'-L','dependency=target/debug/deps','--extern','oxidize_pdf=target/debug/deps/liboxidize_pdf-8d56b1627bc9922c.rlib','-o',str(binary)],check=True)
 r=subprocess.run([str(binary)],capture_output=True,text=True);ev.joinpath('mutation-'+name+'.log').write_text(r.stdout+r.stderr)
 assert (r.returncode==0)==(name=='control'),name
 results.append({'variant':name,'exit':r.returncode,'scope':'fixture input perturbation, production unchanged'})
ev.joinpath('mutations.json').write_text(json.dumps(results,indent=2)+'\n')
