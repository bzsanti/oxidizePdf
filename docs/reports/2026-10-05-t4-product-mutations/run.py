"""Mutate copied modules, run existing public-API tests, preserve the candidate.

An isolated Cargo harness under target links unchanged source/fixtures read-only.
Only lib.rs, text/mod.rs and the two mutated modules are copied. No repository
clone, checkout, worktree, source edit or historical baseline update is made.
"""
from pathlib import Path
import hashlib
import json
import os
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent / ('verified' if '--resume' in sys.argv else 'initial')
OUT.mkdir(exist_ok=True)
WORK = ROOT / 'target/t4-product-harness-validated'
CORE = WORK / 'core'
BUILD = ROOT / 'target/t4-product-build'
OWNED = {'lib.rs', 'text/mod.rs', 'text/extraction.rs', 'text/extraction_cmap.rs'}


def overlay(source, dest, prefix=''):
    dest.mkdir(parents=True, exist_ok=True)
    for entry in source.iterdir():
        relative = prefix + entry.name
        target = dest / entry.name
        if relative in OWNED:
            if target.is_symlink():
                raise RuntimeError('owned module unexpectedly links to candidate')
            target.write_bytes(entry.read_bytes())
        elif entry.is_dir() and any(p.startswith(relative + '/') for p in OWNED):
            overlay(entry, target, relative + '/')
        elif not target.exists():
            target.symlink_to(entry.resolve(), target_is_directory=entry.is_dir())


def hashes():
    return {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest()
            for folder in ['src', 'tests']
            for p in (ROOT / 'oxidize-pdf-core' / folder).rglob('*.rs')}


if '--resume' in sys.argv:
    assert WORK.is_dir() and not WORK.is_symlink()
    for name in OWNED:
        assert not (CORE / 'src' / name).is_symlink()
        assert (CORE / 'src' / name).read_bytes() == (ROOT / 'oxidize-pdf-core/src' / name).read_bytes(), name
else:
    if WORK.exists():
        raise SystemExit('Refusing to reuse a mutation harness; inspect it first.')
    WORK.mkdir(parents=True)
    CORE.mkdir()
    workspace = (ROOT / 'Cargo.toml').read_text()
    workspace = '[workspace]\nmembers = ["core"]\nresolver = "2"\n\n' + workspace[workspace.index('[workspace.package]'):]
    (WORK / 'Cargo.toml').write_text(workspace)
    shutil.copyfile(ROOT / 'Cargo.lock', WORK / 'Cargo.lock')
    for name in ['README.md', 'docs', 'tools']:
        (WORK / name).symlink_to(ROOT / name, target_is_directory=(ROOT / name).is_dir())
    for entry in (ROOT / 'oxidize-pdf-core').iterdir():
        if entry.name == 'src':
            overlay(entry, CORE / 'src')
        elif entry.name == 'Cargo.toml':
            shutil.copyfile(entry, CORE / entry.name)
        else:
            (CORE / entry.name).symlink_to(entry.resolve(), target_is_directory=entry.is_dir())
extraction = 'text/extraction.rs'
fonts = 'text/extraction_cmap.rs'
# name, module, old, new, target, exact existing test (empty means full target).
mutations = [
    ('positive-metrics', extraction, 'GlyphZeroWidthStatus::ExplicitNonZero => return None,',
     'GlyphZeroWidthStatus::ExplicitNonZero => {},', 'text_spacing_contract_test',
     'declared_positive_widths_keep_word_separators'),
    ('strict-mode', extraction, 'let allow_damaged_type0_recovery = !document.options().strict_mode;',
     'let allow_damaged_type0_recovery = true;', 'text_tracking_recovery_contract_test',
     'damaged_type0_recovery_covers_full_em_tolerance_and_mode'),
    ('descendant', extraction, '|| font.descendant_font.is_some()\n', '|| false\n',
     'text_tracking_recovery_contract_test', 'resolved_unusable_descendant_never_enables_missing_descendant_recovery'),
    ('explicit-space', extraction, 'if !has_explicit_space || !(0.85..=1.15).contains(&baseline) {',
     'if !(0.85..=1.15).contains(&baseline) {', 'text_spacing_contract_test',
     'unresolved_type0_without_space_reaches_and_rejects_the_fallback'),
    ('macroman-currency', fonts, "0xDB => '¤',", "0xDB => '€',", 'text_encoding_contract_test',
     'macroman_encoding_matches_independent_table'),
    ('differences', fonts, 'enc_dict.get("Differences")', 'enc_dict.get("IgnoredDifferences")',
     'text_encoding_contract_test', 'differences_override_the_base_encoding'),
    ('tounicode', fonts, 'font_dict.get("ToUnicode")', 'font_dict.get("IgnoredToUnicode")',
     'text_encoding_contract_test', 'tounicode_overrides_differences_and_supports_unicode_sequences'),
    ('tj-sign', extraction, 'let tx = -(adjustment as f64) / 1000.0 * state.font_size;',
     'let tx = (adjustment as f64) / 1000.0 * state.font_size;', 'text_tj_scale_contract_test',
     'forward_tj_adjustment_scales_with_tz'),
    ('fragment-scale', extraction,
     'let effective_width = text_width * (state.horizontal_scale / 100.0).abs() * x_scale;',
     'let effective_width = text_width * x_scale;', 'text_state_contract_test',
     'zero_and_reflected_horizontal_scale_reach_fragment_extent'),
]
originals = {name: (CORE / 'src' / name).read_text() for name in [extraction, fonts]}
for name, module, before, _, _, _ in mutations:
    assert originals[module].count(before) == 1, (name, originals[module].count(before))
initial = hashes()
(OUT / 'candidate-start-hashes.json').write_text(json.dumps(initial, indent=2) + '\n')
base = ['cargo', 'test', '--locked', '--offline', '--manifest-path', str(CORE / 'Cargo.toml'),
        '--target-dir', str(BUILD)]
env = dict(os.environ, CARGO_BUILD_JOBS='2')
results = []


def execute(name, command, expected, test=None):
    path = OUT / (name + '.log')
    with path.open('w') as stream:
        result = subprocess.run(command, cwd=WORK, env=env, stdout=stream, stderr=subprocess.STDOUT)
    log = path.read_text()
    record = dict(case=name, command=command, exit=result.returncode, expected=expected)
    if expected:
        record['assertion_detected'] = 'panicked at' in log and 'test result: FAILED' in log and f'test {test} ... FAILED' in log
    results.append(record)
    (OUT / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
    assert result.returncode == expected and (not expected or record['assertion_detected']), record
    print(name + ': verified', flush=True)


try:
    command = base.copy()
    for target in sorted({m[4] for m in mutations}):
        command.extend(['--test', target])
    execute('control', command, 0)
    for name, module, before, after, target, test in mutations:
        for path, source in originals.items():
            (CORE / 'src' / path).write_text(source)
        (CORE / 'src' / module).write_text(originals[module].replace(before, after))
        execute(name, base + ['--test', target, test, '--', '--exact'], 101, test)
finally:
    for path, source in originals.items():
        (CORE / 'src' / path).write_text(source)
    assert hashes() == initial, 'reviewed candidate changed during mutation run'
    (OUT / 'candidate-unchanged.json').write_text(json.dumps({'unchanged': True}) + '\n')
