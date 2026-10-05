"""Discriminating input mutations on disposable copies, never the candidate."""
import json
import shutil
import subprocess
import sys
from pathlib import Path
sys.path.insert(0, 'target/issue666-fonttools')
from fontTools.ttLib import TTFont
root = Path.cwd()
work = root / 'target/f07-mutations'
work.mkdir(exist_ok=True)
out = root / 'docs/reports/2026-10-04-f07-evidence'
original = root / 'oxidize-pdf-core/tests/fixtures/text_contracts/symbolic'
source = (root / 'oxidize-pdf-core/tests/text_symbolic_truetype_contract_test.rs').read_text()
source = source.replace('common/text_contracts.rs', str(root / 'oxidize-pdf-core/tests/common/text_contracts.rs'))
source = source.replace('"fixtures/text_contracts/symbolic/provenance.json"', '"'+str(original / 'provenance.json')+'"')
rlib = max((root / 'target/debug/deps').glob('liboxidize_pdf-*.rlib'), key=lambda p:p.stat().st_mtime)
results=[]
for name in ['control', 'glyph-selection', 'outline', 'explicit-encoding']:
    fixtures = work / name
    fixtures.mkdir(exist_ok=True)
    for path in original.glob('*.ttf'):
        shutil.copyfile(path, fixtures / path.name)
    font_path = fixtures / 'symbol-full.ttf'
    if name in ['glyph-selection', 'outline']:
        font = TTFont(font_path, recalcTimestamp=False)
        if name == 'glyph-selection':
            font['cmap'].tables[0].cmap[0xf041] = 'A'
        else:
            coordinates = font['glyf']['B'].coordinates
            for i, (x,y) in enumerate(coordinates):
                if x == 650: coordinates[i] = (550,y)
        font.save(font_path)
    text = source.replace('PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_contracts/symbolic")', 'PathBuf::from("'+str(fixtures)+'")')
    if name == 'explicit-encoding':
        text = text.replace('/BaseFont /ContractSymbolic {encoding}', '/BaseFont /ContractSymbolic /Encoding /WinAnsiEncoding {encoding}')
    path = work / (name + '.rs')
    path.write_text(text)
    binary = work / (name + '-test')
    build = subprocess.run(['rustc','--edition','2021','--test',str(path),'-L','dependency=target/debug/deps','--extern','oxidize_pdf='+str(rlib),'--extern','serde_json='+str(max((root/'target/debug/deps').glob('libserde_json-*.rlib'),key=lambda p:p.stat().st_mtime)),'--extern','sha2='+str(max((root/'target/debug/deps').glob('libsha2-*.rlib'),key=lambda p:p.stat().st_mtime)),'-o',str(binary)],capture_output=True,text=True)
    (out/(name+'-build.log')).write_text(build.stdout+build.stderr)
    assert build.returncode == 0, build.stderr
    target = 'arbitrary_symbolic_text_uses_replacement_without_losing_advance' if name == 'explicit-encoding' else 'full_and_subset_preserve_selected_symbolic_contours'
    result = subprocess.run([str(binary),target,'--exact'],capture_output=True,text=True)
    (out/(name+'-mutation.log')).write_text(result.stdout+result.stderr)
    assert (result.returncode == 0) == (name == 'control'), (name,result.stdout,result.stderr)
    results.append({'case':name,'test':target,'exit':result.returncode})
(out/'mutations.json').write_text(json.dumps(results,indent=2)+'\n')
print('Control passes; three mutations fail at intended assertions.')
