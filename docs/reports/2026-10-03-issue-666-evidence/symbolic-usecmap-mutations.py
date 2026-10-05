"""Input discrimination probes; original tests/fixtures/product remain unchanged."""
from pathlib import Path
import hashlib,json,os,subprocess
root=Path.cwd()
evidence=root/'docs/reports/2026-10-03-issue-666-evidence'
probe=root/'target/issue666-symbolic-usecmap-mutations'
probe.mkdir(parents=True,exist_ok=True)
tests=root/'oxidize-pdf-core/tests'
originals={name:tests/f'text_{name}_contract_test.rs' for name in ['symbolic_truetype','usecmap']}
hashes={name:hashlib.sha256(path.read_bytes()).hexdigest() for name,path in originals.items()}
helper=r'''
fn mutated(input: Vec<u8>) -> Vec<u8> {
    let (before, after): (&[u8], &[u8]) = match std::env::var("ISSUE666_INPUT_MUTATION").as_deref() {
        Ok("unicode") => (b"<00660069>", b"<0066006A>"),
        Ok("advance") => (b"/Widths [700 400]", b"/Widths [900 400]"),
        Ok("cid-width") => (b"17 [400]", b"17 [900]"),
        _ => return input,
    };
    assert_eq!(before.len(), after.len());
    let at = input.windows(before.len()).position(|part| part == before).expect("mutation reaches fixture");
    let mut bytes = input;
    bytes[at..at + after.len()].copy_from_slice(after);
    bytes
}
'''
for name,path in originals.items():
 source=path.read_text().replace('env!("CARGO_MANIFEST_DIR")',json.dumps(str(root/'oxidize-pdf-core')))
 source=source.replace('"common/text_contracts.rs"',json.dumps(str(tests/'common/text_contracts.rs')))
 source=source.replace('"fixtures/text_contracts/', '"'+str(tests/'fixtures/text_contracts')+'/')
 if name=='symbolic_truetype':
  source=source.replace('std::fs::read(root().join(name)).unwrap()', 'mutated(std::fs::read(root().join(name)).unwrap())')
 else:
  source=source.replace('Cursor::new(bytes.clone())','Cursor::new(mutated(bytes.clone()))')
 (probe/f'{name}.rs').write_text(source+helper)
manifest=f'''[package]
name = "issue666-symbolic-usecmap-mutations"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
oxidize-pdf = {{ path = {json.dumps(str(root/'oxidize-pdf-core'))} }}
sha2 = "0.10"
serde_json = "1"
'''
for name in originals:
 manifest+=f'[[test]]\nname = "{name}"\npath = "{name}.rs"\n'
(probe/'Cargo.toml').write_text(manifest)
(probe/'Cargo.lock').write_bytes((root/'Cargo.lock').read_bytes())
rows=[]
for suite,mutation,test,marker in [
 ('symbolic_truetype','unicode','symbolic_tounicode_preserves_sequences_independently_of_glyph_names','fj😀'),
 ('symbolic_truetype','advance','multi_scalar_tounicode_does_not_multiply_the_source_glyph_advance','got (109,700)'),
 ('usecmap','cid-width','flat_maps_are_the_control','expected second origin 104, got 109')]:
 for mutate in [False,True]:
  env=os.environ.copy(); env['ISSUE666_INPUT_MUTATION']=mutation if mutate else 'control'
  cmd=['cargo','test','--offline','--manifest-path',str(probe/'Cargo.toml'),'--target-dir',str(root/'target'),'--test',suite,test,'--','--exact']
  result=subprocess.run(cmd,env=env,capture_output=True,text=True)
  log=result.stdout+result.stderr
  filename=f'{suite}-mutation-{mutation}-{mutate}.log'
  (evidence/filename).write_text(log)
  assert result.returncode==(101 if mutate else 0),log
  if mutate: assert marker in log,log
  rows.append({'suite':suite,'mutation':mutation,'mutated':mutate,'exit_code':result.returncode,'assertion':marker if mutate else None,'log':filename})
assert all(hashlib.sha256(path.read_bytes()).hexdigest()==hashes[name] for name,path in originals.items())
(evidence/'symbolic-usecmap-mutations.json').write_text(json.dumps({'scope':'input discrimination, not product mutation coverage','originals_unchanged':True,'runs':rows},indent=2)+'\n')
print('3 controls pass, 3 intended input mutations detected')
