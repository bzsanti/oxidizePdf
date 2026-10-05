"""Input mutations on copies, with an unchanged candidate and frozen fixtures."""
import json,subprocess,sys
from pathlib import Path
sys.path[:0]=['target/issue666-fonttools']
from fontTools.cffLib import CFFFontSet
from fontTools.ttLib import TTFont
import io
root=Path.cwd();work=root/'target/f09-mutations';work.mkdir(exist_ok=True)
out=root/'docs/reports/2026-10-04-f09-evidence'
source=(root/'oxidize-pdf-core/tests/text_cff_program_contract_test.rs').read_text()
source=source.replace('"fixtures/text_contracts/cff_selection/manifest.json"','"'+str(root/'oxidize-pdf-core/tests/fixtures/text_contracts/cff_selection/manifest.json')+'"').replace('"fixtures/text_contracts/fonts/SourceSans3-Regular.otf"','"'+str(root/'oxidize-pdf-core/tests/fixtures/text_contracts/fonts/SourceSans3-Regular.otf')+'"')
libs={name:max((root/'target/debug/deps').glob('lib'+name+'-*.rlib'),key=lambda p:p.stat().st_mtime) for name in ['oxidize_pdf','serde_json','sha2']}
fixture_root=root/'oxidize-pdf-core/tests/fixtures/text_contracts/composite';results=[]
for variant in ['control','charset','fdselect','charstring']:
    folder=work/variant;folder.mkdir(exist_ok=True)
    for path in fixture_root.glob('*.cff'):(folder/path.name).write_bytes(path.read_bytes())
    path=folder/'cff-fd0-full.cff'
    if variant!='control':
        cff=CFFFontSet();cff.decompile(io.BytesIO(path.read_bytes()),None);top=cff.topDictIndex[0]
        if variant=='charset':
            old=list(top.charset);new=list(old);new[1],new[2]=new[2],new[1]
            top.CharStrings.charStrings={b:top.CharStrings.charStrings[a] for a,b in zip(old,new)};top.charset=new
        elif variant=='fdselect':top.FDSelect.gidArray[2]=0
        else:
            glyph=top.CharStrings['cid00017'];glyph.decompile();glyph.program[0]+=1
        saved=io.BytesIO();cff.compile(saved,TTFont(recalcBBoxes=False));path.write_bytes(saved.getvalue())
    # Only run the live-parser assertion, not the frozen-hash test.
    text=source.replace('PathBuf::from(env!("CARGO_MANIFEST_DIR"))','PathBuf::from("'+str(root/'oxidize-pdf-core')+'")')
    text=text.replace('.join("tests/fixtures/text_contracts/composite")',' .join("'+str(folder)+'")')
    path=work/(variant+'.rs');path.write_text(text);binary=work/(variant+'-test')
    cmd=['rustc','--edition','2021','--test',str(path),'-L','dependency=target/debug/deps','-o',str(binary)]
    for name,lib in libs.items():cmd+=['--extern',name+'='+str(lib)]
    build=subprocess.run(cmd,capture_output=True,text=True);(out/(variant+'-build.log')).write_text(build.stdout+build.stderr);assert build.returncode==0,build.stderr
    result=subprocess.run([str(binary),'real_full_and_subset_cff_select_distinct_font_dicts_and_charstrings','--exact'],capture_output=True,text=True)
    (out/(variant+'-mutation.log')).write_text(result.stdout+result.stderr)
    assert (result.returncode==0)==(variant=='control'),(variant,result.stdout,result.stderr)
    results.append({'mutation':variant,'exit':result.returncode})
(out/'mutations.json').write_text(json.dumps(results,indent=2)+'\n')
print('Control passes; charset, FDSelect and CharString mutations fail.')
