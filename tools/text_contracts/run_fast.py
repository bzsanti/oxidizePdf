#!/usr/bin/env python3
"""Run #666 deterministic contracts with the same commands locally and in CI.

Python 3.11+, no external readers or host fonts. Heavy corpus tests are separate.
The minimal profile builds the library without dev-dependency feature unification
before running tests, so a successful test build cannot hide missing dependencies.
"""
import argparse
import json
import os
import re
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
PROFILES = {
    "default": [],
    "minimal": ["--no-default-features", "--features", "compression"],
    "spi": ["--features", "unstable-spi,semantic"],
}


def contract_targets():
    directory = ROOT / "oxidize-pdf-core/tests"
    inventory = json.loads((directory / "fixtures/text_contracts/coverage.json").read_text(encoding="utf-8"))
    declared = {name for row in inventory["rows"] for name in row["tests"]}
    discovered = {path.stem for path in directory.glob("text_*_contract_test.rs")}
    targets = sorted(declared | discovered)
    if not targets or any(not (directory / f"{name}.rs").is_file() for name in targets):
        raise ValueError("empty contract suite or missing declared test target")
    return targets


def verify_contract_log(text, target_count):
    results = re.findall(r"^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", text, re.MULTILINE)
    if len(results) != target_count or any(int(passed) == 0 or int(failed) or int(ignored) for passed, failed, ignored in results):
        raise ValueError("contracts require every selected target to execute nonempty tests without failures or ignores")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=PROFILES, default="default")
    parser.add_argument("--toolchain", help="installed rustup toolchain, e.g. 1.88")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    scratch = args.output.resolve() / "scratch"
    scratch.mkdir(exist_ok=True)
    environment = dict(os.environ, PYTHONUTF8="1", TMPDIR=str(scratch), TEMP=str(scratch), TMP=str(scratch))
    targets = contract_targets()
    cargo = ["cargo"] + ([f"+{args.toolchain}"] if args.toolchain else [])
    common = ["--locked", "-p", "oxidize-pdf"] + PROFILES[args.profile]
    if args.offline:
        common += ["--offline"]
    tests = targets + (["analysis_spi_test"] if args.profile == "spi" else [])
    test_args = [arg for name in tests for arg in ["--test", name]]
    commands = [
        ("traceability", [sys.executable, "tools/text_contracts/validate_coverage.py"]),
        ("tool-tests", [sys.executable, "-m", "unittest", "discover", "-s", "tools/tests", "-v"]),
        ("library", cargo + ["build", "--lib"] + common),
        ("contracts", cargo + ["test", "--no-fail-fast"] + common + test_args),
    ]
    summary = dict(profile=args.profile, toolchain=args.toolchain, targets=tests, completed=False, checks=[])
    # Exclusive creation prevents an interrupted retry from masquerading as the
    # prior run. Each invocation needs a fresh evidence directory.
    with (args.output / "result.json").open("x", encoding="utf-8") as report:
        try:
            for name, command in commands:
                print(f"{args.profile}: {name}", flush=True)
                with (args.output / f"{name}.log").open("x", encoding="utf-8") as log:
                    result = subprocess.run(command, cwd=ROOT, env=environment, stdout=log, stderr=subprocess.STDOUT)
                summary["checks"].append(dict(name=name, command=command, exit=result.returncode))
                if result.returncode:
                    raise SystemExit(result.returncode)
                if name == "contracts":
                    verify_contract_log((args.output / f"{name}.log").read_text(encoding="utf-8"), len(tests))
            summary["completed"] = True
        finally:
            json.dump(summary, report, indent=2)
            report.write("\n")


if __name__ == "__main__":
    main()
