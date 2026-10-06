"""Reproduce input mutations against an immutable copy of the #666 assertions.

Run from repository root. Only a small external consumer is generated under
target; no product sources, original tests, or canonical fixtures are mutated.
"""
from pathlib import Path
import hashlib
import json
import os
import subprocess

root = Path.cwd()
evidence = root / "docs/reports/2026-10-03-issue-666-evidence"
probe = root / "target/issue666-type1-mutations"
probe.mkdir(parents=True, exist_ok=True)
original = root / "oxidize-pdf-core/tests/text_type1_intrinsic_contract_test.rs"
source = original.read_text()
original_hash = hashlib.sha256(original.read_bytes()).hexdigest()
tests = root / "oxidize-pdf-core/tests"
source = source.replace('"common/text_contracts.rs"', json.dumps(str(tests / "common/text_contracts.rs")))
source = source.replace('"fixtures/text_contracts/type1/', '"' + str(tests / "fixtures/text_contracts/type1") + '/')
source = source.replace('env!("CARGO_MANIFEST_DIR")', json.dumps(str(root / "oxidize-pdf-core")))
source = source.replace('select(case).to_vec()', 'mutated(select(case))')
source = source.replace('Cursor::new(select(case))', 'Cursor::new(mutated(select(case)))')
source += r'''
fn mutated(input: &[u8]) -> Vec<u8> {
    let (before, after): (&[u8], &[u8]) = match std::env::var("ISSUE666_INPUT_MUTATION").as_deref() {
        Ok("shown-code") => (b"(A) Tj", b"(B) Tj"),
        Ok("unicode") => (b"<0058>", b"<005A>"),
        Ok("advance") => (b"/Widths [400 700]", b"/Widths [900 700]"),
        _ => return input.to_vec(),
    };
    assert_eq!(before.len(), after.len(), "mutation must preserve xref/Length");
    let at = input.windows(before.len()).position(|part| part == before).expect("mutation reaches fixture");
    let mut bytes = input.to_vec();
    bytes[at..at + after.len()].copy_from_slice(after);
    bytes
}
'''
(probe / "contracts.rs").write_text(source)
(probe / "Cargo.toml").write_text(f'''[package]
name = "issue666-type1-mutations"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
oxidize-pdf = {{ path = {json.dumps(str(root / 'oxidize-pdf-core'))} }}
sha2 = "0.10"
serde_json = "1"
[[test]]
name = "contracts"
path = "contracts.rs"
''')
# Exact dependency versions initially copied; consumer-only lock reconciliation
# happens under target, leaving the product lock unchanged.
(probe / "Cargo.lock").write_bytes((root / "Cargo.lock").read_bytes())
rows = []
for mutation, test, marker in [
    ("shown-code", "explicit_pdf_encoding_overrides_the_embedded_encoding", 'expected "AB", got "BB"'),
    ("unicode", "tounicode_overrides_pdf_differences_and_intrinsic_encoding", 'expected "XY", got "ZY"'),
    ("advance", "explicit_encoding_glyph_width_advances_the_text_pen", "expected (104,700), actual (109,700)"),
]:
    for mutate in [False, True]:
        env = os.environ.copy()
        env["ISSUE666_INPUT_MUTATION"] = mutation if mutate else "control"
        command = ["cargo", "test", "--offline", "--manifest-path", str(probe / "Cargo.toml"),
                   "--target-dir", str(root / "target"), "--test", "contracts", test, "--", "--exact"]
        result = subprocess.run(command, env=env, capture_output=True, text=True)
        log = result.stdout + result.stderr
        filename = f"mutation-{mutation}-{'red' if mutate else 'control'}.log"
        (evidence / filename).write_text(log)
        assert result.returncode == (101 if mutate else 0), log
        if mutate:
            assert marker in log, log
        rows.append({"mutation": mutation, "mutated": mutate, "test": test,
                     "exit_code": result.returncode, "assertion_verified": marker if mutate else None,
                     "log": filename})
assert hashlib.sha256(original.read_bytes()).hexdigest() == original_hash
(evidence / "type1-mutations.json").write_text(json.dumps({
    "scope": "Input mutation discrimination, not product mutation coverage",
    "original_test_sha256": original_hash, "original_unchanged": True, "runs": rows,
}, indent=2) + "\n")
print("Three input mutations detected by intended assertions; three controls pass; original unchanged.")
