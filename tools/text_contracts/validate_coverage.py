#!/usr/bin/env python3
"""Validate #666 scope traceability; success is not a product conformance claim."""
import argparse
import json
from pathlib import Path
import re


STATUSES = {"planned", "oracle-ready", "executable", "executable-partial",
            "validated", "failing", "unsupported", "policy-pending"}


def validate(manifest, root):
    root = root.resolve()
    errors = []
    if manifest.get("schema_version") != 1 or manifest.get("issue") != 666:
        return ["unsupported schema or issue"]
    plan = (root / manifest["plan"]).resolve()
    if not plan.is_relative_to(root):
        return ["plan escapes repository"]
    expected = re.findall(r"^\| ([FECUDS]\d{2}) \|", plan.read_text(), re.MULTILINE)
    if not expected or len(set(expected)) != len(expected):
        return ["plan has empty or duplicate scope IDs"]
    rows = manifest.get("rows", [])
    ids = [row.get("id") for row in rows]
    if len(set(ids)) != len(ids):
        errors.append("duplicate coverage ID")
    if set(ids) != set(expected):
        errors.append(f"coverage differs from plan: missing={sorted(set(expected) - set(ids))}, extra={sorted(set(ids) - set(expected), key=str)}")
    for row in rows:
        name = row.get("id", "<missing>")
        if row.get("status") not in STATUSES:
            errors.append(f"{name}: unknown status")
        for key in ["scope", "contract", "fixture_strategy", "closure_criterion"]:
            if not isinstance(row.get(key), str) or not row[key].strip():
                errors.append(f"{name}: missing {key}")
        sources = row.get("oracle_sources")
        if not isinstance(sources, list) or not sources or any(not isinstance(s, str) or not s.strip() for s in sources):
            errors.append(f"{name}: missing independent oracle sources")
        if row.get("status") != "validated" and not row.get("remaining"):
            errors.append(f"{name}: incomplete row must state remaining work")
        tests = row.get("tests", [])
        if row.get("status") in {"executable", "executable-partial", "validated", "failing"} and not tests:
            errors.append(f"{name}: executable row has no test target")
        for test in tests:
            if not isinstance(test, str) or not re.fullmatch(r"[a-z0-9_]+", test):
                errors.append(f"{name}: invalid test target")
            elif not (root / "oxidize-pdf-core/tests" / f"{test}.rs").is_file():
                errors.append(f"{name}: missing test target {test}")
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--manifest", type=Path,
                        default=Path("oxidize-pdf-core/tests/fixtures/text_contracts/coverage.json"))
    args = parser.parse_args()
    try:
        manifest = json.loads((args.root / args.manifest).read_text())
        errors = validate(manifest, args.root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        errors = [str(error)]
    print(json.dumps({"traceability_valid": not errors, "errors": errors,
                      "scope": "Inventory only; no product-support or integration certification"}, indent=2))
    return int(bool(errors))


if __name__ == "__main__":
    raise SystemExit(main())
