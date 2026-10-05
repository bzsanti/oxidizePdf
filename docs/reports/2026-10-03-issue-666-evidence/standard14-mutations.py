"""Immutable assertion copy; three input mutations, never edit canonical PDFs/product."""
from pathlib import Path
import hashlib,json,os,subprocess
root=Path.cwd();evidence=root/'docs/reports/2026-10-03-issue-666-evidence';probe=root/'target/issue666-standard14-mutations';probe.mkdir(parents=True,exist_ok=True)
original=root/'oxidize-pdf-core/tests/text_standard14_metrics_contract_test.rs';digest=hashlib.sha256(original.read_bytes()).hexdigest()
s=original.read_text().replace('env!("CARGO_MANIFEST_DIR")',json.dumps(str(root/'oxidize-pdf-core'))).replace('"fixtures/text_contracts/', '"'+str(root/'oxidize-pdf-core/tests/fixtures/text_contracts')+'/')
needle='Cursor::new(std::fs::read(root().join(path)).unwrap())';assert needle in s
s=s.replace(needle,'Cursor::new(mutated(std::fs::read(root().join(path)).unwrap()))')
s+=r'''
fn mutated(mut bytes: Vec<u8>) -> Vec<u8> {
    let (before,after):(&[u8],&[u8])=match std::env::var("ISSUE666_INPUT_MUTATION").as_deref() {
        Ok("width")=>(b"391",b"791"),
        Ok("code")=>(b"<41>",b"<42>"),
        Ok("font")=>(b"/F1 10 Tf",b"/F0 10 Tf"),
        _=>return bytes,
    };
    let positions:Vec<_>=bytes.windows(before.len()).enumerate().filter_map(|(i,s)|(s==before).then_some(i)).collect();
    assert!(!positions.is_empty(),"mutation reaches fixture");
    for i in positions { bytes[i..i+after.len()].copy_from_slice(after); }
    bytes
}
'''
(probe/'contracts.rs').write_text(s)
(probe/'Cargo.toml').write_text(f'''[package]
name="issue666-standard14-mutations"
version="0.0.0"
edition="2021"
[workspace]
[dependencies]
oxidize-pdf={{path={json.dumps(str(root/'oxidize-pdf-core'))}}}
sha2="0.10"
serde_json="1"
[[test]]
name="contracts"
path="contracts.rs"
''');(probe/'Cargo.lock').write_bytes((root/'Cargo.lock').read_bytes());rows=[]
for mutation,test,marker in [('width','declared_widths_override_implicit_metrics_for_all_fourteen_fonts','expected'),('code','helvetica_styles_follow_adobe_implicit_metrics','got "B"'),('font','every_font_pair_and_return_transition_selects_the_current_metrics','-to-Helvetica')]:
 for mutate in [False,True]:
  env=os.environ.copy();env['ISSUE666_INPUT_MUTATION']=mutation if mutate else 'control'
  r=subprocess.run(['cargo','test','--offline','--manifest-path',str(probe/'Cargo.toml'),'--target-dir',str(root/'target'),'--test','contracts',test,'--','--exact'],env=env,capture_output=True,text=True)
  log=r.stdout+r.stderr;filename=f'standard14-mutation-{mutation}-{mutate}.log';(evidence/filename).write_text(log)
  assert r.returncode==(101 if mutate else 0),log
  if mutate:assert marker in log and 'panicked at' in log,log
  rows.append({'mutation':mutation,'mutated':mutate,'exit_code':r.returncode,'log':filename})
assert hashlib.sha256(original.read_bytes()).hexdigest()==digest
(evidence/'standard14-mutations.json').write_text(json.dumps({'scope':'input discrimination, not product mutation coverage','original_unchanged':True,'runs':rows},indent=2)+'\n')
print('3 controls pass;3 input mutations detected')
