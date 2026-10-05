"""Perturb independent font input, without modifying the product or frozen fixture."""
import io
import json
from pathlib import Path
import subprocess
from fontTools.ttLib import TTFont

root = Path.cwd()
evidence = root / 'docs/reports/2026-10-04-issue-666-continuation-evidence'
work = root / 'target/issue666-competing-mutations'
work.mkdir(exist_ok=True)
fixture = root / 'oxidize-pdf-core/tests/fixtures/text_contracts/symbolic/competing'
source = (root / 'oxidize-pdf-core/tests/text_competing_cmap_contract_test.rs').read_text()
results = []
for variant in ['control', 'wrong-full-gid', 'missing-full-table']:
    font = TTFont(io.BytesIO((fixture / 'competing.ttf').read_bytes()), recalcTimestamp=False)
    if variant == 'wrong-full-gid':
        font['cmap'].tables[-1].cmap[65] = 'A'
    elif variant == 'missing-full-table':
        font['cmap'].tables.pop()
    program = work / f'{variant}.ttf'
    font.save(program)
    test = work / f'{variant}.rs'
    test.write_text(source.replace('fixtures/text_contracts/symbolic/competing/competing.ttf', str(program)).replace('fixtures/text_contracts/symbolic/competing/provenance.json', str(fixture / 'provenance.json')))
    binary = work / variant
    cmd = ['rustc', '--edition=2021', '--test', '--crate-name', 'competing_probe', str(test), '-L', 'dependency=target/debug/deps', '-o', str(binary)]
    for crate in ['oxidize_pdf', 'sha2', 'serde_json']:
        library = max((root/'target/debug/deps').glob(f'lib{crate}-*.rlib'), key=lambda p:p.stat().st_mtime)
        cmd += ['--extern', f'{crate}={library}']
    subprocess.run(cmd, check=True)
    result = subprocess.run([str(binary)], capture_output=True, text=True)
    (evidence / f'mutation-{variant}.log').write_text(result.stdout + result.stderr)
    assert (result.returncode == 0) == (variant == 'control')
    results.append({'variant':variant, 'exit_code':result.returncode, 'scope':'font input perturbation, not production branch mutation'})
(evidence / 'mutations.json').write_text(json.dumps(results, indent=2)+'\n')
