"""Discriminating checks without modifying the candidate or historical evidence."""
import json
import subprocess
from pathlib import Path

root = Path.cwd()
evidence = root / 'docs/reports/2026-10-04-f08-fixes-evidence'
work = root / 'target/f08-fix-mutations'
work.mkdir(exist_ok=True)
results = []
def run(name, command, expected):
    result = subprocess.run(command, capture_output=True, text=True)
    (evidence / (name + '.log')).write_text(result.stdout + result.stderr)
    results.append({'name': name, 'command': [str(x) for x in command], 'exit': result.returncode, 'expected': expected})
    assert (result.returncode == 0) == (expected == 0), (name, result.returncode)

reader = (root / 'docs/reports/2026-10-04-issue-666-type3-program-evidence/readers.py').read_text()
for name, script, expected in [
    ('reader-control', reader, 0),
    ('reader-width-mutation', reader.replace("'fractional',125.5,'125.5 0 d0'", "'fractional',250.5,'250.5 0 d0'"), 1),
    ('reader-version-mutation', reader.replace("('1.26.5', '1.26.10')", "('0.0.0', '1.26.10')"), 1),
]:
    script = script.replace("target/f08-fixed-readers", f"target/f08-fix-mutations/{name}")
    path = work / (name + '.py')
    path.write_text(script)
    run(name, ['python3', path, evidence / (name + '.json')], expected)

source = (root / 'oxidize-pdf-core/tests/text_type3_program_boundary_contract_test.rs').read_text()
source = source.replace('common/text_contracts.rs', str(root / 'oxidize-pdf-core/tests/common/text_contracts.rs'))
rlib = max((root / 'target/debug/deps').glob('liboxidize_pdf-*.rlib'), key=lambda p: p.stat().st_mtime)
for name, text, expected in [
    ('operations-control', source, 0),
    ('operations-mutation', source.replace('0 0 400 600 re f', '1 1 m S'), 101),
]:
    path = work / (name + '.rs')
    binary = work / name
    path.write_text(text)
    run(name + '-build', ['rustc', '--edition', '2021', '--test', path, '-L', 'dependency=target/debug/deps', '--extern', 'oxidize_pdf=' + str(rlib), '-o', binary], 0)
    run(name, [binary, 'consistent_d0_and_d1_preserve_metrics_and_program_operations', '--exact'], expected)
(evidence / 'mutations.json').write_text(json.dumps(results, indent=2) + '\n')
print('All controls pass; graphical, fractional-width and version mutations rejected.')
