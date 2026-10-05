"""Exercise the actual Standard14 assertion on disposable copies, old and fixed."""
import json,subprocess
from pathlib import Path
root=Path.cwd();out=root/'docs/reports/2026-10-05-666-reconciliation-evidence';work=root/'target/standard14-nan-probe';work.mkdir(exist_ok=True)
libs={n:max((root/'target/debug/deps').glob('lib'+n+'-*.rlib'),key=lambda p:p.stat().st_mtime) for n in ['oxidize_pdf','serde_json','sha2']}
results=[]
for version in ['before','after']:
    source=(out/'standard14-before.rs' if version=='before' else root/'oxidize-pdf-core/tests/text_standard14_metrics_contract_test.rs').read_text()
    for mutant in [False,True]:
        text=source.replace('PathBuf::from(env!("CARGO_MANIFEST_DIR"))','PathBuf::from("'+str(root/'oxidize-pdf-core')+'")')
        for file in ['readers.json','provenance.json']:
            text=text.replace('"fixtures/text_contracts/standard14/'+file+'"','"'+str(root/'oxidize-pdf-core/tests/fixtures/text_contracts/standard14'/file)+'"')
        if mutant:
            text=text.replace('let result = TextExtractor','let mut result = TextExtractor')
            text=text.replace('            let expected = case[','            for fragment in &mut result.fragments { fragment.x = f64::NAN; fragment.y = f64::NAN; }\n            let expected = case[')
        name=f'{version}-{mutant}';rs=work/(name+'.rs');rs.write_text(text);binary=work/(name+'-test')
        cmd=['rustc','--edition','2021','--test',str(rs),'-L','dependency=target/debug/deps','-o',str(binary)]
        for n,lib in libs.items():cmd+=['--extern',n+'='+str(lib)]
        p=subprocess.run(cmd,capture_output=True,text=True);assert p.returncode==0,p.stderr
        p=subprocess.run([str(binary),'helvetica_styles_follow_adobe_implicit_metrics','--exact'],capture_output=True,text=True)
        (out/(name+'.log')).write_text(p.stdout+p.stderr)
        assert (p.returncode!=0)==(version=='after' and mutant),(name,p.stdout,p.stderr)
        if p.returncode:assert 'NaN' in p.stdout
        results.append({'version':version,'nan_mutation':mutant,'exit':p.returncode})
(out/'nan-probe.json').write_text(json.dumps(results,indent=2)+'\n')
print('Both controls pass; NaN accepted before, rejected after.')
