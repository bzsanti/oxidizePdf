"""Run isolated parser mutations against public operation/pixel assertions."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

root = Path.cwd()
evidence = root / "docs/reports/2026-10-05-t4-content-evidence"
artifact_path = evidence / "stream-final-artifacts.jsonl"
artifacts = [json.loads(line) for line in artifact_path.read_text().splitlines()] if artifact_path.exists() else json.loads((evidence / "length-artifacts.json").read_text())
if len(sys.argv) > 1:
    evidence = Path(sys.argv[1])
    evidence.mkdir(exist_ok=True)
work = root / "target/t4-length-parser-mutations"
work.mkdir(exist_ok=True)
source = root / "oxidize-pdf-core/src/parser/content.rs"
original = source.read_text()
def library(name):
    return next(f for a in artifacts if a.get("reason") == "compiler-artifact" and a["target"]["name"] == name for f in a["filenames"] if f.endswith(".rlib"))
(work / "wrapper.rs").write_text("pub use oxidize_pdf::objects;\npub use oxidize_pdf::parser::{ParseError, ParseResult};\nmod content;\nmod regression;\n")
tests = (root / "oxidize-pdf-core/tests/text_inline_image_boundary_contract_test.rs").read_text()
tests = tests.replace("mod common;", f'#[path = "{root}/oxidize-pdf-core/tests/common/mod.rs"] mod common;').replace("use oxidize_pdf::parser::content::", "use crate::content::")
(work / "regression.rs").write_text(tests)
cases = [
    ("control", original, "regression::", 0),
    ("nul-not-whitespace", original.replace("b'\\0' | ", ""), "regression::nul_separates", 101),
    ("ignore-raw-length", original.replace(".and_then(|len| start.checked_add(len))", ".and_then(|_len| None::<usize>)"), "regression::unfiltered_length_keeps", 101),
    ("omit-row-padding", original.replace(".checked_add(7)?", ".checked_add(0)?"), "regression::unfiltered_length_uses", 101),
    ("invalid-mask-default", original.replace('_ => return None,\n    };\n    let components', '_ => false,\n    };\n    let components'), "content::tests::inline_length_distinguishes", 101),
    ("retain-virtual-whitespace", original.replace('data.truncate(length);', 'let _ = length;'), "content::tests::incremental_operations_match_batch", 101),
]
results = []
for name, variant, test_filter, expected in cases:
    assert name == "control" or variant != original, name
    (work / "content.rs").write_text(variant)
    binary = work / name
    command = ["rustc", "--edition=2021", "--test", "-A", "dead_code", str(work / "wrapper.rs"), "-L", f"dependency={root}/target/debug/deps", "--extern", "oxidize_pdf=" + library("oxidize_pdf"), "--extern", "tracing=" + library("tracing"), "-o", str(binary)]
    build = subprocess.run(command, capture_output=True, text=True)
    (evidence / f"length-mutation-{name}-build.log").write_text(build.stdout + build.stderr)
    assert build.returncode == 0, name
    result = subprocess.run([str(binary), test_filter, "--nocapture"], capture_output=True, text=True)
    output = result.stdout + result.stderr
    (evidence / f"length-mutation-{name}.log").write_text(output)
    assert result.returncode == expected, (name, output)
    assert expected == 0 or "assertion" in output or (
        name == "nul-not-whitespace" and "Invalid character in hex string" in output
    ) or (
        name == "ignore-raw-length" and "Unexpected '>'" in output
    ), (name, output)
    results.append({"case": name, "exit": result.returncode, "filter": test_filter, "source_sha256": hashlib.sha256(variant.encode()).hexdigest()})
    (evidence / "length-parser-mutations.json").write_text(json.dumps(results, indent=2))
    print(name, result.returncode, flush=True)
assert source.read_text() == original
