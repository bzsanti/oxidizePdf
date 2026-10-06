"""Run copied-source measurement mutations; never modify the candidate."""
from pathlib import Path
import hashlib
import json
import subprocess

root = Path(__file__).resolve().parents[3]
out = Path(__file__).resolve().parent / 'final-mutations'
out.mkdir(exist_ok=True)
work = root / 'target/t4-content-mutations-final'
work.mkdir(exist_ok=True)
source = root / 'tools/text_contracts/content_differential.py'
test = root / 'tools/tests/test_content_differential.py'
original = source.read_text()
test_source = test.read_text()
mutations = {
    'control': [],
    'autojunk': [('autojunk=False).get_opcodes()', 'autojunk=True).get_opcodes()')],
    'case-folding': [('ref = "".join(c for c in reference if not c.isspace())',
                      'reference, candidate = reference.lower(), candidate.lower()\n    ref = "".join(c for c in reference if not c.isspace())')],
    'bytes-not-scalars': [('reference_scalars=len(ref), candidate_scalars=len(got)',
                          'reference_scalars=len(ref.encode("utf-8")), candidate_scalars=len(got)')],
    'invented-denominator': [('if denominator else None', 'if denominator else 0.0')],
    'unverified-input': [('if sha256(path) != entry["sha256"]:', 'if False:')],
    'positional-page-alignment': [('if len(pages) != len(reference):', 'if False:'),
        ('for index in range(max(len(pages), len(reference))):',
         'for index in range(min(len(pages), len(reference))):')],
}
results = []
for name, edits in mutations.items():
    text = original
    for before, after in edits:
        assert text.count(before) == 1, (name, before)
        text = text.replace(before, after)
    module = work / (name + '.py')
    module.write_text(text)
    runner = work / (name + '_test.py')
    runner.write_text(test_source.replace(
        'ROOT = Path(__file__).resolve().parents[2]', 'ROOT = Path(' + repr(str(root)) + ')').replace(
        'ROOT / "tools/text_contracts/content_differential.py"', 'Path(' + repr(str(module)) + ')'))
    run = subprocess.run(['python3', str(runner), '-v'], capture_output=True, text=True, timeout=30)
    (out / (name + '.log')).write_text(run.stdout + run.stderr)
    assert run.returncode == (0 if name == 'control' else 1), (name, run.stderr)
    if name != 'control':
        assert 'FAIL:' in run.stderr and 'ERROR:' not in run.stderr, (name, run.stderr)
    results.append(dict(case=name, exit=run.returncode,
                        source_sha256=hashlib.sha256(text.encode()).hexdigest()))
assert source.read_text() == original
(out / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
print('Control PASS; six mutations fail at assertions; candidate unchanged.')
